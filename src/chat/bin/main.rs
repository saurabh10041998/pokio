#![warn(rust_2018_idioms)]

use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex};
use tokio_stream::StreamExt;
use tokio_util::codec::{Framed, LinesCodec};

use futures::SinkExt;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;

type Tx = mpsc::UnboundedSender<String>;
type Rx = mpsc::UnboundedReceiver<String>;

struct Shared {
    peers: HashMap<SocketAddr, Tx>,
}

struct Peer {
    lines: Framed<TcpStream, LinesCodec>,
    rx: Rx,
}

impl Shared {
    fn new() -> Self {
        Shared {
            peers: HashMap::new(),
        }
    }

    async fn broadcast(&mut self, sender: SocketAddr, message: &str) {
        for peer in self.peers.iter_mut() {
            if *peer.0 != sender {
                let _ = peer.1.send(message.into());
            }
        }
    }
}

impl Peer {
    async fn new(
        state: Arc<Mutex<Shared>>,
        lines: Framed<TcpStream, LinesCodec>,
    ) -> io::Result<Self> {
        // Get the client socket address
        let addr = lines.get_ref().peer_addr()?;

        // Create the channel for this peer
        let (tx, rx) = mpsc::unbounded_channel();

        //Add an entry for this `peer` in the shared state map
        state.lock().await.peers.insert(addr, tx);

        Ok(Peer { lines, rx })
    }
}

// Process individual chat client

async fn process(
    state: Arc<Mutex<Shared>>,
    stream: TcpStream,
    addr: SocketAddr,
) -> Result<(), Box<dyn Error>> {
    let mut lines = Framed::new(stream, LinesCodec::new());

    lines.send("Please enter your username: ").await?;

    let username = match lines.next().await {
        Some(Ok(line)) => line,
        _ => {
            tracing::error!("Failed to get username from {}. Client disconnected.", addr);
            return Ok(());
        }
    };
    let mut peer = Peer::new(state.clone(), lines).await?;

    {
        let mut state = state.lock().await;
        let msg = format!("{} has joined the chat", username);
        tracing::info!("{}", msg);
        state.broadcast(addr, &msg).await;
    }

    // Process incoming messages until our stream is exhausted by disconnect.

    loop {
        tokio::select! {
            // message was received from peer send to current user
            Some(msg) = peer.rx.recv() => {
                peer.lines.send(&msg).await?;
            }

            result = peer.lines.next() => match result {
                Some(Ok(msg)) => {
                    let mut state = state.lock().await;
                    let msg = format!("{}: {}", username, msg);

                    state.broadcast(addr, &msg).await;
                },

                Some(Err(e)) => {
                    tracing::error!(
                        "an error occured while processing message for {}; error = {:?}",
                        username,
                        e
                    );
                }

                None => break,

            }

        }
    }

    // If this section is reached, it means the client was disconnected!
    // Let's let everyone still connected know about it.

    {
        let mut state = state.lock().await;
        state.peers.remove(&addr);

        let msg = format!("{} has left the chat", username);
        tracing::info!("{}", msg);
        state.broadcast(addr, &msg).await;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    use tracing_subscriber::{fmt::format::FmtSpan, EnvFilter};
    // Configure a `tracing` subsriber that logs traces emitted
    // by chat server

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("chat=info".parse()?))
        .with_span_events(FmtSpan::FULL)
        .init();

    let state = Arc::new(Mutex::new(Shared::new()));

    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:6142".to_string());

    let listener = TcpListener::bind(&addr).await?;

    tracing::info!("server running on {}", addr);

    loop {
        let (stream, addr) = listener.accept().await?;
        let state = Arc::clone(&state);
        tokio::spawn(async move {
            tracing::debug!("Accepted connection");
            if let Err(e) = process(state, stream, addr).await {
                tracing::info!("an error occured; error = {:?}", e);
            }
        });
    }

    Ok(())
}
