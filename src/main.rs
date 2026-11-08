mod app;
mod coverage;
mod ui;

use crate::app::App;
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    io,
    time::{Duration, Instant},
};
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;

#[tokio::main]
async fn main() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and load resources
    let mut app = App::new()?;
    let syntax_set = SyntaxSet::load_defaults_newlines();
    let theme_set = ThemeSet::load_defaults();

    let res = run_app(&mut terminal, &mut app, &syntax_set, &theme_set).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    syntax_set: &SyntaxSet,
    theme_set: &ThemeSet,
) -> io::Result<()> {
    let tick_rate = Duration::from_millis(250);
    let mut last_tick = Instant::now();
    loop {
        terminal.draw(|f| ui::render(f, app, syntax_set, theme_set))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or(Duration::from_secs(0));

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Tab => app.switch_focus(),

                        // Panel-specific handling
                        _ => match app.active_panel {
                            crate::app::ActivePanel::Filter => match key.code {
                                KeyCode::Enter | KeyCode::Down => {
                                    app.active_panel = crate::app::ActivePanel::Files
                                }
                                KeyCode::Backspace => {
                                    app.filter_query.pop();
                                    app.apply_filter();
                                }
                                KeyCode::Char(c) => {
                                    app.filter_query.push(c);
                                    app.apply_filter();
                                }
                                _ => {}
                            },
                            crate::app::ActivePanel::Files => match key.code {
                                KeyCode::Down | KeyCode::Char('j') => app.next_file(),
                                KeyCode::Up | KeyCode::Char('k') => app.prev_file(),
                                KeyCode::Enter | KeyCode::Right => {
                                    app.active_panel = crate::app::ActivePanel::Source
                                }
                                KeyCode::Char('/') => {
                                    app.active_panel = crate::app::ActivePanel::Filter
                                }
                                _ => {}
                            },
                            crate::app::ActivePanel::Source => match key.code {
                                KeyCode::Down | KeyCode::Char('j') => {
                                    app.scroll_offset = app.scroll_offset.saturating_add(1)
                                }
                                KeyCode::Up | KeyCode::Char('k') => {
                                    app.scroll_offset = app.scroll_offset.saturating_sub(1)
                                }
                                KeyCode::PageDown => {
                                    app.scroll_offset = app.scroll_offset.saturating_add(10)
                                }
                                KeyCode::PageUp => {
                                    app.scroll_offset = app.scroll_offset.saturating_sub(10)
                                }
                                KeyCode::Left | KeyCode::Backspace => {
                                    app.active_panel = crate::app::ActivePanel::Files
                                }
                                KeyCode::Char('/') => {
                                    app.active_panel = crate::app::ActivePanel::Filter
                                }
                                _ => {}
                            },
                        },
                    }
                }
            }
        }
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
}
