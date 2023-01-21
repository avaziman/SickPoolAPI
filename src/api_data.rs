use serde::Deserialize;

#[derive(Clone)]
pub struct SickApiData {
    pub redis: redis::aio::ConnectionManager,
    pub mysql: mysql::Pool,
    pub hashrate_interval: crate::routes::history::TimeSeriesInterval,
    pub block_interval: crate::routes::history::TimeSeriesInterval,
    pub min_payout: u64,
    pub fee: f64,
}

#[derive(Deserialize)]
pub struct CoinQuery {
    pub coin: String,
}