use anyhow::Result;
use crossterm::{
    cursor::Show,
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::stdout;

mod app;
mod auth;
mod config;
mod spotify;
mod ui;

use app::App;

fn restore_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(stdout(), LeaveAlternateScreen, Show);
}

#[tokio::main]
async fn main() -> Result<()> {
    // Install custom panic hook to restore terminal before printing panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        restore_terminal();
        original_hook(panic_info);
    }));

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create and run application
    let mut app = App::new().await?;
    let res = app.run(&mut terminal).await;

    // Restore terminal on exit or error
    restore_terminal();

    if let Err(err) = res {
        eprintln!("Error: {err:#}");
    }

    Ok(())
}
