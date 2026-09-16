use super::consts::GAMMA_API;
struct GammaApi {}

impl GammaApi {
    async fn list_markets(limit: i32, ascending: bool) -> Result<(),()> {
        let res = reqwest::get(GAMMA_API.to_owned() +"/markets/keyset").await?;

        
        Ok(())
    }
}