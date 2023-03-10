use crate::api_data::{self, SickApiData};
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
use ts_rs::TS;
use std::time::SystemTime;

use super::pool::{history_error, redis_error};
use super::redis::get_range_params;
use super::types::{TimestampInfo, HistoryResult, ValueTypeTrait, ValuesTrait, CoinQuery};


#[derive(Clone)]
pub struct TimeSeriesInterval {
    pub interval: u64,
    pub retention: u64,
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

fn history_res<T: Serialize + ValuesTrait + TS>(
    interval: &TimeSeriesInterval,
    values: T,
) -> HistoryResult<T> {
    HistoryResult {
        timestamps: get_timestamp_info(&interval),
        values: values,
    }
}

async fn history_single<T: ValueTypeTrait + Serialize + Default + Copy + FromRedisValue + TS>(
    con: &mut ConnectionManager,
    key: &String,
    interval: &TimeSeriesInterval,
) -> HistoryResult<Vec<T>> {
    let points: Vec<T> = match get_ts_values(con, key, interval).await {
        Some(r) => r,
        None => {
            return history_error();
        }
    };

    history_res(interval, points)
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
