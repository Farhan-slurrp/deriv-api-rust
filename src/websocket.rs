use futures_util::{SinkExt, StreamExt};
use serde_json;
use std::{str, vec};
use tokio::net::TcpStream;
use tokio_tungstenite::{tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream};

pub struct WebSocket {
    ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl WebSocket {
    fn string_to_json<'a>(&'a mut self, string: String) -> serde_json::Value {
        let json: serde_json::Value = serde_json::from_str(&string).unwrap();
        json
    }

    fn print_json<'a>(&'a mut self, json: serde_json::Value) {
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    }

    async fn receive<'a>(&'a mut self) -> Option<Message> {
        let res = self.ws_stream.next().await;
        match res {
            Some(Ok(msg)) => Some(msg),
            _ => None,
        }
    }

    async fn get_response<'a>(&'a mut self) {
        while let Some(res) = self.receive().await {
            let res_json = self.string_to_json(res.to_text().unwrap().to_string());
            self.print_json(res_json);
        }
    }

    // public methods
    pub fn new(ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>) -> Self {
        Self { ws_stream }
    }

    pub async fn send<'a>(&'a mut self, message: String) {
        let res = self.ws_stream.send(Message::Text(message.clone())).await;
        match res {
            Ok(_) => println!("Sent: {}", message),
            Err(e) => println!("Error sending message: {}", e),
        }
        self.get_response().await;
    }

    pub async fn subscribe<'a>(&'a mut self, message: String) {
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
        self.get_response().await;
    }

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
    }

    pub async fn close<'a>(&'a mut self) {
        self.ws_stream.close(None).await.unwrap();
        println!("Connection closed");
    }

    pub async fn forget<'a>(&'a mut self, id: String) {
        self.send(format!(r#"{{"forget": {}}}"#, id).to_string())
            .await;
    }

    pub async fn ping<'a>(&'a mut self) {
        let message = r#"{"ping": 1}"#.to_string();
        self.send(message.clone()).await;
    }

    pub async fn ticks<'a>(&'a mut self, symbol: String) {
        let message = format!(r#"{{"ticks": "{}"}}"#, symbol).to_string();
        self.subscribe(message.clone()).await;
    }

    pub async fn website_status<'a>(&'a mut self) {
        let message = r#"{"website_status": 1}"#.to_string();
        self.send(message.clone()).await;
    }
}
