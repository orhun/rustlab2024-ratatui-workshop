mod app;
mod args;
mod message_list;
mod popup;
mod room_list;
mod ui;

use app::App;
use args::Args;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let addr = Args::parse_socket_addr();
    let app = App::new(addr);
    let terminal = ratatui::init();
    let result = app.run(terminal).await;
    ratatui::restore();
    result
}
