use super::consts::POLYMARKET_WS_URL;
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message};
use url::Url;

struct PolymarketWs {
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl PolymarketWs {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let url = Url::parse(POLYMARKET_WS_URL)?;
        let (ws_stream, _) = connect_async(url.as_str()).await?;

        println!("Connection of polymarket websocket established");

        Ok(Self { ws: ws_stream })
    }

    async fn ping(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let ping = json!({
            "req": "post",
            "op": {
                "type": "ping"
            }
        });

        self.ws.send(Message::Text(ping.to_string().into())).await?;

        while let Some(msg) = self.ws.next().await {
            match msg? {
                Message::Text(text) => {
                    println!("Received message {}", text);
                }
                _ => {}
            }
        }

        Ok(())
    }
}
