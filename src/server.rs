use bincode;
use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{future, pin_mut, stream::TryStreamExt, StreamExt};
use log::{error, info};
use std::{
    collections::HashMap,
    io::Error as IoError,
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tokio::{
    net::{TcpListener, TcpStream},
    time::Interval,
};
use tokio_tungstenite::tungstenite::protocol::Message;

use crate::{
    errors::SwapError,
    messages::{SwapRequest, SwapResponse},
};

pub type Tx = UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;
type TokenMap = Arc<Mutex<(HashMap<SocketAddr, String>, HashMap<String, SocketAddr>)>>;

pub struct Server {
    addr: String,
    peer_map: PeerMap,
    token_map: TokenMap,
    timeout: Interval,
}

impl Server {
    pub fn new(addr: String, timeout_secs: u64) -> Self {
        let peer_map = PeerMap::new(Mutex::new(HashMap::new()));
        let token_map = TokenMap::new(Mutex::new((HashMap::new(), HashMap::new())));
        let timeout = tokio::time::interval(tokio::time::Duration::from_secs(timeout_secs));
        Self {
            addr,
            peer_map,
            token_map,
            timeout,
        }
    }

    pub fn send_swap_request_message(
        message: SwapRequest,
        ws_sender: Tx,
        peer_addr: SocketAddr,
    ) -> bool {
        let serialized_message = bincode::serialize(&message)
            .map_err(|e| SwapError::SerializeError(e))
            .expect("Impossible serializing error");
        match ws_sender.unbounded_send(Message::Binary(serialized_message)) {
            Ok(_) => {
                info!("Sent message to {}", peer_addr);
                true
            }
            Err(send_error) => {
                info!("Error sending message to {}: {}", peer_addr, send_error);
                info!("Removing peer {}", peer_addr);
                false
            }
        }
    }

    pub async fn start(mut self) -> Result<(), IoError> {
        let listener = TcpListener::bind(&self.addr)
            .await
            .expect("Failed to bind address");
        info!("Listening on: {}", self.addr);

        loop {
            tokio::select! {
                _ = self.timeout.tick() => {
                    info!("Sending Messages to all peers");
                    let token_map_locked = self.token_map.lock().expect("Token map mutex not poisoned");
                    let mut peers = self.peer_map.lock().expect("Peers Mutex Poisoned");
                    // Send TokenPrice Message to all peers
                    peers.retain(|peer_addr, ws_sink| {
                        if let Some(_) = token_map_locked.0.get(peer_addr) {
                            let message = SwapRequest::TokenPrice;
                            Self::send_swap_request_message(message, ws_sink.clone(), *peer_addr)
                        } else {
                            true
                        }
                    });
                },
                accept_result = listener.accept() => {
                    match accept_result {
                        Ok((stream, addr)) => {
                            tokio::spawn(Server::handle_connection(self.peer_map.clone(), stream, addr, self.token_map.clone()));
                        }
                        Err(e) => {
                            error!("Error aceptando conexión: {}", e);
                        }
                    }
                }
            }
        }
    }

    async fn handle_connection(
        peer_map: PeerMap,
        raw_stream: TcpStream,
        addr: SocketAddr,
        token_map: TokenMap,
    ) {
        info!("Incoming TCP connection from: {}", addr);
        // Create a WebSocket by upgrading the connection from TCP to WS
        let ws_stream = match tokio_tungstenite::accept_async(raw_stream)
            .await
            .map_err(|e| SwapError::WsError(e))
        {
            Ok(ws_stream) => ws_stream,
            Err(e) => {
                error!("Error creating WS connection: {}", e);
                return;
            }
        };
        info!("WebSocket connection established: {}", addr);

        // Update peers map with the new connection
        let (tx, rx) = unbounded();
        // tx.unbounded_send(Message::Text("HOLA".into()))
        //     .expect("Error sending message to peer");
        match peer_map.lock() {
            Ok(mut peers) => {
                info!("Inserting peer {} into peer map", addr);
                peers.insert(addr, tx.clone());
            }
            Err(poisoned) => {
                error!("Error locking peer map: {}", poisoned);
                return;
            }
        };

        let (outgoing, incoming) = ws_stream.split();

        let receive_from_others = rx.map(Ok).forward(outgoing);

        // Send WichToken message to the new peer
        let request = SwapRequest::WhichToken;
        Self::send_swap_request_message(request, tx.clone(), addr);

        let broadcast_incoming = incoming.try_for_each(|msg| {
            info!("Received a message from {}", addr);
            match msg {
                Message::Binary(bytes) => match bincode::deserialize::<SwapResponse>(&bytes) {
                    Ok(message) => match message {
                        SwapResponse::TokenPrice(token_info) => {
                            info!("Received TokenPrice message from {}", addr);
                            // Check addr is valid and token is what we expect
                            let token_map_locked =
                                token_map.lock().expect("Token map mutex not poisoned");
                            if let Some(_) = token_map_locked.0.get(&addr) {
                                info!("TokenPrice: {}", token_info);
                            } else {
                                info!("Not Registered yet");
                            }
                        }
                        SwapResponse::WhichToken(token) => {
                            info!("Received WhichToken message from {}", addr);
                            info!("Token: {}", token);
                            // Check in token map if token is already taken
                            let mut token_map_locked =
                                token_map.lock().expect("Token map mutex not poisoned");
                            if let Some(_) = token_map_locked.1.get(&token) {
                                // Token already taken
                                info!("Token already taken");
                                let response = SwapRequest::RepeatedToken;
                                Self::send_swap_request_message(response, tx.clone(), addr);
                            } else {
                                // Token not taken
                                info!("Token not taken");
                                token_map_locked.0.insert(addr, token.clone());
                                token_map_locked.1.insert(token, addr);
                                let response = SwapRequest::ValidToken;
                                Self::send_swap_request_message(response, tx.clone(), addr);
                            }
                        }
                    },
                    Err(deserialize_error) => {
                        error!(
                            "Error deserializing message from {}: {}",
                            addr, deserialize_error
                        );
                        return future::ok(());
                    }
                },
                _ => {
                    error!("Received a non-binary message from {}", addr);
                }
            };
            future::ok(())
        });

        pin_mut!(broadcast_incoming, receive_from_others);
        future::select(broadcast_incoming, receive_from_others).await;

        info!("{} disconnected", &addr);
        peer_map
            .lock()
            .expect("Peer map mutex not poisoned")
            .remove(&addr);
        let mut token_map_locked = token_map.lock().expect("Token map mutex not poisoned");
        if let Some((_, token)) = token_map_locked.0.remove_entry(&addr) {
            token_map_locked.1.remove(&token);
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use std::env;

//     use super::*;
//     use crate::client::Client;

//     #[test]
//     fn integration_test() {
//         pretty_env_logger::init();
//         let rt = tokio::runtime::Runtime::new().unwrap();
//         rt.block_on(async {
//             let mut server = Server::new("127.0.0.1:8080".to_string(), 10);
//             let mut client1 = Client::new(
//                 "ws://127.0.0.1:8080".to_string(),
//                 "SUI".to_string(),
//                 "https://coins.llama.fi/prices/current/sui:".to_string(),
//             );
//             // Start server
//             let server_handle = tokio::spawn(async move {
//                 server.start().await.unwrap();
//             });
//             tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
//             // Start client
//             let client_handle = tokio::spawn(async move {
//                 client1.start().await;
//             });
//         });
//     }
// }
