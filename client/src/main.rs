use std::{
    io::{self, BufRead},
    net::{IpAddr, Ipv4Addr, SocketAddr},
    thread,
};

use clap::Parser;
use clap_verbosity_flag::{InfoLevel, Verbosity};
use color_eyre::eyre::{bail, WrapErr};
use colored_json::{ColoredFormatter, CompactFormatter};
use common::ServerEvent;
use futures::{SinkExt, StreamExt};
use tokio::{
    net::TcpStream,
    sync::mpsc::{self, UnboundedSender},
};
use tokio_util::codec::{Framed, LinesCodec};
use tracing::level_filters::LevelFilter;
use tracing_log::AsTrace;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let args = Args::parse();

    let level = args.verbosity.log_level_filter().as_trace();
    init_tracing(level);

    let stream = TcpStream::connect(args.address()).await?;
    tracing::info!("Connected to server: {}", stream.local_addr()?);
    let mut server = Framed::new(stream, LinesCodec::new());

    let (stdin_sender, mut stdin_receiver) = mpsc::unbounded_channel();
    thread::spawn(move || read_stdin(stdin_sender));

    let json_formatter = ColoredFormatter::new(CompactFormatter);

    loop {
        tokio::select! {
            line = stdin_receiver.recv() => {
                let Some(line) = line else {
                    tracing::info!("Stdin closed");
                    return Ok(());
                };
                server.send(line).await?;
                server.send("\n".to_string()).await?;
            },
            line = server.next() => {
                let Some(line) = line else {
                    tracing::info!("Server closed connection");
                    return Ok(());
                };
                let line = line.wrap_err("failed to read line from server")?;
                let event = ServerEvent::from_json_str(&line).wrap_err("failed to parse event")?;
                // a real client might do something more interesting with the event here, but for
                // now we just print the event as colored JSON
                println!("{}", json_formatter.clone().to_colored_json_auto(&event)?);
            },
            else => bail!("all streams closed"),
        }
    }
}

/// A thread that reads lines from stdin and sends them to the main part of the program
///
/// This uses standard threads and blocking I/O to read from stdin as the tokio stdin is actually
/// implemented using normal blocking I/O and recommends this approach.
fn read_stdin(sender: UnboundedSender<String>) {
    let mut lines = io::BufReader::new(io::stdin()).lines();
    while let Some(line) = lines.next() {
        match line {
            Ok(line) => {
                if sender.send(line).is_err() {
                    tracing::warn!("server closed connection. message not sent");
                    break;
                }
            }
            Err(err) => {
                tracing::error!("failed to read line from stdin: {err}");
                break;
            }
        }
    }
}

#[derive(Debug, clap::Parser)]
struct Args {
    /// The hostname of the server to connect to
    #[arg(short, long, default_value_t = Ipv4Addr::LOCALHOST.into())]
    ip: IpAddr,

    /// The port to connect to
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
    fn address(&self) -> SocketAddr {
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
