use tokio::sync::mpsc;

use futures_util::stream::{SplitSink,SplitStream};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async,MaybeTlsStream, WebSocketStream};
use tokio_tungstenite::tungstenite::protocol::Message;
use futures_util::{StreamExt, SinkExt};
use crate::venues::polymarket::types::MarketEvent;

use super::consts::POLYMARKET_WS;

type Ws = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub struct PolymarketWebsocket {
    write: SplitSink<Ws, Message>,
    read: SplitStream<Ws>,
}

impl PolymarketWebsocket {
    pub async fn connect() -> Result<Self, Box<dyn std::error::Error>> {
        let (stream, _resp) = connect_async(POLYMARKET_WS).await?;
        let (write, read) = stream.split();
        Ok(Self { write, read })
    }
    pub async fn subcribe(&mut self, token_ids: &[String]) -> Result<(), Box<dyn std::error::Error>> {
        let payload = serde_json::json!({"assets_ids": token_ids, "type": "market"});
        self.write.send(Message::Text(payload.to_string().into())).await?;
        Ok(())
    }

    pub async fn ping(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.write.send(Message::Text("PING".into())).await?;
        Ok(())
    }
    pub async fn close(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.write.close().await?;
        Ok(())
    }

    pub async fn run(&mut self, tx: mpsc::Sender<MarketEvent>) -> Result<(), Box<dyn std::error::Error>> {
        while let Some(msg) = self.read.next().await {
            let text = match msg? {
                Message::Text(t) => t,
                Message::Ping(_) | Message::Pong(_) => continue,
                Message::Close(_) => break,
                _ => continue,
            };

            let parsed: Result<Vec<MarketEvent>, _> = serde_json::from_str(text.as_str())
            .or_else(|_| serde_json::from_str::<MarketEvent>(&text).map(|e| vec![e]));

            let Ok(events) = parsed else {
                eprintln!("ws: undecodable frame: {text}");
                continue;
            };

            for ev in events {
                if tx.send(ev).await.is_err() {
                    return Ok(());
                }
            }
        }
        Ok(())
    }
}