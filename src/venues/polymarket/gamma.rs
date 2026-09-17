use super::consts::GAMMA_API;
use super::types::{ListMarketsResponse, Profile, PublicSearchResponse};
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

    async fn search_profiles(
        &self,
        query: &str,
        limit_per_type: i32,
    ) -> Result<Vec<Profile>, Box<dyn std::error::Error>> {
        let req = self
            .client
            .get(format!("{GAMMA_API}/public-search"))
            .query(&[
                ("q", query.to_string()),
                ("limit_per_type", limit_per_type.to_string()),
                ("search_profiles", "true".to_string()),
            ]);

        let res = req.send().await?.error_for_status()?;

        let parsed: PublicSearchResponse = serde_json::from_str(res.text().await?.as_str())?;

        Ok(parsed.profiles)
    }
}
