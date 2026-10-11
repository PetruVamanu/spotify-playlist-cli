use ratatui::layout::Alignment;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

use crate::app::App;
use crate::ui::centered_rect;

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let popup_area = centered_rect(60, 45, area);

    let mut text_lines = Vec::new();
    text_lines.push(Line::from(""));
    text_lines.push(Line::from(Span::styled(
        "♫ Spotify Playlist Editor",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));
    text_lines.push(Line::from(""));

    if app.loading {
        text_lines.push(Line::from(Span::styled(
            "Opening browser for login...",
            Style::default().fg(Color::White),
        )));
        text_lines.push(Line::from(""));
        text_lines.push(Line::from(Span::styled(
            "Waiting for authentication... ⠋",
            Style::default().fg(Color::Yellow),
        )));
    } else {
        text_lines.push(Line::from(Span::styled(
            "Authentication required",
            Style::default().fg(Color::White),
        )));
        text_lines.push(Line::from(""));
        text_lines.push(Line::from(Span::styled(
            "Please follow the browser login prompt",
            Style::default().fg(Color::Yellow),
        )));
    }

    if !app.status_msg.is_empty() {
        text_lines.push(Line::from(""));
        text_lines.push(Line::from(Span::styled(
            app.status_msg.clone(),
            Style::default().fg(Color::LightCyan),
        )));
    }

    text_lines.push(Line::from(""));
    text_lines.push(Line::from(Span::styled(
        "Press 'q' to quit",
        Style::default().fg(Color::DarkGray),
    )));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));

    let paragraph = Paragraph::new(text_lines)
        .block(block)
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, popup_area);
}
