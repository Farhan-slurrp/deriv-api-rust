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

    async fn send(&mut self, message: String) {
        self.ws_stream
            .send(Message::Text(message.clone()))
            .await
            .unwrap();
    }

    async fn receive(&mut self) -> Option<Message> {
        let res = self.ws_stream.next().await;
        match res {
            Some(Ok(msg)) => Some(msg),
            _ => None,
        }
    }

    // public methods
    pub async fn close(&mut self) {
        self.ws_stream.close(None).await.unwrap();
        println!("Connection closed");
    }

    pub async fn active_symbols(&mut self, active_symbols: ActiveSymbols) {
        let active_symbols = match active_symbols {
            ActiveSymbols::Brief => "brief",
            ActiveSymbols::Full => "full",
        };
        let message = format!(
            r#"{{"active_symbols": "{}", "product_type": "basic"}}\n"#,
            active_symbols
        )
        .to_string()
            + "\n";
        self.send(message.clone()).await;
        let res = self.receive().await;
        println!("{:?}", res);
    }

    pub async fn forget(&mut self, id: String) {
        self.send(format!(r#"{{"forget": "{}"}}"#, id).to_string() + "\n")
            .await;
        let res = self.receive().await;
        println!("{:?}", res);
    }

    pub async fn ping(&mut self, interval: u64) {
        let message = r#"{
            "ping": 1
        }"#
        .to_string()
            + "\n";

        tokio::time::sleep(tokio::time::Duration::from_millis(interval)).await;
        self.send(message.clone()).await;
        let res = self.receive().await;
        println!("{:?}", res);
    }

    pub async fn ticks(&mut self, symbol: String) {
        let message = format!(r#"{{"ticks": "{}", "subscribe": 1}}"#, symbol).to_string() + "\n";
        self.send(message.clone()).await;
        let res = self.receive().await;
        println!("{:?}", res);
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
                _ => {
                    println!("Invalid command");
                }
            }
        }
        buffer.clear();
    }
}
