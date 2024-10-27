# Server Connection

Let's hook up our application with the server.

## Parsing Command-Line Arguments

You can start the server by running `cargo run -p server` at the workspace root. It uses `127.0.0.1:42069` as default but it is configurable via command-line arguments.

We would like to offer the same flexibilty to our client as well. So let's make it happen via using the [`clap`](https://docs.rs/clap) crate:

```sh
cargo add clap@4.5.20 --features derive
```

Add this to `src/main.rs` before your `main` function:

```rust
use clap::Parser;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

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
```

This will parse `--ip` and `--port` arguments from the command-line. If not provided, it will use the default values.

To do the actual parsing, we can update our `main` function and pass the parsed values to our `App`:

```diff
#[tokio::main]
async fn main() -> anyhow::Result<()> {
-   let app = App::new();
+   let addr = Args::parse_socket_addr();
+   let app = App::new(addr);
```

## Connecting to the Server

For creating the connection, we will be using a [`TcpStream`](https://docs.rs/tokio/latest/tokio/net/struct.TcpStream.html) from `tokio`. Also, we will be _splitting_ the connection into read/write halves with using `into_split` method. This is a pattern that will allow us to read and write to the connection concurrently.

```sh
cargo add tokio-util@0.7.12 --features codec # for framed read/write
```

Here is the code snippet that we will be using:

```rust
use tokio::net::{tcp::OwnedWriteHalf, TcpStream};
use tokio_util::codec::{FramedRead, FramedWrite, LinesCodec};

let connection = TcpStream::connect(addr).await?;
let (reader, writer) = connection.into_split();
let tcp_reader = FramedRead::new(reader, LinesCodec::new());
let tcp_writer = FramedWrite::new(writer, LinesCodec::new());
```

Using the `tcp_writer` we can send any bytes to the server. In our case, we will be sending commands defined in the `common::Command` enum. So it is a good time to add the `common` module to our client:

```sh
cargo add --path ../common/
```

And then update `src/app.rs` as follows:

```rust
use common::Command;
use crossterm::event::{Event, EventStream, KeyCode};
use futures::{SinkExt, StreamExt};
use ratatui::DefaultTerminal;
use std::net::SocketAddr;
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
        let mut tcp_reader = FramedRead::new(reader, LinesCodec::new());
        self.tcp_writer = Some(FramedWrite::new(writer, LinesCodec::new()));

        while self.is_running {
            terminal.draw(|frame| frame.render_widget("Hello Ratatui!", frame.area()))?;

            tokio::select! {
                Some(crossterm_event) = self.term_stream.next() => {
                    let crossterm_event = crossterm_event?;
                    if let Event::Key(key_event) = crossterm_event {
                        if key_event.code == KeyCode::Esc {
                            if let Some(writer) = self.tcp_writer.as_mut() {
                                let _ = writer.send(Command::Quit.to_string()).await;
                            }
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
```

A couple of points to note:

- [`tokio::select!`](https://tokio.rs/tokio/tutorial/select) macro allows us to wait for multiple futures concurrently. In our case, we are waiting for both terminal events and server responses.
- We made `tcp_writer` a part of our `App` struct for easy access. It is an `Option` because we will be setting it after the connection is established.
- We are sending a `Command::Quit` to the server when the user presses the `Esc` key. You can observe that from the server logs:

```
 INFO 127.0.0.1:34710 connected with the name: woodcock
 INFO handle{addr=127.0.0.1:34710 username=woodcock}: Received command: Quit
 INFO handle{addr=127.0.0.1:34710 username=woodcock}: disconnected
```

Now every time you run the TUI, it will connect to the server and read/write events. 🎉
