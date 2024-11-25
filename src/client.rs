use futures::TryStreamExt;
use futures_util::{future, pin_mut, StreamExt};
use log::{error, info};
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

    pub async fn start(self) {
        let token_address = self.get_token_address();
        info!("Token address: {}", token_address);
        // self.get_token_price().await;
        // let (stdin_tx, stdin_rx) = futures_channel::mpsc::unbounded();
        // tokio::spawn(read_stdin(stdin_tx));

        let (ws_stream, _) = connect_async(&self.url)
            .await
            .expect("Failed to connect to server");
        info!("WebSocket handshake has been successfully completed");
        let (tx, rx) = futures_channel::mpsc::unbounded();

        let (outgoing, incoming) = ws_stream.split();
        let in_to_ws = rx.map(Ok).forward(outgoing);
        let ws_to_server = incoming.try_for_each(|msg| {
            info!("Received a message from server");
            match msg {
                Message::Binary(bytes) => match bincode::deserialize::<SwapRequest>(&bytes) {
                    Ok(message) => {
                        info!("Received message: {:?}", message);
                        match message {
                            SwapRequest::TokenPrice => {
                                tokio::spawn(Client::get_token_price(
                                    self.token_balance_url.clone(),
                                    token_address.clone(),
                                    tx.clone(),
                                ));
                            }
                            SwapRequest::WhichToken => {
                                let response = SwapResponse::WhichToken(self.token.clone());
                                Client::send_swap_response_message(response, tx.clone())
                                    .expect("Error sending WhichToken message to server");
                            }
                            SwapRequest::ValidToken => {
                                info!("Received ValidToken message from server");
                            }
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

        pin_mut!(in_to_ws, ws_to_server);
        future::select(in_to_ws, ws_to_server).await;
    }

    fn get_token_address(&self) -> String {
        let mut file = File::open("tokens.json").unwrap();
        let mut data = String::new();
        file.read_to_string(&mut data)
            .expect("Unable to read  tokens file");
        let v: Value = serde_json::from_str(&data).unwrap();
        let token_address = v[&self.token].as_str().unwrap();
        token_address.to_string()
    }

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