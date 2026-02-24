mod app;
mod audio;
mod export;
mod keys;
mod pattern;
mod synth;
mod ui;

use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyEventKind};

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let mut app = app::App::new();

    while app.running {
        terminal.draw(|frame| ui::draw(frame, &app))?;

        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    app.handle_key(key);
                }
            }
        }

        app.tick();
    }

    ratatui::restore();
    Ok(())
}
