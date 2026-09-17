use super::consts::CLOB_POLYMARKET;
use super::types::{
    LastTradePriceResponse, MidpointResponse, OrderBook, PriceHistoryQuery, PriceHistoryResponse,
    PriceResponse, PricesResponse, SpreadResponse, TickSizeResponse, TokenValueMap,
};
use serde_json::json;

struct ClobApi {
    client: reqwest::Client,
}

impl ClobApi {
    fn new() -> Self {
        let client = reqwest::Client::new();

        Self { client }
    }
    
    async fn get_book(&self, token_id: &str) -> Result<OrderBook, Box<dyn std::error::Error>> {
        let req = self
            .client
            .get(format!("{CLOB_POLYMARKET}/book"))
            .query(&[("token_id", token_id)]);

        let res = req.send().await?.error_for_status()?;

        Ok(serde_json::from_str(res.text().await?.as_str())?)
    }

    async fn get_books(
        &self,
        token_ids: &[&str],
    ) -> Result<Vec<OrderBook>, Box<dyn std::error::Error>> {
        let body: Vec<_> = token_ids.iter().map(|t| json!({ "token_id": t })).collect();

        let req = self
            .client
            .post(format!("{CLOB_POLYMARKET}/books"))
            .json(&body);

        let res = req.send().await?.error_for_status()?;

        Ok(serde_json::from_str(res.text().await?.as_str())?)
    }

    async fn get_price(
        &self,
        token_id: &str,
        side: &str,
    ) -> Result<PriceResponse, Box<dyn std::error::Error>> {
        let req = self
            .client
            .get(format!("{CLOB_POLYMARKET}/price"))
            .query(&[("token_id", token_id), ("side", side)]);

        let res = req.send().await?.error_for_status()?;

        Ok(serde_json::from_str(res.text().await?.as_str())?)
    }

    async fn get_prices(
        &self,
        params: &[(&str, &str)],
    ) -> Result<PricesResponse, Box<dyn std::error::Error>> {
        let body: Vec<_> = params
            .iter()
            .map(|(t, s)| json!({ "token_id": t, "side": s }))
            .collect();

        let req = self
            .client
            .post(format!("{CLOB_POLYMARKET}/prices"))
            .json(&body);

        let res = req.send().await?.error_for_status()?;

        Ok(serde_json::from_str(res.text().await?.as_str())?)
    }

    async fn get_midpoint(
        &self,
        token_id: &str,
    ) -> Result<MidpointResponse, Box<dyn std::error::Error>> {
        let req = self
            .client
            .get(format!("{CLOB_POLYMARKET}/midpoint"))
            .query(&[("token_id", token_id)]);

        let res = req.send().await?.error_for_status()?;

        Ok(serde_json::from_str(res.text().await?.as_str())?)
    }

    async fn get_midpoints(
        &self,
        token_ids: &[&str],
    ) -> Result<TokenValueMap, Box<dyn std::error::Error>> {
        let body: Vec<_> = token_ids.iter().map(|t| json!({ "token_id": t })).collect();

        let req = self
            .client
            .post(format!("{CLOB_POLYMARKET}/midpoints"))
            .json(&body);

        let res = req.send().await?.error_for_status()?;

        Ok(serde_json::from_str(res.text().await?.as_str())?)
    }

    async fn get_spread(
        &self,
        token_id: &str,
    ) -> Result<SpreadResponse, Box<dyn std::error::Error>> {
        let req = self
            .client
            .get(format!("{CLOB_POLYMARKET}/spread"))
            .query(&[("token_id", token_id)]);

        let res = req.send().await?.error_for_status()?;

        Ok(serde_json::from_str(res.text().await?.as_str())?)
    }

    async fn get_spreads(
        &self,
        token_ids: &[&str],
    ) -> Result<TokenValueMap, Box<dyn std::error::Error>> {
        let body: Vec<_> = token_ids.iter().map(|t| json!({ "token_id": t })).collect();

        let req = self
            .client
            .post(format!("{CLOB_POLYMARKET}/spreads"))
            .json(&body);

        let res = req.send().await?.error_for_status()?;

        Ok(serde_json::from_str(res.text().await?.as_str())?)
    }

    async fn get_last_trade_price(
        &self,
        token_id: &str,
    ) -> Result<LastTradePriceResponse, Box<dyn std::error::Error>> {
        let req = self
            .client
            .get(format!("{CLOB_POLYMARKET}/last-trade-price"))
            .query(&[("token_id", token_id)]);

        let res = req.send().await?.error_for_status()?;

        Ok(serde_json::from_str(res.text().await?.as_str())?)
    }

    async fn get_tick_size(
        &self,
        token_id: &str,
    ) -> Result<TickSizeResponse, Box<dyn std::error::Error>> {
        let req = self
            .client
            .get(format!("{CLOB_POLYMARKET}/tick-size"))
            .query(&[("token_id", token_id)]);

        let res = req.send().await?.error_for_status()?;

        Ok(serde_json::from_str(res.text().await?.as_str())?)
    }

    async fn get_history(
        &self,
        query: PriceHistoryQuery<'_>,
    ) -> Result<PriceHistoryResponse, Box<dyn std::error::Error>> {
        let mut params: Vec<(&str, String)> = vec![("market", query.market.to_string())];

        if let Some(i) = query.interval {
            params.push(("interval", i.to_string()));
        }
        if let Some(s) = query.start_ts {
            params.push(("startTs", s.to_string()));
        }
        if let Some(e) = query.end_ts {
            params.push(("endTs", e.to_string()));
        }
        if let Some(f) = query.fidelity {
            params.push(("fidelity", f.to_string()));
        }

        let req = self
            .client
            .get(format!("{CLOB_POLYMARKET}/prices-history"))
            .query(&params);

        let res = req.send().await?.error_for_status()?;

        Ok(serde_json::from_str(res.text().await?.as_str())?)
    }
}
