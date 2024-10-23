mod app;
mod args;
mod message_list;
mod popup;
mod room_list;
mod ui;

use app::{App, Event};
use args::Args;
use futures::StreamExt;
use tokio::net::TcpStream;
use tokio_util::codec::{FramedRead, FramedWrite, LinesCodec};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let addr = Args::parse_socket_addr();
    let mut connection = TcpStream::connect(addr).await?;
    let (reader, writer) = connection.split();
    let mut tcp_writer = FramedWrite::new(writer, LinesCodec::new());
    let mut tcp_reader = FramedRead::new(reader, LinesCodec::new());

    let mut app = App::new();
    let mut terminal = ratatui::init();
    let mut term_stream = crossterm::event::EventStream::new();

    while app.is_running {
        terminal.draw(|f| app.draw_ui(f))?;
        tokio::select! {
            Some(event) = term_stream.next() => {
                let event = Event::Terminal(event?);
                app.handle_event(event, &mut tcp_writer).await?;
            },
            Some(event) = app.event_receiver.recv() => {
                app.handle_event(event, &mut tcp_writer).await?;
            }
            Some(tcp_event) = tcp_reader.next() => {
                app.handle_tcp_event(tcp_event?, &mut tcp_writer).await?;
            },
        }
    }

    ratatui::restore();
    Ok(())
}
