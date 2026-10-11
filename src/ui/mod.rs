pub mod browser;
pub mod help;
pub mod login;
pub mod search;

use crate::app::{App, Screen};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::Frame;

pub fn draw(frame: &mut Frame, app: &mut App) {
    match app.screen {
        Screen::Login => login::draw(frame, app),
        Screen::Browser => {
            browser::draw(frame, app);
            help::draw_browser_help(frame, app);
        }
        Screen::Search => {
            browser::draw(frame, app);
            search::draw(frame, app);
            help::draw_search_help(frame, app);
        }
    }
}

pub(crate) fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(area);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1])[1]
}
