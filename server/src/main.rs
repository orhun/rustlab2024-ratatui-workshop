use clap::{
    builder::{styling::AnsiColor, Styles},
    Parser,
};
use clap_verbosity_flag::{InfoLevel, Verbosity};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tracing::level_filters::LevelFilter;
use tracing_log::AsTrace;
use tracing_subscriber::EnvFilter;

use self::server::Server;

mod connection;
mod room;
mod rooms;
mod server;
mod users;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();
    let level = args.verbosity.log_level_filter().as_trace();
    init_tracing(level);
    tracing::debug!("Starting server with args: {:#?}", args);
    let server = Server::listen(args.address()).await?;
    server.run().await;
    Ok(())
}

const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Blue.on_default().bold())
    .usage(AnsiColor::Blue.on_default().bold())
    .literal(AnsiColor::White.on_default())
    .placeholder(AnsiColor::Green.on_default());

#[derive(Debug, Parser)]
#[command(styles = STYLES)]
pub struct Args {
    /// The IP address to listen on
    #[arg(short, long, default_value_t = Ipv4Addr::LOCALHOST.into())]
    ip: IpAddr,

    /// The port to listen on
    #[arg(short, long, default_value_t = 42069)]
    port: u16,

    /// Verbosity flags
    ///
    /// Automatically parses one or more --verbose and --quiet flags to set the log level.
    /// Default level is INFO. Use -v to increase the log level, and -q to decrease it.
    #[command(flatten)]
    verbosity: Verbosity<InfoLevel>,
}

impl Args {
    pub fn address(&self) -> SocketAddr {
        SocketAddr::new(self.ip, self.port)
    }
}

pub fn init_tracing(level_filter: LevelFilter) {
    let env_filter = EnvFilter::builder()
        .with_default_directive(level_filter.into())
        .from_env_lossy();
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .without_time()
        .init();
}
