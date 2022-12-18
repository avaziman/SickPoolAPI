use serde::Deserialize;

#[derive(Clone)]
pub struct SickApiData {
    pub redis: redis::aio::ConnectionManager,
    pub mysql: mysql::Pool,
    pub hashrate_interval: crate::routes::history::TimeSeriesInterval,
    pub block_interval: crate::routes::history::TimeSeriesInterval
}

#[derive(Deserialize)]
pub struct CoinQuery {
    pub coin: String,
}