use super::consts::USER_PNL_API;
use super::types::UserPnlResponse;

struct UserPnlApi {
    client: reqwest::Client,
}

impl UserPnlApi {
    fn new() -> Self {
        let client = reqwest::Client::new();

        Self { client }
    }

    async fn get_pnl(
        &self,
        user_address: &str,
        interval: &str,
        fidelity: &str,
    ) -> Result<UserPnlResponse, Box<dyn std::error::Error>> {
        let req = self.client.get(format!("{USER_PNL_API}/user-pnl")).query(&[
            ("user_address", user_address),
            ("interval", interval),
            ("fidelity", fidelity),
        ]);

        let res = req.send().await?.error_for_status()?;

        Ok(serde_json::from_str(res.text().await?.as_str())?)
    }
}
