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

/// Unicode block characters for sparkline bars, ordered from shortest (`▁`) to tallest (`█`).
pub const SPARKLINE_BLOCKS: [&str; 8] = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];

/// Height (rows) of the block-character sparkline charts used in plain-text and Markdown output.
const SPARKLINE_CHART_HEIGHT: usize = 8;

/// Render a bar chart of 8 block-character rows plus a value row and label row.
///
/// Layout (from top to bottom):
/// 1. 8 sparkline rows, each using `SPARKLINE_BLOCKS` for fractional-height bars
/// 2. A value row (centered per-column, abbreviated via [`format_value_fit`])
/// 3. A label row (centered per-column, possibly truncated by [`fit_bar_dimensions`])
///
/// If every value is 0, returns `{labels joined by space}\n{empty_message}\n` so the caller still
/// sees something useful. `max_width` is the total chart-area width (same semantics as
/// [`fit_bar_dimensions`]).
pub fn render_sparkline_bars(
    bars: &[(String, u64)],
    max_width: usize,
    empty_message: &str,
) -> String {
    let max_val = bars.iter().map(|(_, v)| *v).max().unwrap_or(0);
    if max_val == 0 {
        let labels: String = bars
            .iter()
            .map(|(l, _)| l.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        return format!("{labels}\n{empty_message}\n");
    }

    let (col_width, gap, display_labels) = fit_bar_dimensions(bars, max_width);
    let mut out = String::new();

    for row in (0..SPARKLINE_CHART_HEIGHT).rev() {
        let threshold = (row as f64 + 0.5) / SPARKLINE_CHART_HEIGHT as f64;
        for (i, (_, value)) in bars.iter().enumerate() {
            if i > 0 && gap > 0 {
                out.push_str(&" ".repeat(gap));
            }
            let ratio = *value as f64 / max_val as f64;
            if ratio >= threshold {
                let block = if ratio >= threshold + 1.0 / SPARKLINE_CHART_HEIGHT as f64 {
                    "█"
                } else {
                    let frac = ((ratio - threshold) * SPARKLINE_CHART_HEIGHT as f64 * 8.0).round()
                        as usize;
                    SPARKLINE_BLOCKS[frac.min(7)]
                };
                out.push_str(&block.repeat(col_width));
            } else {
                out.push_str(&" ".repeat(col_width));
            }
        }
        out.push('\n');
    }

    for (i, (_, value)) in bars.iter().enumerate() {
        if i > 0 && gap > 0 {
            out.push_str(&" ".repeat(gap));
        }
        let text = format_value_fit(*value, col_width);
        out.push_str(&format!("{:^col_width$}", text));
    }
    out.push('\n');

    for (i, label) in display_labels.iter().enumerate() {
        if i > 0 && gap > 0 {
            out.push_str(&" ".repeat(gap));
        }
        out.push_str(&format!("{:^col_width$}", label));
    }
    out.push('\n');

    out
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

    #[test]
    fn render_sparkline_bars_all_zero_returns_empty_message() {
        let bars = vec![("A".to_string(), 0u64), ("B".to_string(), 0u64)];
        let out = render_sparkline_bars(&bars, 80, "(no data)");
        assert!(out.contains("(no data)"));
        assert!(!out.contains('█'));
    }

    #[test]
    fn render_sparkline_bars_renders_tallest_block_for_max_value() {
        let bars = vec![("A".to_string(), 1u64), ("B".to_string(), 10u64)];
        let out = render_sparkline_bars(&bars, 80, "(no data)");
        assert!(out.contains('█'));
        // Value row contains both values
        assert!(out.contains("10"));
        // Label row contains labels
        assert!(out.contains('A'));
        assert!(out.contains('B'));
    }

    #[test]
    fn render_sparkline_bars_produces_ten_lines_when_values_nonzero() {
        // 8 sparkline rows + 1 value row + 1 label row = 10 newline-terminated lines.
        let bars = vec![("A".to_string(), 5u64), ("B".to_string(), 10u64)];
        let out = render_sparkline_bars(&bars, 80, "(no data)");
        assert_eq!(out.matches('\n').count(), 10);
    }
}
