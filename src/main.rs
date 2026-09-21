mod db;
mod venues;

use std::error::Error;
use venues::polymarket::websocket::PolymarketWebsocket;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let url = std::env::var("DATABASE_URL")?;
    let pool = db::connect(&url).await?;

    sqlx::raw_sql(include_str!("../schema.sql")).execute(&pool).await?;

    let tx = db::writer::spawn(pool.clone());

    let token_ids: Vec<String> = std::env::var("ASSET_IDS")?
    .split(',')
    .map(str::to_owned)
    .collect();

    let mut ws = PolymarketWebsocket::connect().await?;
    ws.subcribe(&token_ids).await?;
    ws.run(tx).await?;

    Ok(())
}