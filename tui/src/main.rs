mod app;
mod args;
mod message_list;
mod popup;
mod room_list;
mod ui;

use app::App;
use args::Args;
use tokio::net::TcpStream;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let addr = Args::parse_socket_addr();
    let connection = TcpStream::connect(addr).await?;
    let app = App::new();
    let terminal = ratatui::init();
    let result = app.run(terminal, connection).await;
    ratatui::restore();
    result
}
