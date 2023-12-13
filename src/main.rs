use futures_util::{SinkExt, StreamExt};
use tokio::{
    io::{AsyncWriteExt, Result},
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

        println!("sent");
    }

    async fn receive(&mut self) -> Option<Message> {
        match self.ws_stream.next().await {
            Some(message) => {
                let data = message.unwrap();
                let message = Message::from(data.clone());
                println!("data: {:?}", message);
                tokio::io::stdout().write(&data.into_data()).await.unwrap();
                Some(message)
            }
            None => {
                println!("No message received");
                None
            }
        }
    }

    // public methods
    pub async fn close(&mut self) {
        self.ws_stream.close(None).await.unwrap();
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
        self.receive().await;
    }

    pub async fn forget(&mut self, id: String) {
        self.send(format!(r#"{{"forget": "{}"}}"#, id).to_string() + "\n")
            .await;
    }

    pub async fn ping(&mut self, interval: u64) {
        let message = r#"{
            "ping": 1
        }"#
        .to_string()
            + "\n";

        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(interval)).await;
            self.send(message.clone()).await;
            self.receive().await;
        }
    }

    pub async fn ticks(&mut self, symbol: String) {
        let message = format!(r#"{{"ticks": "{}", "subscribe": 1}}"#, symbol).to_string() + "\n";
        self.send(message.clone()).await;
        loop {
            let message = self.receive().await;
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let url = Url::parse("wss://blue.derivws.com/websockets/v3?app_id=1&l=EN&brand=deriv").unwrap();

    let (ws_stream, _response) = connect_async(url).await.expect("Failed to connect");
    println!("WebSocket handshake has been successfully completed");
    let mut ws = WebSocket::new(ws_stream);
    ws.ping(2000).await;
    // ws.ticks("R_50".to_string()).await;
    // ws.active_symbols(ActiveSymbols::Brief).await;

    Ok(())
}
