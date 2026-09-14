use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use serde_json::json;
use url::Url;
use std::str;

const POLYMARKET_URL: &str = "wss://ws.perpetuals.polymarket.com/v1/ws";

#[tokio::main]
async fn main() -> Result<(),Box<dyn std::error::Error>> {
    let url = Url::parse(POLYMARKET_URL)?;
    let (mut ws_stream, _) = connect_async(url.as_str()).await?;

    println!("Connected to polymarket time to make fucking money");

    let ping = json!({
        "req": "post",
        "op": {
            "type": "ping"
        }
    });

    ws_stream.send(Message::Text(ping.to_string().into())).await?;

    while let Some(msg) = ws_stream.next().await {
        match msg? {
            Message::Text(text) => {
                println!("Received message {}",text);
            },
            _ => {},
        }
    }
    
    
    Ok(())
}
