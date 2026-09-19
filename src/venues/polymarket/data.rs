use super::consts::DATA_API;
use crate::venues::polymarket::types::{
    ActivityQuery, ActivityResponse, PositionsQuery, PositionsResponse, TopHoldersResponse,
    TradesQuery, TradesResponse, ValueResponse,
};

struct DataApi {
    client: reqwest::Client,
}

impl DataApi {
    fn new() -> Self {
        let client = reqwest::Client::new();
        Self { client }
    }

    async fn get_trades(
        &self,
        query: TradesQuery<'_>,
    ) -> Result<TradesResponse, Box<dyn std::error::Error>> {
        let req = self
            .client
            .get(format!("{DATA_API}/v2/trades"))
            .query(&query);

        let res = req.send().await?.error_for_status()?;

        Ok(serde_json::from_str(res.text().await?.as_str())?)
    }

    async fn get_positions(
        &self,
        query: PositionsQuery<'_>,
    ) -> Result<PositionsResponse, Box<dyn std::error::Error>> {
        let req = self
            .client
            .get(format!("{DATA_API}/positions"))
            .query(&query);

        let res = req.send().await?.error_for_status()?;

        Ok(serde_json::from_str(res.text().await?.as_str())?)
    }

    async fn get_activity(
        &self,
        query: ActivityQuery<'_>,
    ) -> Result<ActivityResponse, Box<dyn std::error::Error>> {
        let req = self
            .client
            .get(format!("{DATA_API}/activity"))
            .query(&query);

        let res = req.send().await?.error_for_status()?;

        Ok(serde_json::from_str(res.text().await?.as_str())?)
    }

    async fn get_value(
        &self,
        user: &str,
        market: Option<&str>,
    ) -> Result<ValueResponse, Box<dyn std::error::Error>> {
        let mut params: Vec<(&str, String)> = vec![("user", user.to_string())];

        if let Some(m) = market {
            params.push(("market", m.to_string()));
        }

        let req = self
            .client
            .get(format!("{DATA_API}/value"))
            .query(&params);

        let res = req.send().await?.error_for_status()?;

        Ok(serde_json::from_str(res.text().await?.as_str())?)
    }

    async fn get_holders(
        &self,
        market: &str,
        limit: i32,
    ) -> Result<TopHoldersResponse, Box<dyn std::error::Error>> {
        let req = self
            .client
            .get(format!("{DATA_API}/holders"))
            .query(&[("market", market.to_string()), ("limit", limit.to_string())]);

        let res = req.send().await?.error_for_status()?;

        Ok(serde_json::from_str(res.text().await?.as_str())?)
    }
}
