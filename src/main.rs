use futures_util::{SinkExt, StreamExt};
use std::str;
use tokio::{
    io::{AsyncBufReadExt, Result},
    net::TcpStream,
};
use tokio_tungstenite::{
    connect_async, tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream,
};
use url::Url;

struct WebSocket {
    ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

enum ActiveSymbols {
    Brief,
    Full,
}

impl WebSocket {
    fn new(ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>) -> Self {
        Self { ws_stream }
    }

    async fn send<'a>(&'a mut self, message: String) {
        let res = self.ws_stream.send(Message::Text(message.clone())).await;
        match res {
            Ok(_) => println!("Sent: {}", message),
            Err(e) => println!("Error sending message: {}", e),
        }
    }

    async fn receive<'a>(&'a mut self) -> Option<Message> {
        let res = self.ws_stream.next().await;
        match res {
            Some(Ok(msg)) => Some(msg),
            _ => None,
        }
    }

    fn string_to_json<'a>(&'a mut self, string: String) -> serde_json::Value {
        let json: serde_json::Value = serde_json::from_str(&string).unwrap();
        json
    }

    fn print_json<'a>(&'a mut self, json: serde_json::Value) {
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    }

    // public methods
    pub async fn active_symbols<'a>(&'a mut self, active_symbols: ActiveSymbols) {
        let active_symbols = match active_symbols {
            ActiveSymbols::Brief => "brief",
            ActiveSymbols::Full => "full",
        };
        let message = format!(
            r#"{{"active_symbols": "{}", "product_type": "basic"}}"#,
            active_symbols
        )
        .to_string()
            + "\n";
        self.send(message.clone()).await;
        let res = self.receive().await;
        let res_json = self.string_to_json(res.unwrap().to_text().unwrap().to_string());
        self.print_json(res_json);
    }

    pub async fn close<'a>(&'a mut self) {
        self.ws_stream.close(None).await.unwrap();
        println!("Connection closed");
    }

    pub async fn forget<'a>(&'a mut self, id: String) {
        self.send(format!(r#"{{"forget": {}}}"#, id).to_string())
            .await;
        self.receive().await;
    }

    pub async fn ping<'a>(&'a mut self, interval: u64) {
        let message = r#"{"ping": 1}"#.to_string();

        tokio::time::sleep(tokio::time::Duration::from_millis(interval)).await;
        self.send(message.clone()).await;
        let res = self.receive().await;
        let res_json = self.string_to_json(res.unwrap().to_text().unwrap().to_string());
        self.print_json(res_json);
    }

    pub async fn ticks<'a>(&'a mut self, symbol: String) {
        let message = format!(r#"{{"ticks": "{}"}}"#, symbol).to_string();
        self.send(message.clone()).await;
        let res = self.receive().await;
        let res_str = res.clone().unwrap().to_text().unwrap().to_string();
        let res_json = self.string_to_json(res_str);
        let subscription_id = res_json["subscription"]["id"].to_string();
        self.print_json(res_json);
        self.forget(subscription_id).await;
    }

    pub async fn website_status<'a>(&'a mut self) {
        let message = r#"{"website_status": 1}"#.to_string();
        self.send(message.clone()).await;
        let res = self.receive().await;
        let res_json = self.string_to_json(res.unwrap().to_text().unwrap().to_string());
        self.print_json(res_json);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let url = Url::parse("wss://blue.derivws.com/websockets/v3?app_id=1&l=EN&brand=deriv").unwrap();

    let (ws_stream, _response) = connect_async(url).await.expect("Failed to connect");
    println!("WebSocket handshake has been successfully completed");
    let mut ws = WebSocket::new(ws_stream);

    loop {
        print!("\n> ");
        let mut reader = tokio::io::BufReader::new(tokio::io::stdin());
        let mut buffer = Vec::new();
        let _ = reader.read_until(b'\n', &mut buffer).await;
        if let Ok(line) = str::from_utf8(&buffer) {
            let line = line.trim();
            match line.split_whitespace().next().unwrap() {
                "exit" => {
                    ws.close().await;
                    return Ok(());
                }
                "active_symbols" => {
                    let active_symbols = line.split_whitespace().last().unwrap().to_string();
                    match active_symbols.as_str() {
                        "brief" => ws.active_symbols(ActiveSymbols::Brief).await,
                        "full" => ws.active_symbols(ActiveSymbols::Full).await,
                        _ => {
                            println!("Invalid command");
                        }
                    }
                }
                "forget" => {
                    let id = line.split_whitespace().last().unwrap().to_string();
                    ws.forget(id).await;
                }
                "ping" => {
                    let interval = line.split_whitespace().last().unwrap().to_string();
                    match interval.parse::<u64>() {
                        Ok(interval_ms) => ws.ping(interval_ms).await,
                        Err(_) => ws.ping(0).await,
                    }
                }
                "ticks" => {
                    let symbol = line.split_whitespace().last().unwrap().to_string();
                    ws.ticks(symbol).await;
                }
                "website_status" => ws.website_status().await,
                _ => {
                    println!("Invalid command");
                }
            }
        }
        buffer.clear();
    }
}
