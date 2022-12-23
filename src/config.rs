use serde::Deserialize;
use crate::redis_interop::ffi::StatsConfig;

#[derive(Deserialize)]
pub struct MySqlConfig {
    pub host: String,
    pub db_name: String,
    pub user: String,
    pub pass: String,
}

#[derive(Deserialize)]
pub struct RedisConfig
{
    pub host: String,
    pub db_index: u8,
    pub hashrate_ttl_seconds: u32
}

#[derive(Deserialize)]
pub struct CoinConfig {
    pub redis: RedisConfig,
    pub mysql: MySqlConfig,
    pub stats: StatsConfig
}