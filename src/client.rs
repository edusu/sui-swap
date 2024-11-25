use futures::TryStreamExt;
use futures_util::{future, pin_mut, StreamExt};
use log::{debug, error, info};
use serde_json::Value;
use std::io::Read;
use std::process;
use std::{error::Error, fs::File};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

use crate::server::Tx;
use crate::{
    errors::SwapError,
    messages::{SwapRequest, SwapResponse},
    models::TokenInfoResponse,
};

pub struct Client {
    url: String,
    token: String,
    token_balance_url: String,
}

impl Client {
    pub fn new(url: String, token: String, token_balance_url: String) -> Self {
        Self {
            url,
            token,
            token_balance_url,
        }
    }

    /// Refactor for sent messages to server
    pub fn send_swap_response_message(
        message: SwapResponse,
        ws_sender: Tx,
    ) -> Result<(), SwapError> {
        let serialized_message =
            bincode::serialize(&message).map_err(|e| SwapError::SerializeError(e))?;
        match ws_sender
            .unbounded_send(Message::binary(serialized_message))
            .map_err(|e| SwapError::SendRequestError(e.to_string()))
        {
            Ok(_) => Ok(()),
            Err(error) => {
                error!("Error sending token price to server: {}", error);
                return Err(error);
            }
        }
    }

    /// Main function for the client
    pub async fn start(self) {
        let token_address = self.get_token_address();
        debug!("Token address: {}", token_address);

        // Websocket connection with server
        let (ws_stream, _) = connect_async(&self.url)
            .await
            .expect("Failed to connect to server");
        info!("WebSocket handshake has been successfully completed");
        //
        let (tx, rx) = futures_channel::mpsc::unbounded();

        let (outgoing, incoming) = ws_stream.split();
        // Send messages to Server
        let in_to_ws = rx.map(Ok).forward(outgoing);
        // Receive messages from Server
        let ws_to_server = incoming.try_for_each(|msg| {
            info!("Received a message from server");
            match msg {
                Message::Binary(bytes) => match bincode::deserialize::<SwapRequest>(&bytes) {
                    Ok(message) => {
                        info!("Received message: {:?}", message);
                        match message {
                            // Send token price to server
                            SwapRequest::TokenPrice => {
                                tokio::spawn(Client::get_token_price(
                                    self.token_balance_url.clone(),
                                    token_address.clone(),
                                    tx.clone(),
                                ));
                            }
                            // Send Token Name to Server
                            SwapRequest::WhichToken => {
                                let response = SwapResponse::WhichToken(self.token.clone());
                                Client::send_swap_response_message(response, tx.clone())
                                    .expect("Error sending WhichToken message to server");
                            }
                            // Server responded our token is valid
                            SwapRequest::ValidToken => {
                                info!("Received ValidToken message from server");
                            }
                            // Server responded our token is invalid
                            SwapRequest::RepeatedToken => {
                                error!("Received RepeatedToken message from server");
                                // Finish the connection
                                process::exit(0);
                            }
                        }
                    }
                    Err(deserialize_error) => {
                        error!(
                            "Error deserializing message from server: {}",
                            deserialize_error
                        );
                        return future::ok(());
                    }
                },
                _ => {
                    error!("Received a non-binary message from server");
                }
            };
            future::ok(())
        });

        // Listen in both futures, outcoming and incoming messages
        pin_mut!(in_to_ws, ws_to_server);
        future::select(in_to_ws, ws_to_server).await;
    }

    /// Get token address from tokens.json file
    fn get_token_address(&self) -> String {
        let mut file = File::open("tokens.json").expect("Unable to open tokens file");
        let mut data = String::new();
        file.read_to_string(&mut data)
            .expect("Unable to read  tokens file");
        let v: Value = serde_json::from_str(&data).expect("JSON was not well-formatted");
        let token_address = v[&self.token]
            .as_str()
            .expect("Token Selected is not listed in tokens.json");
        token_address.to_string()
    }

    /// Get token price from token_balance_url
    async fn get_token_price(
        token_balance_url: String,
        token_address: String,
        tx: futures_channel::mpsc::UnboundedSender<Message>,
    ) -> Result<(), SwapError> {
        let full_url = format!("{}{}", token_balance_url, token_address);
        info!("Getting token price from: {}", full_url);
        let response = match reqwest::get(&full_url)
            .await
            // .map_err(|e| SwapError::SendRequestError(e.to_string()))
        {
            Ok(response) => response,
            Err(error) => {
                error!("Error getting token price: {:?}", error);
                error!("ERROR SOURCE: {:?}", error.source());
                return Err(SwapError::SendRequestError(error.to_string()));
            }
        };
        let token_price_result = response
            .json::<TokenInfoResponse>()
            .await
            .map_err(|e| SwapError::ParseResponseError(e));
        let token_price = match token_price_result {
            Ok(token_price) => token_price,
            Err(error) => {
                error!("Error parsing token price: {}", error);
                return Err(error);
            }
        };
        info!("Token price: {}", token_price);
        let message = SwapResponse::TokenPrice(token_price);
        Client::send_swap_response_message(message, tx)
    }
}
