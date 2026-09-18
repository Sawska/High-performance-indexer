use futures_util::stream::{SplitSink,SplitStream};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async,MaybeTlsStream, WebSocketStream};
use tokio_tungstenite::tungstenite::protocol::Message;
use futures_util::{StreamExt, SinkExt};
use super::consts::{POLYMARKET_API, POLYMARKET_WS};

type Ws = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub struct PolymarketWebsocket {
    write: SplitSink<Ws, Message>,
    read: SplitStream<Ws>,
}

impl PolymarketWebsocket {
    pub async fn connect() -> Result<Self, Box<dyn std::error::Error>> {
        let (stream, _resp) = connect_async(POLYMARKET_API).await?;
        let (write, read) = stream.split();
        Ok(Self { write, read })
    }
    pub async fn subcribe(&mut self, token_ids: &[str]) -> Result<(), Box<dyn std::error::Error>> {
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
        self.read.close().await?;
        Ok(())
    }
}