use crate::api_data::{self, CoinQuery, SickApiData};
use crate::ffi::Prefix;
use crate::routes::redis::{get_ts_points, key_format};
use actix_web::{get, web, HttpRequest, HttpResponse, Responder};
use redis::aio::ConnectionManager;
use serde_json::{json, Value};
use std::collections::HashSet;

use super::pool::redis_error;

async fn history(
    con: &mut ConnectionManager,
    key: &String,
    interval: &TimeSeriesInterval
) -> HttpResponse {
    let points = match get_ts_points(con, key, interval).await {
        Some(r)=> r,
        None => {return redis_error();}
    };

    HttpResponse::Ok().body(
        json!({
            "error": Value::Null,
            "result": points
        })
        .to_string(),
    )
}

fn get_key_name(coin: &String, prefix: &String, pretty_name: &String) -> String {
    key_format(&[
        coin,
        &prefix,
        &pretty_name.replace('-', ":").to_ascii_uppercase(),
    ])
}

// async fn history_route(con: &mut ConnectionManager, prefix: &String, coin: &String, pretty_name: &String) -> HttpResponse{
//     let key = get_key_name(coin, prefix, pretty_name);

// }

// static NETWORK_HASHRATE: String =
//     key_format(&[&Prefix::HASHRATE.to_string(), &Prefix::NETWORK.to_string()]);

#[derive(Clone)]
pub struct TimeSeriesInterval {
    pub interval: u64,
    pub retention: u64,
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
                    history(
                        &mut app_data.redis.clone(),
                        &key_format(&[&info.coin.clone(), &key_name]),
                        &app_data.hashrate_interval
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
                    history(
                        &mut app_data.redis.clone(),
                        &key_format(&[&info.coin.clone(), &key_name]),
                        &app_data.hashrate_interval
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
                    history(
                        &mut app_data.redis.clone(),
                        &key_format(&[&info.coin.clone(), &key_name]),
                        &app_data.block_interval
                    )
                    .await
                },
            ),
        );
    }
}
