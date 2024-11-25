use chrono::{TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt::{self, Debug},
};

#[derive(Serialize, Deserialize, Debug)]
pub struct TokenInfoResponse {
    pub coins: HashMap<String, TokenInfoInnerResponse>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TokenInfoInnerResponse {
    pub confidence: f64,
    pub decimals: u64,
    pub price: f64,
    pub symbol: String,
    pub timestamp: TimeStamp,
}

impl fmt::Display for TokenInfoResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = String::new();
        for (key, value) in &self.coins {
            s.push_str(&format!("\nContract Address: {}\n{}", key, value));
        }
        write!(f, "{}", s)
    }
}

impl fmt::Display for TokenInfoInnerResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Symbol: {}\nPrice: {}\nDecimals: {}\nConfidence: {}\nTimestamp: {}",
            self.symbol, self.price, self.decimals, self.confidence, self.timestamp
        )
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TimeStamp(pub u64);

impl TimeStamp {
    pub fn to_datetime_string(&self) -> String {
        // Convertir nanosegundos a segundos
        let seconds = (self.0) as i64;
        match Utc.timestamp_opt(seconds as i64, 0).single() {
            Some(datetime) => datetime.format("%d-%m-%Y %H:%M:%S").to_string(),
            None => String::from("Invalid timestamp"),
        }
    }
}

impl fmt::Display for TimeStamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_datetime_string())
    }
}
