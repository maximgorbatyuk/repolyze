/// Compute `(col_width, gap, display_labels)` that fits `bars.len()` columns into `max_width` chars.
///
/// `max_width` is the total chart-area width in characters. Callers that render a leading
/// padding column (e.g. the TUI renders `" "` at the start of each row) must subtract that
/// padding from `max_width` themselves; this function is padding-agnostic.
///
/// Strategy:
/// 1. Try original labels with `gap = 1`.
/// 2. Strip `":00"` suffix from hour labels (e.g. `"14:00"` → `"14"`) and retry.
/// 3. Shrink `col_width` to whatever fits; labels are truncated, values abbreviated via
///    [`format_value_fit`].
///
/// Column width is always at least `max(label_width, value_digit_width, 3)` when the chart fits,
/// so value rows never overflow the label row.
///
/// # Assumptions
/// Labels are ASCII (weekday names and `"HH:00"` hour strings). `.len()` returns byte count which
/// equals char count for ASCII; `chars().take()` truncation is also correct. Non-ASCII labels
/// (CJK, emoji) would be miscounted.
pub fn fit_bar_dimensions(bars: &[(String, u64)], max_width: usize) -> (usize, usize, Vec<String>) {
    let n = bars.len();
    let mut labels: Vec<String> = bars.iter().map(|(l, _)| l.clone()).collect();
    let value_width = bars
        .iter()
        .map(|(_, v)| v.to_string().len())
        .max()
        .unwrap_or(1);

    let fits = |labels: &[String], gap: usize| -> Option<usize> {
        let lw = labels.iter().map(|l| l.len()).max().unwrap_or(1);
        let desired = lw.max(value_width).max(3);
        let gaps = n.saturating_sub(1) * gap;
        if n * desired + gaps <= max_width {
            Some(desired)
        } else {
            None
        }
    };

    let mut gap = 1usize;
    if let Some(cw) = fits(&labels, gap) {
        return (cw, gap, labels);
    }

    // Try stripping ":00" suffix (hour labels like "14:00" → "14")
    if labels.iter().all(|l| l.ends_with(":00") && l.len() > 3) {
        labels = labels
            .iter()
            .map(|l| l[..l.len() - 3].to_string())
            .collect();
        if let Some(cw) = fits(&labels, gap) {
            return (cw, gap, labels);
        }
    }

    // Shrink col_width (labels and values will be truncated/abbreviated)
    let gaps = n.saturating_sub(1) * gap;
    let mut col_width = max_width.saturating_sub(gaps) / n.max(1);
    if col_width == 0 {
        gap = 0;
        col_width = (max_width / n.max(1)).max(1);
    }

    let display: Vec<String> = labels
        .iter()
        .map(|l| l.chars().take(col_width).collect::<String>())
        .collect();
    (col_width, gap, display)
}

/// Format a bar value so it fits within `col_width` chars.
///
/// Uses SI suffixes (`k`, `M`, `G`) when the raw number would overflow the column width.
/// Falls back to a `head+` truncation if even the abbreviated form is too wide.
pub fn format_value_fit(value: u64, col_width: usize) -> String {
    let raw = value.to_string();
    if raw.len() <= col_width || col_width == 0 {
        return raw;
    }
    let (scaled, suffix) = if value >= 1_000_000_000 {
        (value / 1_000_000_000, 'G')
    } else if value >= 1_000_000 {
        (value / 1_000_000, 'M')
    } else if value >= 1_000 {
        (value / 1_000, 'k')
    } else {
        return raw.chars().take(col_width).collect();
    };
    let candidate = format!("{scaled}{suffix}");
    if candidate.len() <= col_width {
        candidate
    } else if col_width >= 2 {
        let head: String = candidate.chars().take(col_width - 1).collect();
        format!("{head}+")
    } else {
        "+".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_value_fit_small_values_pass_through() {
        assert_eq!(format_value_fit(0, 3), "0");
        assert_eq!(format_value_fit(42, 3), "42");
        assert_eq!(format_value_fit(999, 3), "999");
    }

    #[test]
    fn format_value_fit_si_abbreviation() {
        assert_eq!(format_value_fit(1_000, 2), "1k");
        assert_eq!(format_value_fit(12_000, 3), "12k");
        assert_eq!(format_value_fit(1_000_000, 2), "1M");
        assert_eq!(format_value_fit(1_000_000_000, 2), "1G");
    }

    #[test]
    fn format_value_fit_truncates_with_plus() {
        // "12k" is 3 chars, col_width=2 → "1+"
        assert_eq!(format_value_fit(12_000, 2), "1+");
    }

    #[test]
    fn format_value_fit_col_width_zero_returns_raw() {
        assert_eq!(format_value_fit(12_345, 0), "12345");
    }

    #[test]
    fn fit_bar_dimensions_weekday_fits_at_default() {
        let bars: Vec<(String, u64)> = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
            .iter()
            .map(|l| (l.to_string(), 10u64))
            .collect();
        let (cw, gap, labels) = fit_bar_dimensions(&bars, 80);
        // max(label_width=3, value_width=2, 3) = 3
        assert_eq!(cw, 3);
        assert_eq!(gap, 1);
        assert_eq!(labels.len(), 7);
        assert_eq!(labels[0], "Mon");
    }

    #[test]
    fn fit_bar_dimensions_hour_strips_suffix() {
        let bars: Vec<(String, u64)> = (0..24).map(|h| (format!("{h:02}:00"), 5u64)).collect();
        let (_, _, labels) = fit_bar_dimensions(&bars, 80);
        // With 24 bars × 5-char labels + gaps = 143 > 80; stripped to bare "HH".
        assert_eq!(labels[0], "00");
        assert_eq!(labels[23], "23");
    }

    /// Caller reserving a leading pad column passes (max_width - 1). The returned layout
    /// plus gaps must not exceed that reduced budget.
    #[test]
    fn fit_bar_dimensions_respects_caller_padded_width() {
        let bars: Vec<(String, u64)> = (0..24).map(|h| (format!("{h:02}:00"), 99u64)).collect();
        let tui_width: usize = 71;
        // Caller reserves 1 char for the row's leading space.
        let (cw, gap, _) = fit_bar_dimensions(&bars, tui_width - 1);
        let rendered = 1 + 24 * cw + 23 * gap;
        assert!(
            rendered <= tui_width,
            "rendered width {rendered} exceeded terminal {tui_width} (cw={cw}, gap={gap})"
        );
    }

    #[test]
    fn fit_bar_dimensions_includes_value_width() {
        // 7 bars, all values = 999 (3 digits). col_width must be >= 3.
        let bars: Vec<(String, u64)> = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
            .iter()
            .map(|l| (l.to_string(), 999u64))
            .collect();
        let (cw, _, _) = fit_bar_dimensions(&bars, 80);
        assert!(cw >= 3, "col_width {cw} should accommodate 3-digit values");
    }
}
