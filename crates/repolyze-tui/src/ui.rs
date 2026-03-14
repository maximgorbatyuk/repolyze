use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use crate::app::AppState;

pub fn draw(frame: &mut Frame, app: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(20), Constraint::Percentage(80)])
        .split(frame.area());

    // Left menu
    let items: Vec<ListItem> = app
        .menu_items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let style = if i == app.selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(Span::styled(item.to_string(), style)))
        })
        .collect();

    let menu = List::new(items).block(Block::default().borders(Borders::ALL).title("Repolyze"));
    frame.render_widget(menu, chunks[0]);

    // Main content
    let content = Paragraph::new("Welcome to Repolyze\n\nPress ? for help, q to quit.")
        .block(Block::default().borders(Borders::ALL).title("Help"));
    frame.render_widget(content, chunks[1]);
}
