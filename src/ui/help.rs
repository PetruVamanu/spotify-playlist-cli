use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;

pub fn browser_help_text() -> Line<'static> {
    Line::from(vec![
        Span::styled(" /", Style::default().fg(Color::Yellow)),
        Span::raw(" Search "),
        Span::styled("n", Style::default().fg(Color::Yellow)),
        Span::raw(" New "),
        Span::styled("d", Style::default().fg(Color::Yellow)),
        Span::raw(" Delete "),
        Span::styled("Tab", Style::default().fg(Color::Yellow)),
        Span::raw(" Switch "),
        Span::styled("r", Style::default().fg(Color::Yellow)),
        Span::raw(" Refresh "),
        Span::styled("q", Style::default().fg(Color::Yellow)),
        Span::raw(" Quit"),
    ])
}

pub fn search_help_text() -> Line<'static> {
    Line::from(vec![
        Span::styled(" Enter", Style::default().fg(Color::Yellow)),
        Span::raw(" Toggle "),
        Span::styled("↑↓", Style::default().fg(Color::Yellow)),
        Span::raw(" Navigate "),
        Span::styled("Esc", Style::default().fg(Color::Yellow)),
        Span::raw(" Close"),
    ])
}

pub fn draw_browser_help(frame: &mut Frame, _app: &mut App) {
    let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(frame.area());

    let paragraph = Paragraph::new(browser_help_text());
    frame.render_widget(paragraph, chunks[1]);
}

pub fn draw_search_help(frame: &mut Frame, _app: &mut App) {
    let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(frame.area());

    let paragraph = Paragraph::new(search_help_text());
    frame.render_widget(paragraph, chunks[1]);
}
