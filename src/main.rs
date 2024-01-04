use futures_util::{SinkExt, StreamExt};
use serde_json;
use std::{cell::RefCell, rc::Rc, str, vec};
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

    async fn subscribe<'a>(&'a mut self, message: String) {
        let mut message = message.split("{").collect::<Vec<&str>>();
        message[0] = "{\"subscribe\": 1, ";

        let res = self
            .ws_stream
            .send(Message::Text(message.join("").to_string()))
            .await;
        match res {
            Ok(_) => println!("Sent: {}", message.join("").to_string()),
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
    pub async fn active_symbols<'a>(&'a mut self, active_symbols: String) {
        let active_symbols = match active_symbols {
            x if vec!["brief".to_string(), "full".to_string()].contains(&x) => x,
            _ => panic!("Invalid active_symbols"),
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

    pub async fn ping<'a>(&'a mut self, sleep: u64) {
        let message = r#"{"ping": 1}"#.to_string();
        match sleep {
            0 => self.send(message.clone()).await,
            _ => {
                tokio::time::sleep(tokio::time::Duration::from_millis(sleep)).await;
                self.subscribe(message.clone()).await
            }
        }

        let res = self.receive().await;
        let res_json = self.string_to_json(res.unwrap().to_text().unwrap().to_string());
        self.print_json(res_json);
    }

    pub async fn ticks<'a>(&'a mut self, symbol: String) {
        let message = format!(r#"{{"ticks": "{}"}}"#, symbol).to_string();
        self.subscribe(message.clone()).await;
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
    let ws = Rc::new(RefCell::new(WebSocket::new(ws_stream)));

    loop {
        print!("\n> ");
        let mut reader = tokio::io::BufReader::new(tokio::io::stdin());
        let mut buffer = Vec::new();
        let _ = reader.read_until(b'\n', &mut buffer).await;
        if let Ok(line) = str::from_utf8(&buffer) {
            let line = line.trim();
            match line.split_whitespace().next().unwrap() {
                "exit" => {
                    ws.borrow_mut().close().await;
                    return Ok(());
                }
                "active_symbols" => {
                    let active_symbols = line.split_whitespace().last().unwrap().to_string();
                    match active_symbols.as_str() {
                        x if vec!["brief", "full"].contains(&x) => {
                            ws.borrow_mut().active_symbols(x.to_string()).await
                        }
                        _ => {
                            println!("Invalid command");
                        }
                    }
                }
                "forget" => {
                    let id = line.split_whitespace().last().unwrap().to_string();
                    ws.borrow_mut().forget(id).await;
                }
                "ping" => {
                    let sleep = line.split_whitespace().last().unwrap().to_string();
                    match sleep.parse::<u64>() {
                        Ok(sleep_ms) => ws.borrow_mut().ping(sleep_ms).await,
                        Err(_) => ws.borrow_mut().ping(0).await,
                    }
                }
                "ticks" => {
                    let symbol = line.split_whitespace().last().unwrap().to_string();
                    ws.borrow_mut().ticks(symbol).await;
                }
                "website_status" => ws.borrow_mut().website_status().await,
                _ => {
                    println!("Invalid command");
                }
            }
        }
        buffer.clear();
    }
}
