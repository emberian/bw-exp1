//! WebSocket client for server communication

use anyhow::Result;
use bw_shared::messages::{ClientMessage, ServerMessage, deserialize_message, serialize_message};
use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// Events from the network layer
#[derive(Debug)]
pub enum NetworkEvent {
    /// Successfully connected
    Connected,
    /// Disconnected (with reason)
    Disconnected(String),
    /// Received a message from server
    Message(ServerMessage),
    /// Network error
    Error(String),
}

/// Network client for communicating with the game server
pub struct NetworkClient {
    /// Server URL
    server_url: String,
    /// Auth token
    token: Option<String>,
    /// Channel to send messages to the server
    tx: mpsc::UnboundedSender<ClientMessage>,
    /// Receiver for outgoing messages (used internally)
    rx: Option<mpsc::UnboundedReceiver<ClientMessage>>,
    /// Channel to receive events from the network task
    event_tx: mpsc::UnboundedSender<NetworkEvent>,
    /// Handle to the network task
    task_handle: Option<tokio::task::JoinHandle<()>>,
}

impl NetworkClient {
    /// Create a new network client
    pub fn new(
        server_url: &str,
        token: Option<String>,
    ) -> (Self, mpsc::UnboundedReceiver<NetworkEvent>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        (
            Self {
                server_url: server_url.to_string(),
                token,
                tx,
                rx: Some(rx),
                event_tx,
                task_handle: None,
            },
            event_rx,
        )
    }

    /// Connect to the server
    pub async fn connect(&mut self) -> Result<()> {
        let url = self.server_url.clone();
        let token = self.token.clone();
        let event_tx = self.event_tx.clone();
        let rx = self.rx.take().expect("connect called twice");

        // Spawn the network task
        let handle = tokio::spawn(async move {
            if let Err(e) = run_connection(url, token, rx, event_tx.clone()).await {
                let _ = event_tx.send(NetworkEvent::Error(e.to_string()));
            }
        });

        self.task_handle = Some(handle);
        Ok(())
    }

    /// Send a message to the server
    pub fn send(&self, msg: ClientMessage) -> Result<()> {
        self.tx.send(msg)?;
        Ok(())
    }

    /// Disconnect from the server
    pub async fn disconnect(&mut self) {
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
        }
    }
}

async fn run_connection(
    url: String,
    token: Option<String>,
    mut rx: mpsc::UnboundedReceiver<ClientMessage>,
    event_tx: mpsc::UnboundedSender<NetworkEvent>,
) -> Result<()> {
    // Connect to WebSocket
    let (ws_stream, _) = connect_async(&url).await?;
    let (mut write, mut read) = ws_stream.split();

    // Notify connected
    let _ = event_tx.send(NetworkEvent::Connected);

    // Send auth message if we have a token
    if let Some(token) = token {
        let auth_msg = ClientMessage::Authenticate { token };
        let bytes = serialize_message(&auth_msg)?;
        write.send(Message::Binary(bytes)).await?;
    }

    // Main loop - handle both incoming and outgoing messages
    loop {
        tokio::select! {
            // Outgoing message from application
            Some(msg) = rx.recv() => {
                let bytes = serialize_message(&msg)?;
                if let Err(e) = write.send(Message::Binary(bytes)).await {
                    let _ = event_tx.send(NetworkEvent::Disconnected(e.to_string()));
                    break;
                }
            }

            // Incoming message from server
            Some(result) = read.next() => {
                match result {
                    Ok(Message::Binary(bytes)) => {
                        match deserialize_message::<ServerMessage>(&bytes) {
                            Ok(msg) => {
                                let _ = event_tx.send(NetworkEvent::Message(msg));
                            }
                            Err(e) => {
                                let _ = event_tx.send(NetworkEvent::Error(
                                    format!("Failed to deserialize message: {}", e)
                                ));
                            }
                        }
                    }
                    Ok(Message::Close(_)) => {
                        let _ = event_tx.send(NetworkEvent::Disconnected("Server closed connection".into()));
                        break;
                    }
                    Ok(Message::Ping(data)) => {
                        let _ = write.send(Message::Pong(data)).await;
                    }
                    Ok(_) => {
                        // Ignore other message types (Text, Pong, Frame)
                    }
                    Err(e) => {
                        let _ = event_tx.send(NetworkEvent::Disconnected(e.to_string()));
                        break;
                    }
                }
            }

            // Both channels closed
            else => {
                let _ = event_tx.send(NetworkEvent::Disconnected("Connection ended".into()));
                break;
            }
        }
    }

    Ok(())
}
