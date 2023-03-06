use crate::api_data::{self, CoinQuery, SickApiData};
use crate::ffi::Prefix;
use crate::routes::redis::{get_ts_values, key_format};
use actix_web::body::{BoxBody, MessageBody};
use actix_web::http::header::{ContentType, HeaderName, HeaderValue};
use actix_web::http::StatusCode;
use actix_web::{get, web, HttpRequest, HttpResponse, HttpResponseBuilder, Responder};
use redis::aio::ConnectionManager;
use redis::FromRedisValue;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::time::SystemTime;

use super::pool::redis_error;
use super::redis::get_range_params;

#[derive(Serialize)]
pub struct TimestampInfo {
    pub start: u64,
    pub interval: u64,
    pub amount: u64,
}

pub fn get_timestamp_info(interval: &TimeSeriesInterval) -> TimestampInfo {
    let (first_timestamp, last_timestamp) = get_range_params(interval);
    TimestampInfo {
        start: first_timestamp / 1000,
        interval: interval.interval,
        amount: interval.amount,
    }
}

pub trait ValueTypeTrait {}
impl ValueTypeTrait for u64 {}
impl ValueTypeTrait for f64 {}

pub trait ValuesTrait {}
impl<ValueTypeTrait> ValuesTrait for HashMap<String, Vec<ValueTypeTrait>> {}
// single value queries
impl<ValueTypeTrait> ValuesTrait for Vec<ValueTypeTrait> {}

#[derive(Serialize)]
pub struct SickResult {
    #[serde(skip)]
    pub status_code: StatusCode,
    pub error: Value,
    pub result: Value,
}

impl Responder for SickResult {
    type Body = BoxBody;

    fn respond_to(self, req: &HttpRequest) -> HttpResponse {
        HttpResponse::build(self.status_code)
            .content_type(ContentType::json())
            .body(json!(self).to_string())
    }
}

#[derive(Serialize)]
pub struct HistoryResult<T: Serialize + ValuesTrait> {
    pub timestamps: TimestampInfo,
    pub values: T,
}

impl<T: Serialize + ValuesTrait> Responder for HistoryResult<T> {
    type Body = BoxBody;

    fn respond_to(self, req: &HttpRequest) -> HttpResponse<Self::Body> {
        let mut res = SickResult {
            status_code: StatusCode::OK,
            error: Value::Null,
            result: json!(self),
        }
        .respond_to(req);

        let curtime = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let next = self.timestamps.start + self.timestamps.amount * self.timestamps.interval + 1;
        let cache_for = next - curtime;

        res.headers_mut().insert(
            HeaderName::from_lowercase(b"cache-control").unwrap(),
            HeaderValue::from_str(&format!("max-age={}", cache_for)).unwrap(),
        );

        res
    }
}

fn history_res<T: Serialize + ValuesTrait>(
    interval: &TimeSeriesInterval,
    values: T,
) -> HistoryResult<T> {
    HistoryResult {
        timestamps: get_timestamp_info(&interval),
        values: values,
    }
}

async fn history_single<T: ValueTypeTrait + Serialize + Default + Copy + FromRedisValue>(
    con: &mut ConnectionManager,
    key: &String,
    interval: &TimeSeriesInterval,
) -> HistoryResult<Vec<T>> {
    let points: Vec<T> = match get_ts_values(con, key, interval).await {
        Some(r) => r,
        None => Vec::new(),
    };

    history_res(interval, points)
}

#[derive(Clone)]
pub struct TimeSeriesInterval {
    pub interval: u64,
    pub retention: u64,
    pub amount: u64,
}

pub fn get_time_series(interval: u64, retention: u64) -> TimeSeriesInterval {
    TimeSeriesInterval {
        interval,
        retention,
        amount: retention / interval,
    }
}

pub fn network_history_route(cfg: &mut web::ServiceConfig) {
    let arr = [
        ("hashrate", "HASHRATE:NETWORK"),
        ("difficulty", "DIFFICULTY"),
    ];

    for (path, key_name) in arr {
        cfg.route(
            &path,
            web::get().to(
                move |app_data: web::Data<SickApiData>, info: web::Query<CoinQuery>| async move {
                    history_single::<f64>(
                        &mut app_data.redis.clone(),
                        &key_format(&[&info.coin.clone(), &key_name]),
                        &app_data.hashrate_interval,
                    )
                    .await
                },
            ),
        );
    }
}

pub fn pool_history_route(cfg: &mut web::ServiceConfig) {
    let arr = [
        ("hashrate", "HASHRATE:POOL"),
        ("miner-count", "MINER_COUNT:POOL"),
        ("worker-count", "WORKER_COUNT:POOL"),
    ];

    for (path, key_name) in arr {
        cfg.route(
            &path,
            web::get().to(
                move |app_data: web::Data<SickApiData>, info: web::Query<CoinQuery>| async move {
                    history_single::<f64>(
                        &mut app_data.redis.clone(),
                        &key_format(&[&info.coin.clone(), &key_name]),
                        &app_data.hashrate_interval,
                    )
                    .await
                },
            ),
        );
    }

    let arr = [
        ("blocks-mined", "MINED_BLOCK:NUMBER:COMPACT"),
        ("round-effort", "MINED_BLOCK:NUMBER"),
        ("round-duration", "BLOCK:EFFORT_PERCENT:COMPACT"),
    ];

    for (path, key_name) in arr {
        cfg.route(
            &path,
            web::get().to(
                move |app_data: web::Data<SickApiData>, info: web::Query<CoinQuery>| async move {
                    history_single::<f64>(
                        &mut app_data.redis.clone(),
                        &key_format(&[&info.coin.clone(), &key_name]),
                        &app_data.block_interval,
                    )
                    .await
                },
            ),
        );
    }
}
