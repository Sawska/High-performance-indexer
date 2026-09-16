use super::consts::POLYMARKET_API;
use super::types::TopHoldersResponse;
use std::error::Error;
struct Polymarket {
    client: reqwest::Client,
}

impl Polymarket {
    fn new() -> Self {
        let client = reqwest::Client::new();

        Self { client }
    }

    async fn top_holders(
        &self,
        after_cursor: Option<&str>,
        condition_id: &str,
    ) -> Result<TopHoldersResponse, Box<dyn Error>> {
        let mut req = self
            .client
            .get(format!("{POLYMARKET_API}/top-holders"))
            .query(&[("condition_token", condition_id.to_string())]);

        if let Some(cursor) = after_cursor {
            req = req.query(&[("afterCursor", cursor.to_string())]);
        }

        let res = req.send().await?.error_for_status()?;

        Ok(serde_json::from_str(res.text().await?.as_str())?)
    }
}
