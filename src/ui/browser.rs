use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::{App, Pane};

pub fn draw(frame: &mut Frame, app: &mut App) {
    let outer_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(frame.area());

    let main_area = outer_chunks[0];
    let status_area = outer_chunks[1];

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(main_area);

    let playlist_area = main_chunks[0];
    let track_area = main_chunks[1];

    let (playlist_border_style, track_border_style) = match app.active_pane {
        Pane::Playlists => (
            Style::default().fg(Color::Cyan),
            Style::default().fg(Color::DarkGray),
        ),
        Pane::Tracks => (
            Style::default().fg(Color::DarkGray),
            Style::default().fg(Color::Cyan),
        ),
    };

    // Render Playlists pane
    let playlist_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Playlists ")
        .border_style(playlist_border_style);

    let pl_inner_width = playlist_area.width.saturating_sub(4) as usize;
    let playlist_items: Vec<ListItem> = if app.playlists.is_empty() {
        if app.loading {
            vec![ListItem::new(Line::from(Span::styled(
                "  Loading playlists...",
                Style::default().fg(Color::Yellow),
            )))]
        } else {
            vec![ListItem::new(Line::from(Span::styled(
                "  (No playlists)",
                Style::default().fg(Color::DarkGray),
            )))]
        }
    } else {
        app.playlists
            .iter()
            .map(|playlist| {
                let count_str = format!("({})", playlist.items.total);
                let count_len = count_str.len();
                let max_name_len = pl_inner_width.saturating_sub(count_len + 1);

                let name = if playlist.name.chars().count() > max_name_len {
                    let mut s: String = playlist
                        .name
                        .chars()
                        .take(max_name_len.saturating_sub(1))
                        .collect();
                    s.push('…');
                    s
                } else {
                    playlist.name.clone()
                };

                let padding = " ".repeat(max_name_len.saturating_sub(name.chars().count()).max(1));

                let line = Line::from(vec![
                    Span::raw(name),
                    Span::raw(padding),
                    Span::styled(count_str, Style::default().fg(Color::DarkGray)),
                ]);

                ListItem::new(line)
            })
            .collect()
    };

    let playlist_list = List::new(playlist_items)
        .block(playlist_block)
        .highlight_symbol("> ")
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_stateful_widget(playlist_list, playlist_area, &mut app.playlist_state);

    // Render Tracks pane
    let playlist_title = if let Some(selected_idx) = app.playlist_state.selected() {
        if let Some(playlist) = app.playlists.get(selected_idx) {
            format!(" {} ({} tracks) ", playlist.name, playlist.items.total)
        } else {
            " Tracks ".to_string()
        }
    } else {
        " Tracks ".to_string()
    };

    let track_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(playlist_title)
        .border_style(track_border_style);

    let track_inner = track_block.inner(track_area);
    frame.render_widget(track_block, track_area);

    let track_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(track_inner);

    let header_area = track_chunks[0];
    let list_area = track_chunks[1];

    let available_width = header_area.width.saturating_sub(4) as usize;
    let fixed_width = 4 + 7 + 4; // num (4), duration (7), spacings (4)
    let remaining_width = available_width.saturating_sub(fixed_width);
    let title_width = (remaining_width * 6 / 10).max(10);
    let artist_width = remaining_width.saturating_sub(title_width).max(8);

    // Track column header
    let header_line = Line::from(vec![
        Span::styled("   #  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{:<width$}  ", "Title", width = title_width),
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{:<width$}  ", "Artist", width = artist_width),
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "Duration",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    frame.render_widget(Paragraph::new(header_line), header_area);

    let track_items: Vec<ListItem> = if app.tracks.is_empty() {
        if app.loading {
            vec![ListItem::new(Line::from(Span::styled(
                "  (Loading tracks...)",
                Style::default().fg(Color::Yellow),
            )))]
        } else {
            vec![ListItem::new(Line::from(Span::styled(
                "  (No tracks in playlist)",
                Style::default().fg(Color::DarkGray),
            )))]
        }
    } else {
        app.tracks
            .iter()
            .enumerate()
            .map(|(idx, track)| {
                let num_str = format!("{:>3}", idx + 1);

                let title = truncate_str(&track.name, title_width);
                let artists = track
                    .artists
                    .iter()
                    .map(|a| a.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                let artist = truncate_str(&artists, artist_width);

                let secs = track.duration.num_seconds().max(0);
                let duration_str = format!("{}:{:02}", secs / 60, secs % 60);

                let line = Line::from(vec![
                    Span::styled(format!("{num_str}  "), Style::default().fg(Color::DarkGray)),
                    Span::raw(format!("{:<width$}  ", title, width = title_width)),
                    Span::styled(
                        format!("{:<width$}  ", artist, width = artist_width),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::raw(format!("{:>6}", duration_str)),
                ]);

                ListItem::new(line)
            })
            .collect()
    };

    let track_list = List::new(track_items)
        .highlight_symbol("> ")
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_stateful_widget(track_list, list_area, &mut app.track_state);

    // Render Status bar
    let status_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray));

    let status_line = if let Some(ref input) = app.new_playlist_input {
        Line::from(vec![
            Span::styled(
                "New playlist: ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(input.clone(), Style::default().fg(Color::White)),
            Span::styled("█", Style::default().fg(Color::Cyan)),
        ])
    } else {
        let mut spans = vec![
            Span::styled("Logged in as ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                if app.user_name.is_empty() {
                    "Spotify User"
                } else {
                    &app.user_name
                },
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ];

        if !app.status_msg.is_empty() {
            spans.push(Span::styled(" | ", Style::default().fg(Color::DarkGray)));
            spans.push(Span::styled(
                app.status_msg.clone(),
                Style::default().fg(Color::LightCyan),
            ));
        }

        if app.loading {
            spans.push(Span::styled(" ⠋", Style::default().fg(Color::Yellow)));
        }

        Line::from(spans)
    };

    let status_paragraph = Paragraph::new(status_line).block(status_block);
    frame.render_widget(status_paragraph, status_area);
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
