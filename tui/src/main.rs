use clap::Parser;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

mod app;
mod message_list;
mod ui;

use app::App;

pub const DEFAULT_IP: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);
pub const DEFAULT_PORT: u16 = 42069;

#[derive(Parser)]
pub struct Args {
    #[arg(short, long, default_value_t = DEFAULT_IP)]
    ip: IpAddr,

    #[arg(short, long, default_value_t = DEFAULT_PORT)]
    port: u16,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            ip: DEFAULT_IP,
            port: DEFAULT_PORT,
        }
    }
}

impl Args {
    pub fn parse_socket_addr() -> SocketAddr {
        let cli = Self::parse();
        SocketAddr::new(cli.ip, cli.port)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let addr = Args::parse_socket_addr();
    let app = App::new(addr);
    let terminal = ratatui::init();
    let result = app.run(terminal).await;
    ratatui::restore();
    result
}
