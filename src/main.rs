use client::Client;
use dotenv::dotenv;
use log::error;
use server::Server;
use std::env;
mod client;
mod errors;
mod messages;
mod models;
mod server;

const TOKEN_BALANCE_ENV: &str = "TOKEN_BALANCE_URL";

#[tokio::main]
async fn main() {
    dotenv().ok();
    pretty_env_logger::init();
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "-c" => run_c().await,
            "-s" => run_s().await,
            _ => println!("Invalid argument"),
        }
    } else {
        println!("USE: -c <URL> <TOKEN> or -s <ADDR>");
    }
}

async fn run_c() {
    let url = env::args()
        .nth(2)
        .unwrap_or_else(|| panic!("USE: -c <URL> <TOKEN>"));
    let token = env::args()
        .nth(3)
        .unwrap_or_else(|| panic!("USE: -c <URL> <TOKEN>"));
    let Ok(token_balance_url) = env::var(TOKEN_BALANCE_ENV) else {
        panic!("{} env var is not set", TOKEN_BALANCE_ENV);
    };
    println!("URL: {}", url);
    Client::new(url, token, token_balance_url).start().await;
}

async fn run_s() {
    let addr = env::args()
        .nth(2)
        .unwrap_or_else(|| "127.0.0.1:8080".to_string());
    println!("ADDR: {}", addr);
    if let Err(server_error) = Server::new(addr, 10).start().await {
        error!("Server error: {}", server_error);
    };
}
