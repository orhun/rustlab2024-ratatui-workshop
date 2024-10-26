use anyhow::Ok;
use crossterm::event::{Event, EventStream, KeyCode};
use futures::StreamExt;
use ratatui::DefaultTerminal;

pub struct App {
    term_stream: EventStream,
    is_running: bool,
}

impl App {
    pub fn new() -> Self {
        let term_stream = EventStream::new();
        Self {
            term_stream,
            is_running: false,
        }
    }

    pub async fn run(mut self, mut terminal: DefaultTerminal) -> anyhow::Result<()> {
        self.is_running = true;

        while self.is_running {
            terminal.draw(|frame| frame.render_widget("Hello Ratatui!", frame.area()))?;
            if let Some(crossterm_event) = self.term_stream.next().await {
                let crossterm_event = crossterm_event?;
                if let Event::Key(key_event) = crossterm_event {
                    if key_event.code == KeyCode::Esc {
                        self.is_running = false;
                    }
                }
            }
        }
        Ok(())
    }
}
