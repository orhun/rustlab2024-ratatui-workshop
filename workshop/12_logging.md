# Logging

Let's initialize logging and add a logger pane to our TUI application to display the server events.

![logging](images/logging.gif)

## Implementing the Widget

In the Ratatui world, you can achieve this very easily with using the [`tui-logger`](https://github.com/gin66/tui-logger) crate. It provides required methods to initialize a logger and a smart widget to display logs.

It supports both `log` and `tracing` crates. We will be implementing with the [`tracing_subscriber`](https://docs.rs/tracing-subscriber) crate in this chapter.

Add the required dependencies:

```sh
cargo add tui-logger@0.13.2 --features tracing-support
cargo add tracing
cargo add tracing-appender
cargo add tracing-subscriber --features env-filter
cargo add log
```

Then we can implement the logger widget as follows (don't forget to add `mod logger;` to `src/main.rs`):

```rust
use ratatui::{
    style::{Color, Style},
    widgets::Widget,
};
use tokio::sync::mpsc::UnboundedSender;
use tui_logger::{TuiLoggerLevelOutput, TuiLoggerSmartWidget, TuiWidgetEvent, TuiWidgetState};
use tui_textarea::{Input, Key};

use crate::app::Event;

pub struct Logger {
    pub state: TuiWidgetState,
    pub event_sender: UnboundedSender<Event>,
}

impl Logger {
    pub fn new(event_sender: UnboundedSender<Event>) -> Self {
        Self {
            state: TuiWidgetState::default(),
            event_sender,
        }
    }

    pub async fn handle_input(&mut self, input: Input) -> anyhow::Result<()> {
        tracing::debug!("Logger input: {:?}", input);
        match (input.ctrl, input.key) {
            (true, Key::Char('l')) => {
                let _ = self.event_sender.send(Event::LoggerClosed);
            }
            (false, Key::Char(' ')) => self.state.transition(TuiWidgetEvent::SpaceKey),
            (false, Key::Esc) => self.state.transition(TuiWidgetEvent::EscapeKey),
            (false, Key::PageUp) => self.state.transition(TuiWidgetEvent::PrevPageKey),
            (false, Key::PageDown) => self.state.transition(TuiWidgetEvent::NextPageKey),
            (false, Key::Up) => self.state.transition(TuiWidgetEvent::UpKey),
            (false, Key::Down) => self.state.transition(TuiWidgetEvent::DownKey),
            (false, Key::Left) => self.state.transition(TuiWidgetEvent::LeftKey),
            (false, Key::Right) => self.state.transition(TuiWidgetEvent::RightKey),
            (false, Key::Char('+')) => self.state.transition(TuiWidgetEvent::PlusKey),
            (false, Key::Char('-')) => self.state.transition(TuiWidgetEvent::MinusKey),
            (false, Key::Char('h')) => self.state.transition(TuiWidgetEvent::HideKey),
            (false, Key::Char('f')) => self.state.transition(TuiWidgetEvent::FocusKey),
            _ => {}
        }
        Ok(())
    }
}

impl Widget for &Logger {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        TuiLoggerSmartWidget::default()
            .style_error(Style::default().fg(Color::Red))
            .style_debug(Style::default().fg(Color::Green))
            .style_warn(Style::default().fg(Color::Yellow))
            .style_trace(Style::default().fg(Color::Magenta))
            .style_info(Style::default().fg(Color::Cyan))
            .highlight_style(Style::default().fg(Color::Blue))
            .output_separator(':')
            .output_timestamp(Some("%H:%M:%S".to_string()))
            .output_level(Some(TuiLoggerLevelOutput::Abbreviated))
            .output_target(true)
            .output_file(true)
            .output_line(true)
            .state(&self.state)
            .render(area, buf);
    }
}
```

We are mapping the input to the logger event in `handle_input` and rendering the smart widget in the `Widget` implementation. You can customize the colors and output formats as you wish.

We are also sending `LoggerClosed` event when `Ctrl-l` is pressed. We will be implementing that in the next section.

## Setting up Tracing

Initialize the tracing subscriber in `src/main.rs`:

```diff
 mod app;
+mod logger;
 mod message_list;
 mod popup;
 mod room_list;
 mod ui;

-use app::App;
-
 use clap::Parser;
-use std::net::{IpAddr, Ipv4Addr, SocketAddr};
+use log::LevelFilter;
+use std::{
+    fs::File,
+    net::{IpAddr, Ipv4Addr, SocketAddr},
+};
+use tracing::Level;
+use tracing_appender::non_blocking::WorkerGuard;
+use tracing_subscriber::prelude::*;
+use tracing_subscriber::{fmt, EnvFilter};
+
+use app::App;

 pub const DEFAULT_IP: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);
 pub const DEFAULT_PORT: u16 = 42069;
@@ -37,9 +46,25 @@ impl Args {
     }
 }

+fn init_tracing() -> anyhow::Result<WorkerGuard> {
+    let file = File::create("tracing.log")?;
+    let (non_blocking, guard) = tracing_appender::non_blocking(file);
+    let env_filter = EnvFilter::builder()
+        .with_default_directive(Level::DEBUG.into())
+        .from_env_lossy();
+    tracing_subscriber::registry()
+        .with(tui_logger::tracing_subscriber_layer())
+        .with(fmt::layer().with_writer(non_blocking))
+        .with(env_filter)
+        .init();
+    tui_logger::init_logger(LevelFilter::Debug)?;
+    Ok(guard)
+}
+
 #[tokio::main]
 async fn main() -> anyhow::Result<()> {
     let addr = Args::parse_socket_addr();
+    let _guard = init_tracing()?;
     let app = App::new(addr);
     let terminal = ratatui::init();
     let result = app.run(terminal).await;

```

Two important things to note here:

1. We need to use the `tui_logger::tracing_subscriber_layer()` to integrate the logger widget with the tracing subscriber.
2. The worker guard is necessary to keep the logging thread alive.

## Updating the Application

Update `src/app.rs` as follows to show the logger pane when `Ctrl-l` is pressed:

```diff
use tokio::{
 use tokio_util::codec::{FramedRead, FramedWrite, LinesCodec};
 use tui_textarea::{Input, Key, TextArea};

-use crate::message_list::MessageList;
 use crate::popup::Popup;
 use crate::room_list::RoomList;
+use crate::{logger::Logger, message_list::MessageList};

 const KEY_BINDINGS: &str = r#"
 - [Ctrl + h] Help
@@ -23,6 +23,7 @@ const KEY_BINDINGS: &str = r#"
     - [Enter] Select file
     - [Right/Left] Navigate directories
 - [Ctrl + p] Preview file
+- [Ctrl + l] Show logger
 - [Esc] Quit
 "#;

@@ -44,6 +45,7 @@ pub struct App {
     pub message_list: MessageList,
     pub room_list: RoomList,
     pub text_area: TextArea<'static>,
+    pub logger: Option<Logger>,
     pub popup: Option<Popup>,
 }

@@ -52,6 +54,7 @@ pub enum Event {
     Terminal(CrosstermEvent),
     FileSelected(File),
     PopupClosed,
+    LoggerClosed,
     EffectRendered,
 }

@@ -75,6 +78,7 @@ impl App {
             message_list: MessageList::default(),
             room_list: RoomList::default(),
             text_area: create_text_area(),
+            logger: None,
             popup: None,
         }
     }
@@ -115,6 +119,10 @@ impl App {
                     popup.handle_input(input, raw_event).await?;
                     return Ok(());
                 }
+                if let Some(logger) = &mut self.logger {
+                    logger.handle_input(input).await?;
+                    return Ok(());
+                }
                 self.handle_key_input(input).await?;
             }
             Event::FileSelected(file) => {
@@ -127,6 +135,9 @@ impl App {
             Event::PopupClosed => {
                 self.popup = None;
             }
+            Event::LoggerClosed => {
+                self.logger = None;
+            }
             Event::EffectRendered => {}
         }
         Ok(())
@@ -143,6 +154,7 @@ impl App {
             (true, Key::Char('h')) => self.show_help(),
             (true, Key::Char('e')) => self.show_file_explorer()?,
             (true, Key::Char('p')) => self.preview_file()?,
+            (true, Key::Char('l')) => self.show_logger(),
             (_, _) => {
                 let _ = self.text_area.input_without_shortcuts(input);
             }
@@ -174,6 +186,10 @@ impl App {
         Ok(())
     }

+    fn show_logger(&mut self) {
+        self.logger = Some(Logger::new(self.event_sender.clone()));
+    }
+
     fn preview_file(&mut self) -> Result<(), anyhow::Error> {
         let selected_event = self.message_list.selected_event();
         let event_sender = self.event_sender.clone();
@@ -194,6 +210,7 @@ impl App {

     pub async fn handle_server_event(&mut self, event: String) -> anyhow::Result<()> {
         let event = ServerEvent::from_json_str(&event)?;
+        tracing::info!("Handling server event: {event:?}");
         self.message_list.events.push(event.clone());
         match event {
             ServerEvent::CommandHelp(username, _help) => self.message_list.username = username,
```

This also adds an info log when handling server events.

## Rendering

We want to show the logger pane at the bottom of the screen when it is active. To achieve that, we can use a new constraint and set its percentage to 0 when logger is `None`.

```diff
use crate::app::App;

 impl App {
     pub fn draw_ui(&mut self, frame: &mut Frame) {
-        let [message_area, text_area] =
-            Layout::vertical([Constraint::Fill(1), Constraint::Max(3)]).areas(frame.area());
+        let [message_area, text_area, logger_area] = Layout::vertical([
+            Constraint::Fill(1),
+            Constraint::Max(3),
+            Constraint::Percentage(40 * self.logger.is_some() as u16),
+        ])
+        .areas(frame.area());

         self.text_area.set_block(
             Block::bordered()
@@ -29,6 +33,9 @@ impl App {

         frame.render_widget(&mut self.message_list, message_area);
         frame.render_widget(&mut self.room_list, room_area);
+        if let Some(logger) = &self.logger {
+            frame.render_widget(logger, logger_area);
+        }

         if let Some(popup) = &mut self.popup {
             frame.render_widget(popup, frame.area());
```

The rest is simply calling `render_widget` with the logger widget if it is active.

Run your TUI and press `Ctrl-l` to see the logger pane at the bottom of the screen now! 🎉
