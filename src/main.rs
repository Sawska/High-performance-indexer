mod api;
mod db;
mod scraper;
mod stats;
mod venues;

use std::error::Error;
use venues::polymarket::websocket::PolymarketWebsocket;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let url = std::env::var("DATABASE_URL")?;
    let pool = db::connect(&url).await?;

    sqlx::raw_sql(include_str!("../schema.sql")).execute(&pool).await?;

    let http_addr = std::env::var("HTTP_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
    let http_addr = http_addr.parse()?;
    let web_dir = std::env::var("WEB_DIR").unwrap_or_else(|_| "web/dist".to_string());
    let api_pool = pool.clone();
    tokio::spawn(async move {
        if let Err(e) = api::serve(api_pool, http_addr, web_dir.as_ref()).await {
            eprintln!("api: {e}");
        }
    });

    let scrape = scraper::run(pool.clone(), scraper::Config::from_env());

    let Ok(assets) = std::env::var("ASSET_IDS") else {
        scrape.await;
        return Ok(());
    };

    let token_ids: Vec<String> = assets.split(',').map(str::to_owned).collect();

    let tx = db::writer::spawn(pool.clone());

    let mut ws = PolymarketWebsocket::connect().await?;
    ws.subcribe(&token_ids).await?;

    tokio::select! {
        res = ws.run(tx) => res?,
        _ = scrape => {}
    }

    Ok(())
}
