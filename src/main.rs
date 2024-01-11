mod websocket;

use clap::Parser;
use std::{
    sync::{Arc, Mutex},
    vec,
};
use tokio::io::Result;
use tokio_tungstenite::connect_async;
use url::Url;
use websocket::WebSocket;

#[derive(Parser, Debug)]
#[command(name = "Deriv API")]
#[command(author = "Farhan Ahmad Nurzi")]
#[command(version = "1.0")]
#[command(about = "This is your deriv API CLI Playground", long_about = None)]
struct Cli {
    /// Endpoint to call with payload divided by space (e.g. --e 'active_symbols brief')
    #[arg(short, long)]
    e: String,
    /// API token
    p: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let url = Url::parse("wss://blue.derivws.com/websockets/v3?app_id=1&l=EN&brand=deriv").unwrap();

    let (ws_stream, _response) = connect_async(url).await.expect("Failed to connect");
    println!("WebSocket handshake has been successfully completed");
    let ws = Arc::new(Mutex::new(WebSocket::new(ws_stream)));

    let cli = Cli::parse();

    let arg = cli.e.clone();
    let endpoint = arg.split(" ").collect::<Vec<&str>>()[0].to_string();

    match endpoint.as_str() {
        "exit" => {
            ws.lock().unwrap().close().await;
            return Ok(());
        }
        "active_symbols" => {
            let active_symbols = arg.split(" ").collect::<Vec<&str>>()[1].to_string();
            match active_symbols.as_str() {
                x if vec!["brief", "full"].contains(&x) => {
                    ws.lock().unwrap().active_symbols(x.to_string()).await
                }
                _ => {
                    println!("Invalid command");
                }
            }
        }
        "forget" => {
            let id = arg.split(" ").collect::<Vec<&str>>()[1].to_string();
            ws.lock().unwrap().forget(id).await;
        }
        "ping" => ws.lock().unwrap().ping().await,
        "ticks" => {
            let symbol = arg.split(" ").collect::<Vec<&str>>()[1].to_string();
            ws.lock().unwrap().ticks(symbol).await;
        }
        "website_status" => ws.lock().unwrap().website_status().await,
        _ => {
            println!("Invalid command");
        }
    }

    Ok(())
}
