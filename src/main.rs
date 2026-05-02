mod api;
mod app;
mod cache;
mod config;
mod ui;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, time::{Duration, Instant}};

use crate::app::App;

fn main() -> Result<()> {
    let cfg = config::load()?;
    let mut app = App::new(cfg)?;
    app.spawn_refresh();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut term = Terminal::new(backend)?;

    let result = run(&mut term, &mut app);

    disable_raw_mode()?;
    execute!(term.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    term.show_cursor()?;

    result
}

fn run<B: ratatui::backend::Backend>(term: &mut Terminal<B>, app: &mut App) -> Result<()> {
    let auto_tick = if app.cfg.refresh_interval > 0 {
        Some(Duration::from_secs(app.cfg.refresh_interval))
    } else {
        None
    };
    let mut last_auto = Instant::now();

    loop {
        term.draw(|f| ui::draw(f, app))?;

        // Drain the worker channel; any new snapshot triggers a redraw next iteration.
        let _ = app.drain();

        // Short poll keeps the UI responsive for both keypresses and worker results.
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(k) = event::read()? {
                if k.kind != KeyEventKind::Press {
                    continue;
                }
                handle_key(app, k.code, k.modifiers);
            }
        }

        if let Some(tick) = auto_tick {
            if last_auto.elapsed() >= tick {
                app.spawn_refresh();
                last_auto = Instant::now();
            }
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}

fn handle_key(app: &mut App, code: KeyCode, mods: KeyModifiers) {
    let k = app.cfg.keys.clone();
    match code {
        KeyCode::Char(c) => {
            if c == k.quit {
                app.should_quit = true;
            } else if c == k.next_tab {
                app.next_tab();
            } else if c == k.prev_tab {
                app.prev_tab();
            } else if c == k.down {
                app.move_down();
            } else if c == k.up {
                app.move_up();
            } else if c == k.top {
                app.jump_top();
            } else if c == k.bottom {
                app.jump_bottom();
            } else if c == k.refresh {
                app.spawn_refresh();
            } else if c == k.toggle_range {
                app.cycle_range();
            } else if c == 'c' && mods.contains(KeyModifiers::CONTROL) {
                app.should_quit = true;
            }
        }
        KeyCode::Enter => app.open_selected(),
        KeyCode::Esc => app.should_quit = true,
        KeyCode::Down => app.move_down(),
        KeyCode::Up => app.move_up(),
        KeyCode::Left => app.prev_tab(),
        KeyCode::Right => app.next_tab(),
        KeyCode::Home => app.jump_top(),
        KeyCode::End => app.jump_bottom(),
        _ => {}
    }
}
