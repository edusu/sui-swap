use serde::{Deserialize, Serialize};

use crate::models::TokenInfoResponse;

#[derive(Serialize, Deserialize, Debug)]
pub enum SwapRequest {
    WhichToken,
    ValidToken,
    RepeatedToken,
    TokenPrice,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum SwapResponse {
    WhichToken(String),
    TokenPrice(TokenInfoResponse),
}
