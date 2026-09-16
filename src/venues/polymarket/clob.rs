use super::consts::CLOB_API;
use super::types::PriceHistoryResponse;
use reqwest::Client;

struct ClobApi {
    client: reqwest::Client,
}

impl ClobApi {
    fn new() -> Self {
        let client = reqwest::Client::new();

        Self { client }
    }

    async fn get_history(
        &self,
        markte: &str,
    ) -> Result<PriceHistoryResponse, Box<dyn std::error::Error>> {
        let mut req = self
            .client
            .get(format!("{CLOB_API}/prices-history"))
            .query(&[("market", markte)]);

        let res = req.send().await?.error_for_status()?;

        Ok(serde_json::from_str(res.text().await?.as_str())?)
    }
}
