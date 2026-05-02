mod api;
mod app;
mod config;
mod ui;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, time::Duration};

use crate::app::App;

fn main() -> Result<()> {
    let cfg = config::load()?;
    let mut app = App::new(cfg)?;
    app.refresh();

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
    let tick = if app.cfg.refresh_interval > 0 {
        Duration::from_secs(app.cfg.refresh_interval)
    } else {
        Duration::from_secs(60 * 60)
    };

    loop {
        term.draw(|f| ui::draw(f, app))?;

        if event::poll(tick)? {
            if let Event::Key(k) = event::read()? {
                if k.kind != KeyEventKind::Press {
                    continue;
                }
                handle_key(app, k.code, k.modifiers);
            }
        } else if app.cfg.refresh_interval > 0 {
            app.refresh();
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
                app.refresh();
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
