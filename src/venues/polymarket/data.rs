use super::consts::DATA_API;
use crate::venues::polymarket::types::{TradesQuery, TradesResponse};

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
        let mut params: Vec<(&str, String)> = vec![
            ("limit", query.limit.to_string()),
            ("takerOnly", query.taker_only.to_string()),
        ];

        if let Some(c) = query.condition_id {
            params.push(("conditionId", c.to_string()));
        }
        if let Some(c) = query.cursor {
            params.push(("cursor", c.to_string()));
        }
        if let Some(s) = query.side {
            params.push(("side", s.to_string()));
        }
        if let Some(s) = query.start {
            params.push(("start", s.to_string()));
        }
        if let Some(e) = query.end {
            params.push(("end", e.to_string()));
        }

        let req = self
            .client
            .get(format!("{DATA_API}/v2/trades"))
            .query(&params);

        let res = req.send().await?.error_for_status()?;

        Ok(serde_json::from_str(res.text().await?.as_str())?)
    }
}
