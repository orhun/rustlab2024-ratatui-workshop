mod app;
mod logger;
mod message_list;
mod popup;
mod room_list;
mod ui;

use clap::Parser;
use log::LevelFilter;
use std::{
    fs::File,
    net::{IpAddr, Ipv4Addr, SocketAddr},
};
use tracing::Level;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, EnvFilter};

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

fn init_tracing() -> anyhow::Result<WorkerGuard> {
    let file = File::create("tracing.log")?;
    let (non_blocking, guard) = tracing_appender::non_blocking(file);
    let env_filter = EnvFilter::builder()
        .with_default_directive(Level::DEBUG.into())
        .from_env_lossy();
    tracing_subscriber::registry()
        .with(tui_logger::tracing_subscriber_layer())
        .with(fmt::layer().with_writer(non_blocking))
        .with(env_filter)
        .init();
    tui_logger::init_logger(LevelFilter::Debug)?;
    Ok(guard)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let addr = Args::parse_socket_addr();
    let _guard = init_tracing()?;
    let app = App::new(addr);
    let terminal = ratatui::init();
    let result = app.run(terminal).await;
    ratatui::restore();
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;
    use ratatui::{backend::TestBackend, Terminal};

    #[tokio::test]
    async fn test_render_app() -> anyhow::Result<()> {
        let addr = Args::default();
        let addr = SocketAddr::new(addr.ip, addr.port);
        let mut app = App::new(addr);
        let mut terminal = Terminal::new(TestBackend::new(80, 20)).unwrap();
        terminal.draw(|frame| app.draw_ui(frame))?;
        assert_snapshot!(terminal.backend());
        Ok(())
    }
}
