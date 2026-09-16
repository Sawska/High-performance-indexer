use super::consts::GAMMA_API;
use super::types::ListMarketsResponse;
struct GammaApi {
    client: reqwest::Client,
}

impl GammaApi {
    fn new() -> Self {
        let client = reqwest::Client::new();
        Self { client }
    }
    async fn list_markets(
        &self,
        limit: i32,
        after_cursor: Option<&str>,
    ) -> Result<ListMarketsResponse, Box<dyn std::error::Error>> {
        let mut req = self
            .client
            .get(format!("{GAMMA_API}/markets/keyset"))
            .query(&[("limit", limit.to_string())]);

        if let Some(c) = after_cursor {
            req = req.query(&[("after_cursor", c)]);
        }

        let res = req.send().await?.error_for_status()?;

        Ok(serde_json::from_str(res.text().await?.as_str())?)
    }
}
