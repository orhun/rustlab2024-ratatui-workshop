use std::net::SocketAddr;

use anyhow::Ok;
use common::Command;
use crossterm::event::{Event, EventStream, KeyCode};
use futures::{SinkExt, StreamExt};
use ratatui::DefaultTerminal;
use tokio::net::{tcp::OwnedWriteHalf, TcpStream};
use tokio_util::codec::{FramedRead, FramedWrite, LinesCodec};

pub struct App {
    addr: SocketAddr,
    term_stream: EventStream,
    is_running: bool,
    tcp_writer: Option<FramedWrite<OwnedWriteHalf, LinesCodec>>,
}

impl App {
    pub fn new(addr: SocketAddr) -> Self {
        let term_stream = EventStream::new();
        Self {
            addr,
            term_stream,
            is_running: false,
            tcp_writer: None,
        }
    }

    pub async fn run(mut self, mut terminal: DefaultTerminal) -> anyhow::Result<()> {
        self.is_running = true;

        let connection = TcpStream::connect(self.addr).await?;
        let (reader, writer) = connection.into_split();
        self.tcp_writer = Some(FramedWrite::new(writer, LinesCodec::new()));
        let mut tcp_reader = FramedRead::new(reader, LinesCodec::new());

        while self.is_running {
            terminal.draw(|frame| frame.render_widget("Hello Ratatui!", frame.area()))?;
            tokio::select! {
                Some(crossterm_event) = self.term_stream.next() => {
                    let crossterm_event = crossterm_event?;
                    if let Event::Key(key_event) = crossterm_event {
                        if key_event.code == KeyCode::Esc {
                            let framed = self.tcp_writer.as_mut().unwrap();
                            let _ = framed.send(Command::Quit.to_string()).await;
                            self.is_running = false;
                        }
                    }
                },
                Some(_tcp_event) = tcp_reader.next() => {}
            }
        }
        Ok(())
    }
}
