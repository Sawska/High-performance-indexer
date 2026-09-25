use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct ListMarketsResponse {
    pub markets: Vec<ListMarkets>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListMarkets {
    pub id: String,
    pub question: Option<String>,
    pub condition_id: String,
    pub slug: Option<String>,
    pub twitter_card_image: Option<String>,
    pub resolution_source: Option<String>,
    pub end_date: Option<String>,
    pub category: Option<String>,
    pub liquidity: Option<String>,
    pub sponsor_name: Option<String>,
    pub sponsor_image: Option<String>,
    pub start_date: Option<String>,
    pub x_axis_value: Option<String>,
    pub denomination_token: Option<String>,
    pub fee: Option<String>,
    pub image: Option<String>,
    pub icon: Option<String>,
    pub lower_bound: Option<String>,
    pub upper_bound: Option<String>,
    pub description: Option<String>,
    pub outcomes: Option<String>,
    pub outcome_prices: Option<String>,
    pub volume: Option<String>,
    pub active: Option<bool>,
    pub market_type: Option<String>,
    pub format_type: Option<String>,
    pub lower_bound_date: Option<String>,
    pub upper_bound_date: Option<String>,
    pub closed: Option<bool>,
    pub created_by: Option<i64>,
    pub updated_by: Option<i64>,
    pub closed_time: Option<String>,
    pub wide_format: Option<bool>,
    pub new: Option<bool>,
    pub mailchimp_tag: Option<String>,
    pub featured: Option<bool>,
    pub archived: Option<bool>,
    pub resolved_by: Option<String>,
    pub restricted: Option<bool>,
    pub market_group: Option<i64>,
    pub group_item_title: Option<String>,
    pub group_item_threshold: Option<String>,
    #[serde(rename = "questionID")]
    pub question_id: Option<String>,
    pub uma_end_date: Option<String>,
    pub enable_order_book: Option<bool>,
    pub order_price_min_tick_size: Option<f64>,
    pub order_min_size: Option<f64>,
    pub uma_resolution_status: Option<String>,
    pub curation_order: Option<i64>,
    pub volume_num: Option<f64>,
    pub liquidity_num: Option<f64>,
    pub end_date_iso: Option<String>,
    pub start_date_iso: Option<String>,
    pub uma_end_date_iso: Option<String>,
    pub has_reviewed_dates: Option<bool>,
    pub ready_for_cron: Option<bool>,
    pub comments_enabled: Option<bool>,
    pub volume_24hr: Option<f64>,
    pub volume_1wk: Option<f64>,
    pub volume_1mo: Option<f64>,
    pub volume_1yr: Option<f64>,
    pub game_start_time: Option<String>,
    pub seconds_delay: Option<i64>,
    pub clob_token_ids: Option<String>,
    pub disqus_thread: Option<String>,
    pub short_outcomes: Option<String>,
    #[serde(rename = "teamAID")]
    pub team_a_id: Option<String>,
    #[serde(rename = "teamBID")]
    pub team_b_id: Option<String>,
    pub uma_bond: Option<String>,
    pub volume_24hr_clob: Option<f64>,
    pub volume_1wk_clob: Option<f64>,
    pub volume_1mo_clob: Option<f64>,
    pub volume_1yr_clob: Option<f64>,
    pub volume_clob: Option<f64>,
    pub liquidity_clob: Option<f64>,
    pub maker_base_fee: Option<f64>,
    pub taker_base_fee: Option<f64>,
    pub custom_liveness: Option<i64>,
    pub accepting_orders: Option<bool>,
    pub notifications_enabled: Option<bool>,
    pub score: Option<i64>,
    pub image_optimized: Option<ImageOptimized>,
    pub icon_optimized: Option<IconOptimized>,
    pub categories: Option<Categories>,
    pub tags: Option<Tags>,
    pub creator: Option<String>,
    pub ready: Option<bool>,
    pub funded: Option<bool>,
    pub past_slug: Option<String>,
    pub funded_timestamp: Option<String>,
    pub accepting_orders_timetstamp: Option<String>,
    pub competitive: Option<f64>,
    pub rewards_min_size: Option<f64>,
    pub rewards_max_spread: Option<f64>,
    pub spread: Option<f64>,
    pub automatically_resloved: Option<bool>,
    pub one_day_price_change: Option<f64>,
    pub one_hour_price_change: Option<f64>,
    pub one_week_price_change: Option<f64>,
    pub one_month_price_change: Option<f64>,
    pub one_year_price_change: Option<f64>,
    pub last_trade_price: Option<f64>,
    pub best_bid: Option<f64>,
    pub best_ask: Option<f64>,
    pub automatically_active: Option<bool>,
    pub clear_book_on_start: Option<bool>,
    pub chart_color: Option<String>,
    pub series_color: Option<String>,
    pub show_gmp_series: Option<bool>,
    pub show_gm_outcome: Option<bool>,
    pub manual_activation: Option<bool>,
    pub neg_risk_other: Option<bool>,
    pub game_id: Option<String>,
    pub group_item_range: Option<String>,
    pub sports_market_type: Option<String>,
    pub line: Option<f64>,
    pub uma_resolution_statuses: Option<String>,
    pub pending_deployment: Option<bool>,
    pub deploying: Option<bool>,
    pub deploying_timestamp: Option<String>,
    pub scheluded_deployment_timestamp: Option<String>,
    pub rfq_enabled: Option<bool>,
    pub event_start_time: Option<String>,
    pub fees_enabled: Option<bool>,
    pub fee_schedule: Option<FeeSchedule>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeeSchedule {
    pub exponent: Option<f64>,
    pub rate: Option<f64>,
    pub taker_only: Option<bool>,
    pub rebate_rate: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageOptimized {
    pub id: String,
    pub image_url_source: Option<String>,
    pub image_url_optimized: Option<String>,
    pub image_size_kb_source: Option<f64>,
    pub image_size_kb_optimized: Option<f64>,
    pub image_optimized_last_updated: Option<String>,
    #[serde(rename = "relID")]
    pub rel_id: Option<i64>,
    pub field: Option<String>,
    pub relname: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IconOptimized {
    pub id: String,
    pub image_url_source: Option<String>,
    pub image_url_optimized: Option<String>,
    pub image_size_kb_source: Option<f64>,
    pub image_size_kb_optimized: Option<f64>,
    pub image_optimized_complete: Option<bool>,
    pub image_optimized_last_updated: Option<String>,
    #[serde(rename = "relID")]
    pub rel_id: Option<String>,
    pub field: Option<String>,
    pub relname: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub id: String,
    pub ticker: Option<String>,
    pub slug: Option<String>,
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub description: Option<String>,
    pub resolution_source: Option<String>,
    pub start_date: Option<String>,
    pub creation_date: Option<String>,
    pub end_date: Option<String>,
    pub image: Option<String>,
    pub icon: Option<String>,
    pub active: Option<bool>,
    pub closed: Option<bool>,
    pub archived: Option<bool>,
    pub new: Option<bool>,
    pub featured: Option<bool>,
    pub restricted: Option<bool>,
    pub liquidity: Option<f64>,
    pub volume: Option<f64>,
    pub open_interest: Option<f64>,
    pub sort_by: Option<String>,
    pub category: Option<String>,
    pub subcategory: Option<String>,
    pub is_template: Option<bool>,
    pub template_variables: Option<String>,
    pub published_at: Option<String>,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
    pub updated_at: Option<String>,
    pub comments_enabled: Option<bool>,
    pub competitive: Option<f64>,
    pub volume_24hr: Option<f64>,
    pub volume_1wk: Option<f64>,
    pub volume_1mo: Option<f64>,
    pub volume_1yr: Option<f64>,
    pub featured_image: Option<String>,
    pub disqus_thread: Option<String>,
    pub parent_event_id: Option<i64>,
    pub enable_order_book: Option<bool>,
    pub liquidity_clob: Option<f64>,
    pub neg_risk: Option<bool>,
    #[serde(rename = "negRiskMarketID")]
    pub neg_risk_market_id: Option<String>,
    pub neg_risk_fee_bips: Option<i64>,
    pub comment_count: Option<i64>,
    pub image_optimized: ImageOptimized,
    pub icon_optimized: IconOptimized,
    pub featured_image_optimized: Option<ImageOptimized>,
    pub sub_events: Option<Vec<String>>,
    pub markets: Option<Vec<ListMarkets>>,
    pub series: Series,
    pub collections: Collections,
    pub tags: Vec<Tags>,
    pub cyom: Option<bool>,
    pub closed_time: Option<String>,
    pub show_all_outcomes: Option<bool>,
    pub show_market_images: Option<bool>,
    pub automatically_resolved: Option<bool>,
    pub enable_neg_risk: Option<bool>,
    pub event_date: Option<String>,
    pub start_time: Option<String>,
    pub event_week: Option<i64>,
    pub series_slug: Option<String>,
    pub score: Option<String>,
    pub elapsed: Option<String>,
    pub period: Option<String>,
    pub live: Option<bool>,
    pub ended: Option<bool>,
    pub finished_timestamp: Option<String>,
    pub gmp_char_mode: Option<String>,
    pub event_creators: EventCreators,
    pub tweet_count: Option<i64>,
    pub chats: Chats,
    pub featured_order: Option<i64>,
    pub cant_estimate: Option<bool>,
    pub estimated_value: Option<String>,
    pub templates: Templates,
    pub spreads_main_line: Option<f64>,
    pub totals_main_line: Option<f64>,
    pub carousel_map: Option<String>,
    pub pending_deployment: Option<bool>,
    pub deploying: Option<bool>,
    pub deploying_time_stamp: Option<String>,
    pub scheduled_deployment_time_stamp: Option<String>,
    pub game_status: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Templates {
    pub id: Option<String>,
    pub event_title: Option<String>,
    pub event_image: Option<String>,
    pub market_title: Option<String>,
    pub description: Option<String>,
    pub resolution_source: Option<String>,
    pub neg_risk: Option<bool>,
    pub sort_by: Option<String>,
    pub show_market_image: Option<bool>,
    pub series_slug: Option<String>,
    pub outcomes: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Chats {
    pub id: Option<String>,
    pub channel: Option<String>,
    pub channel_image: Option<String>,
    pub live: Option<bool>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventCreators {
    pub id: Option<String>,
    pub creator_name: Option<String>,
    pub creator_handle: Option<String>,
    pub creator_url: Option<String>,
    pub creator_image: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Series {
    pub id: String,
    pub ticker: Option<String>,
    pub slug: Option<String>,
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub series_type: Option<String>,
    pub description: Option<String>,
    pub image: Option<String>,
    pub icon: Option<String>,
    pub layout: Option<String>,
    pub active: Option<bool>,
    pub closed: Option<bool>,
    pub archived: Option<bool>,
    pub new: Option<bool>,
    pub featured: Option<bool>,
    pub restricted: Option<bool>,
    pub is_template: Option<bool>,
    pub template_variables: Option<bool>,
    pub published_at: Option<String>,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub competitive: Option<f64>,
    pub volume_24hr: Option<f64>,
    pub volume: Option<f64>,
    pub liquidity: Option<f64>,
    pub start_date: Option<String>,
    #[serde(rename = "pythTokenID")]
    pub pyth_token_id: Option<String>,
    pub cg_asset_name: Option<String>,
    pub score: Option<i64>,
    pub events: Vec<Event>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Collections {
    pub id: String,
    pub ticker: Option<String>,
    pub slug: Option<String>,
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub collection_type: Option<String>,
    pub description: Option<String>,
    pub image: Option<String>,
    pub icon: Option<String>,
    pub header_image: Option<String>,
    pub layout: Option<String>,
    pub active: Option<bool>,
    pub closed: Option<bool>,
    pub archived: Option<bool>,
    pub new: Option<bool>,
    pub featured: Option<bool>,
    pub restricted: Option<bool>,
    pub is_template: Option<bool>,
    pub template_variables: Option<String>,
    pub published_at: Option<String>,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
    pub updated_at: Option<String>,
    pub comments_enabled: Option<bool>,
    pub image_optimized: ImageOptimized,
    pub icon_optimized: IconOptimized,
    pub header_image_optimized: HeaderImageOptimized,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tags {
    pub id: Option<String>,
    pub labbel: Option<String>,
    pub slug: Option<String>,
    pub force_show: Option<bool>,
    pub published_at: Option<String>,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub force_hide: Option<bool>,
    pub is_carousel: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Categories {
    pub id: String,
    pub label: Option<String>,
    pub parent_category: Option<String>,
    pub slug: Option<String>,
    pub published_at: Option<String>,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeaderImageOptimized {
    pub id: String,
    pub image_url_source: Option<String>,
    pub image_url_optimized: Option<String>,
    pub image_size_kb_source: Option<f64>,
    pub image_size_kb_optimized: Option<f64>,
    pub image_optimized_complete: Option<bool>,
    pub image_optimized_last_updated: Option<String>,
    #[serde(rename = "relID")]
    pub rel_id: Option<String>,
    pub field: Option<String>,
    pub relname: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeaturedImageOptimized {
    pub id: String,
    pub image_url_source: Option<String>,
    pub image_url_optimized: Option<String>,
    pub image_size_kb_source: Option<f64>,
    pub image_size_kb_optimized: Option<f64>,
    pub image_optimized_complete: Option<bool>,
    pub image_optimized_last_updated: Option<String>,
    #[serde(rename = "relID")]
    pub rel_id: Option<String>,
    pub field: Option<String>,
    pub relname: Option<String>,
}

pub type TopHoldersResponse = Vec<MarketHolders>;

#[derive(Debug, Clone, Deserialize)]
pub struct MarketHolders {
    pub token: String,
    pub holders: Vec<Holder>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Holder {
    pub proxy_wallet: String,
    pub asset: String,
    pub amount: f64,
    pub outcome_index: Option<i64>,
    pub name: Option<String>,
    pub pseudonym: Option<String>,
    pub bio: Option<String>,
    pub display_username_public: Option<bool>,
    pub profile_image: Option<String>,
    pub profile_image_optimized: Option<String>,
    pub verified: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TradesResponse {
    pub data: Vec<Trade>,
    pub pagination: Pagination,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Pagination {
    pub limit: i32,
    pub offset: i32,
    pub has_more: bool,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Trade {
    pub proxy_wallet: String,
    pub condition_id: String,
    pub token_id: String,
    pub side: String,
    pub size: f64,
    pub price: f64,
    pub timestamp: i64,
    pub transaction_hash: String,
    pub outcome: Option<String>,
    pub outcome_index: Option<i64>,
    pub title: Option<String>,
    pub slug: Option<String>,
    pub event_slug: Option<String>,
    pub icon: Option<String>,
    pub name: Option<String>,
    pub pseudonym: Option<String>,
    pub bio: Option<String>,
    pub profile_image: Option<String>,
    pub profile_image_optimized: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PriceHistoryResponse {
    pub history: Vec<History>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct History {
    pub t: u32,
    pub p: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OrderBook {
    pub market: String,
    pub asset_id: String,
    pub timestamp: String,
    pub hash: String,
    pub bids: Vec<OrderLevel>,
    pub asks: Vec<OrderLevel>,
    pub min_order_size: Option<String>,
    pub tick_size: Option<String>,
    pub neg_risk: Option<bool>,
    pub last_trade_price: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OrderLevel {
    pub price: String,
    pub size: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PriceResponse {
    pub price: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MidpointResponse {
    pub mid: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SpreadResponse {
    pub spread: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LastTradePriceResponse {
    pub price: String,
    pub side: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TickSizeResponse {
    pub minimum_tick_size: f64,
}

pub type PricesResponse = std::collections::HashMap<String, std::collections::HashMap<String, String>>;

pub type TokenValueMap = std::collections::HashMap<String, String>;

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct PriceHistoryQuery<'a> {
    pub market: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_ts: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_ts: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fidelity: Option<i32>,
}

impl<'a> PriceHistoryQuery<'a> {
    pub fn new(market: &'a str) -> Self {
        Self {
            market,
            interval: None,
            start_ts: None,
            end_ts: None,
            fidelity: None,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct TradesQuery<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition_id: Option<&'a str>,
    pub limit: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub side: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<i64>,
    pub taker_only: bool,
}

impl Default for TradesQuery<'_> {
    fn default() -> Self {
        Self {
            condition_id: None,
            limit: 100,
            cursor: None,
            side: None,
            start: None,
            end: None,
            taker_only: true,
        }
    }
}

pub type PositionsResponse = Vec<Position>;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    pub proxy_wallet: String,
    pub asset: String,
    pub condition_id: String,
    pub size: f64,
    pub avg_price: f64,
    pub initial_value: f64,
    pub current_value: f64,
    pub cash_pnl: f64,
    pub percent_pnl: f64,
    pub total_bought: f64,
    pub realized_pnl: f64,
    pub percent_realized_pnl: f64,
    pub cur_price: f64,
    pub redeemable: bool,
    pub mergeable: bool,
    pub title: Option<String>,
    pub slug: Option<String>,
    pub icon: Option<String>,
    pub event_slug: Option<String>,
    pub outcome: Option<String>,
    pub outcome_index: Option<i64>,
    pub opposite_outcome: Option<String>,
    pub opposite_asset: Option<String>,
    pub end_date: Option<String>,
    pub negative_risk: Option<bool>,
}

pub type ActivityResponse = Vec<Activity>;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    pub proxy_wallet: String,
    pub timestamp: i64,
    pub condition_id: String,
    #[serde(rename = "type")]
    pub activity_type: String,
    pub size: f64,
    pub usdc_size: f64,
    pub transaction_hash: String,
    pub price: f64,
    pub asset: String,
    pub side: Option<String>,
    pub outcome_index: Option<i64>,
    pub title: Option<String>,
    pub slug: Option<String>,
    pub icon: Option<String>,
    pub event_slug: Option<String>,
    pub outcome: Option<String>,
    pub name: Option<String>,
    pub pseudonym: Option<String>,
    pub bio: Option<String>,
    pub profile_image: Option<String>,
    pub profile_image_optimized: Option<String>,
}

pub type ValueResponse = Vec<UserValue>;

#[derive(Debug, Clone, Deserialize)]
pub struct UserValue {
    pub user: String,
    pub value: f64,
}

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct PositionsQuery<'a> {
    pub user: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market: Option<&'a str>,
    pub limit: i32,
    pub offset: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_by: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_direction: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redeemable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mergeable: Option<bool>,
}

impl<'a> PositionsQuery<'a> {
    pub fn new(user: &'a str) -> Self {
        Self {
            user,
            market: None,
            limit: 100,
            offset: 0,
            sort_by: None,
            sort_direction: None,
            redeemable: None,
            mergeable: None,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct ActivityQuery<'a> {
    pub user: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "type")]
    pub activity_type: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub side: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<i64>,
    pub limit: i32,
    pub offset: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_by: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_direction: Option<&'a str>,
}

impl<'a> ActivityQuery<'a> {
    pub fn new(user: &'a str) -> Self {
        Self {
            user,
            market: None,
            activity_type: None,
            side: None,
            start: None,
            end: None,
            limit: 100,
            offset: 0,
            sort_by: None,
            sort_direction: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct PublicSearchResponse {
    #[serde(default)]
    pub profiles: Vec<Profile>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub proxy_wallet: String,
    pub name: Option<String>,
    pub pseudonym: Option<String>,
    pub bio: Option<String>,
    pub display_username_public: Option<bool>,
    pub profile_image: Option<String>,
    pub profile_image_optimized: Option<String>,
    pub verified: Option<bool>,
}

pub type UserPnlResponse = Vec<UserPnlPoint>;

#[derive(Debug, Clone, Deserialize)]
pub struct UserPnlPoint {
    pub t: i64,
    pub p: f64,
}

#[derive(Debug,Clone,Deserialize)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum MarketEvent {
    Book(OrderBook),
    PriceChange(PriceChangeEvent),
    LastTradePrice(LastTradePriceEvent),
    TickSizeChange(TickSizeChangeEvent),
    #[serde(other)]
    Unknow,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PriceChangeEvent {
    pub market: String,
    pub price_changes: Vec<PriceChange>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PriceChange {
    pub asset_id: String,
    pub price: String,
    pub size: String,
    pub side: String,
    pub hash: String,
    pub best_ask: String,
}


#[derive(Debug,Clone,Deserialize)]
pub struct LastTradePriceEvent {
    pub market: String,
    pub asset_id: String,
    pub price: String,
    pub size: Option<String>,
    pub side: String,
    pub timestamp: Option<String>,
    pub transaction_hash: Option<String>,
}

#[derive(Debug,Clone,Deserialize)]
pub struct TickSizeChangeEvent {
    pub market: String,
    pub asset_id: String,
    pub old_tick_size: Option<String>,
    pub new_tick_size: String,
    pub timestamp: Option<String>,
}