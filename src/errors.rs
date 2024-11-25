use thiserror::Error;

#[derive(Error, Debug)]
pub enum SwapError {
    #[error("Failed to read tokens file")]
    ReadTokensFileError(#[from] std::io::Error),
    #[error("Failed to parse tokens file")]
    ParseTokensFileError(#[from] serde_json::Error),
    #[error("Failed to send request to: {0}")]
    SendRequestError(String),
    #[error("Failed to parse response")]
    ParseResponseError(#[from] reqwest::Error),
    #[error("Failed to serialize response")]
    SerializeError(#[from] bincode::Error),
    #[error("WS error")]
    WsError(#[from] tokio_tungstenite::tungstenite::Error),
}
