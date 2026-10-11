use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::App;
use crate::ui::centered_rect;

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let popup_area = centered_rect(80, 70, area);

    // Clear background behind popup
    frame.render_widget(Clear, popup_area);

    let popup_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Search Tracks (Esc to close) ")
        .border_style(Style::default().fg(Color::Cyan));

    let inner_area = popup_block.inner(popup_area);
    frame.render_widget(popup_block, popup_area);

    let search_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(inner_area);

    let input_area = search_chunks[0];
    let results_area = search_chunks[1];

    // Render search query input box
    let input_line = Line::from(vec![
        Span::styled(
            "> ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(&app.search_query, Style::default().fg(Color::White)),
        Span::styled("█", Style::default().fg(Color::Cyan)),
    ]);

    let input_block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(Color::DarkGray));

    let input_paragraph = Paragraph::new(input_line).block(input_block);
    frame.render_widget(input_paragraph, input_area);

    // Render search results list
    let available_width = results_area.width.saturating_sub(6) as usize;

    let result_items: Vec<ListItem> = if app.search_results.is_empty() {
        if app.loading {
            vec![ListItem::new(Line::from(Span::styled(
                "  Searching tracks... ⠋",
                Style::default().fg(Color::Yellow),
            )))]
        } else if app.search_query.trim().is_empty() {
            vec![ListItem::new(Line::from(Span::styled(
                "  Type to search for tracks...",
                Style::default().fg(Color::DarkGray),
            )))]
        } else {
            vec![ListItem::new(Line::from(Span::styled(
                "  No results found.",
                Style::default().fg(Color::DarkGray),
            )))]
        }
    } else {
        app.search_results
            .iter()
            .map(|track| {
                let id_str = track
                    .id
                    .as_ref()
                    .map(|id| id.to_string())
                    .unwrap_or_default();

                let is_in_playlist = !id_str.is_empty() && app.playlist_track_ids.contains(&id_str);

                let (check_str, check_style) = if is_in_playlist {
                    (
                        "✓",
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    (" ", Style::default())
                };

                let artists = track
                    .artists
                    .iter()
                    .map(|a| a.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");

                let secs = track.duration.num_seconds().max(0);
                let duration_str = format!("{}:{:02}", secs / 60, secs % 60);

                let track_info = format!("{} — {}", track.name, artists);
                let dur_width = 7;
                let max_info_width = available_width.saturating_sub(4 + dur_width);
                let truncated_info = truncate_str(&track_info, max_info_width);

                let padding = " ".repeat(
                    max_info_width
                        .saturating_sub(truncated_info.chars().count())
                        .max(1),
                );

                let line = Line::from(vec![
                    Span::raw("["),
                    Span::styled(check_str, check_style),
                    Span::raw("] "),
                    Span::raw(truncated_info),
                    Span::raw(padding),
                    Span::styled(duration_str, Style::default().fg(Color::DarkGray)),
                ]);

                ListItem::new(line)
            })
            .collect()
    };

    let results_list = List::new(result_items)
        .highlight_symbol("> ")
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_stateful_widget(results_list, results_area, &mut app.search_state);
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.chars().count() > max_len {
        let mut truncated: String = s.chars().take(max_len.saturating_sub(1)).collect();
        truncated.push('…');
        truncated
    } else {
        s.to_string()
    }
}
