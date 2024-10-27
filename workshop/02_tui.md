# TUI

Now that we have our own project in the workspace, it is time to initialize the terminal user interface and make our application do something more than just `println!`.

It is time to switch to the `tui` directory (`cd tui/`) since we will be working there from now on.

Since our application will be reading events both from a TCP connection and the terminal, we will need to handle them concurrently. In Rust, we can achieve this by making our application asynchronous (or commonly known as "async"). There are various runtimes available for async Rust, but we will be using the [`tokio`](https://tokio.rs) runtime for this workshop. Although it might sound a bit scary at first, all you need to do is add the `tokio` dependency and use the `tokio::main` attribute to make your application async.

On the other hand, we will be using Ratatui along with [`crossterm`](https://github.com/crossterm-rs/crossterm) backend, which provides cross-platform terminal event handling.

So let's add our dependencies:

```sh
cargo add ratatui@0.28.1 # for the terminal UI
cargo add crossterm@0.28.1 --features event-stream # for async terminal events

cargo add tokio@1.40.0 --features full # async runtime
cargo add futures@0.3.31 # async event stream handling

cargo add anyhow # for easier error handling
```

There are various ways of structuring your Ratatui application (as noted in the [documentation](https://ratatui.rs/concepts)), but we will go with the most simple one for now.

Start by updating the `src/main.rs` as follows:

```rust
mod app;

use app::App;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let app = App::new();
    let terminal = ratatui::init();
    let result = app.run(terminal).await;
    ratatui::restore();
    result
}
```

Initializing the terminal (via `ratatui::init`) is necessary to ensure that the TUI uses an alternate screen buffer and let us control the events properly. Then we need to call `ratatui::restore` to back to the previous state respectively. `ratatui::init` also gives us a terminal type to further interact with the terminal in terms of drawing the UI and doing other things. See the [documentation](https://ratatui.rs/concepts/backends/alternate-screen/) for more information.

Then create a new file `src/app.rs` with the following content:

```rust
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
```

Here we are creating an `App` struct which is responsible for:

- Holding the state of the application (e.g. `is_running`).
- Creating the render loop (in the `run` method).
  - Drawing the UI via the `terminal.draw` method.
  - Handling the terminal events.

We will build upon the `App` struct in the next chapters so it is the central part of our application.

When you run the application (`cargo run`), you should see the Ratatui window with the text "Hello Ratatui!" in the center. You can exit the application by pressing the `Esc` key.
