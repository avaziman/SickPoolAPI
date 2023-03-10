use crate::routes::history::get_timestamp_info;
use crate::routes::pool::history_error;
use crate::routes::pool::redis_error;
use crate::routes::redis::get_range_params;
use crate::routes::types::HistoryResult;
use crate::routes::types::MinerStats;
use crate::routes::types::SickResult;
use crate::SickApiData;
use crate::routes::types::TableRes;
use crate::routes::types::WorkerStatsEntry;

use super::redis::get_range_query;
use super::solver::OverviewQuery;
use super::types::ValuesTrait;
use crate::ffi::Prefix;
use crate::routes::redis::fill_gaps;
use crate::routes::redis::key_format;
use crate::solver::miner_alias_filter;
use actix_web::http::StatusCode;
use actix_web::{get, web, HttpRequest, HttpResponse, Responder};
use redis::aio::ConnectionManager;
use redis_ts::TsRangeQuery;
use redis_ts::{AsyncTsCommands, TsAggregationType, TsFilterOptions, TsMget, TsMrange, TsRange};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub fn miner_route(cfg: &mut web::ServiceConfig) {
    cfg.service(stats_history);
    // cfg.service(worker_history);
    cfg.service(workers);
}

// struct HashrateEntry {
//     average_hashrate: f64,
//     current_hashrate: f64,
//     invalid_shares: u64,
//     stale_shares: u64,
//     time: u64,
//     valid_shares: u64,
// }

#[derive(Serialize, Clone)]
struct WorkerTsEntry {
    time: u64,
    workers: u32,
}

#[get("/statsHistory")]
async fn stats_history(
    info: web::Query<OverviewQuery>,
    api_data: web::Data<SickApiData>,
) -> HistoryResult<MinerStats> {
    let mut con = api_data.redis.clone();

    let ts_type = format!(
        "({},{},{},{},{})",
        &Prefix::HASHRATE.to_string(),
        &key_format(&[&Prefix::HASHRATE.to_string(), &Prefix::AVERAGE.to_string()]),
        &key_format(&[&Prefix::SHARES.to_string(), &Prefix::VALID.to_string()]),
        &key_format(&[&Prefix::SHARES.to_string(), &Prefix::STALE.to_string()]),
        &key_format(&[&Prefix::SHARES.to_string(), &Prefix::INVALID.to_string()])
    );

    let filter: TsFilterOptions = miner_alias_filter(&info.address.to_lowercase())
        .equals("prefix", Prefix::MINER.to_string())
        .equals("type", ts_type);

    let (first_timestamp, last_timestamp) = get_range_params(&api_data.hashrate_interval);

    let tms: TsMrange<u64, f64> = match con
        .ts_mrange(
            get_range_query(&api_data.hashrate_interval, first_timestamp, last_timestamp),
            filter,
        )
        .await
    {
        Ok(res) => res,
        Err(err) => {
            return history_error();
        }
    };

    if tms.values.len() != 5 {
        return history_error();
    }

    let mut results: [Vec<f64>; 5] = Default::default();

    for (i, el) in tms.values.into_iter().enumerate() {
        results[i] = fill_gaps(
            &el.values,
            first_timestamp,
            api_data.hashrate_interval.interval,
            api_data.hashrate_interval.amount,
        );
    }
    let mut into_iter = results.into_iter();

    let values = MinerStats {
        average_hashrate: into_iter.next().unwrap(),
        current_hashrate: into_iter.next().unwrap(),
        invalid_shares: into_iter
            .next()
            .unwrap()
            .iter()
            .map(|x| *x as u32)
            .collect(),
        stale_shares: into_iter
            .next()
            .unwrap()
            .iter()
            .map(|x| *x as u32)
            .collect(),
        valid_shares: into_iter
            .next()
            .unwrap()
            .iter()
            .map(|x| *x as u32)
            .collect(),
    };

    HistoryResult {
        timestamps: get_timestamp_info(&api_data.hashrate_interval),
        values,
    }
}

// #[get("/workerHistory")]
// async fn worker_history(
//     req: HttpRequest,
//     info: web::Query<OverviewQuery>,
//     api_data: web::Data<SickApiData>,
// ) -> SickResult {
//     let mut con = api_data.redis.clone();

//     let ts_type = "worker-count";

//     let filter: TsFilterOptions = miner_alias_filter(&info.address).equals("type", ts_type);

//     let tms: TsMrange<u64, f64> = match con.ts_mrange(TsRangeQuery::default(), filter).await {
//         Ok(res) => res,
//         Err(err) => {
//             eprintln!("range query error: {}", err);
//             return redis_error();
//         }
//     };

//     let ts = match tms.values.first() {
//         Some(res) => res,
//         None => {
//             return redis_error();
//         }
//     };

//     let mut res_vec: Vec<WorkerTsEntry> = Vec::new();
//     res_vec.reserve(ts.values.len());

//     // timeserieses are sorted by alphabetical order
//     for (i, el) in ts.values.iter().enumerate() {
//         res_vec.push(WorkerTsEntry {
//             time: el.0,
//             workers: el.1 as u32,
//         });
//     }

//     SickResult {
//         status_code: StatusCode::OK,
//         error: None,
//         result: Some(res_vec),
//     }
// }

#[get("/workers")]
async fn workers(
    req: HttpRequest,
    info: web::Query<OverviewQuery>,
    api_data: web::Data<SickApiData>,
) -> SickResult<TableRes<WorkerStatsEntry>> {
    let mut con = api_data.redis.clone();

    let ts_type = format!(
        "({},{},{},{},{})",
        &Prefix::HASHRATE.to_string(),
        &key_format(&[&Prefix::HASHRATE.to_string(), &Prefix::AVERAGE.to_string()]),
        &key_format(&[&Prefix::SHARES.to_string(), &Prefix::VALID.to_string()]),
        &key_format(&[&Prefix::SHARES.to_string(), &Prefix::STALE.to_string()]),
        &key_format(&[&Prefix::SHARES.to_string(), &Prefix::INVALID.to_string()])
    );

    // TODO: make selected labels support
    let filter: TsFilterOptions;
    filter = miner_alias_filter(&info.address)
        .equals("prefix", Prefix::WORKER.to_string())
        .equals("type", ts_type)
        .with_labels(true);

    let tms: TsMget<u64, f64> = match con.ts_mget(filter).await {
        Ok(res) => res,
        Err(err) => {
            eprintln!("tsget query error: {}", err);
            return redis_error();
        }
    };
    // println!("{:?}", tms);
    let worker_count = tms.values.len() / 5;
    let mut res_vec: Vec<WorkerStatsEntry> = Vec::new();
    res_vec.reserve(worker_count);

    // timeserieses are sorted by alphabetical order
    for i in 0..worker_count {
        let worker_name_label: &(String, String) = &tms.values[i * 5].labels[4];

        let entry = match tms.values[i].value {
            // the worker hasn't gotten any statistics yet
            None => WorkerStatsEntry {
                worker: worker_name_label.1.clone(),
                average_hashrate: 0.0,
                current_hashrate: 0.0,
                invalid_shares: 0,
                stale_shares: 0,
                time: 0,
                valid_shares: 0,
            },
            Some(e) => WorkerStatsEntry {
                worker: worker_name_label.1.clone(),
                average_hashrate: 0.0,
                current_hashrate: 0.0,
                invalid_shares: 0,
                stale_shares: 0,
                time: 0,
                valid_shares: 0,
            },
        };

        // if (tms.values[i].value.unwrap().1 < ){

        // }

        res_vec.push(entry);
    }

    SickResult {
        status_code: StatusCode::OK,
        error: None,
        result: Some(TableRes {
            total: res_vec.len(),
            entries: res_vec,
        }),
    }
}

// #[get("/shareHistory")]
// async fn share_history(
//     req: HttpRequest,
//     info: web::Query<OverviewQuery>,
//     api_data: web::Data<SickApiData>,
// ) -> impl Responder {
//     let mut con = api_data.redis.clone();

//     let mut valid_shares_key = String::from("shares:valid:miner:");
//     valid_shares_key.push_str(&info.address);

//     let mut stale_shares_key = String::from("shares:stale:miner:");
//     stale_shares_key.push_str(&info.address);

//     let mut invalid_shares_key = String::from("shares:invalid:miner:");
//     invalid_shares_key.push_str(&info.address);

//     let mut timeserieses: Vec<TsRange<u64, u32>> = Vec::new();

//     for key in [valid_shares_key, stale_shares_key, invalid_shares_key] {
//         timeserieses.push(match con.ts_range(key, 0, "+", None::<usize>, None).await {
//             Ok(res) => res,
//             Err(err) => {
//                 eprintln!("range query error: {}", err);
//                 return HttpResponse::NotFound().body(
//                     json!({
//                         "error": "Key not found",
//                         "result": Value::Null
//                     })
//                     .to_string(),
//                 );
//             }
//         });
//     }

//     let mut res_vec: Vec<(u64, u32, u32, u32)> = Vec::new();
//     for (i, el) in timeserieses.first().unwrap().values.iter().enumerate() {
//         res_vec.push((el.0, el.1, timeserieses[1].values[i].1, timeserieses[2].values[i].1));
//     }

//     HttpResponse::Ok().body(
//         json!({
//             "error": Value::Null,
//             "result": res_vec
//         })
//         .to_string(),
//     )
// }

// #[get("/workerHistory")]
// async fn share_history(
//     req: HttpRequest,
//     info: web::Query<OverviewQuery>,
//     api_data: web::Data<SickApiData>,
// ) -> impl Responder {
//     let mut con = api_data.redis.clone();

//     let mut valid_shares_key = String::from("shares:valid:miner:");
//     valid_shares_key.push_str(&info.address);

//     let mut stale_shares_key = String::from("shares:stale:miner:");
//     stale_shares_key.push_str(&info.address);

//     let mut invalid_shares_key = String::from("shares:invalid:miner:");
//     invalid_shares_key.push_str(&info.address);

//     let mut timeserieses: Vec<TsRange<u64, u32>> = Vec::new();

//     for key in [valid_shares_key, stale_shares_key, invalid_shares_key] {
//         timeserieses.push(match con.ts_range(key, 0, "+", None::<usize>, None).await {
//             Ok(res) => res,
//             Err(err) => {
//                 eprintln!("range query error: {}", err);
//                 return HttpResponse::NotFound().body(
//                     json!({
//                         "error": "Key not found",
//                         "result": Value::Null
//                     })
//                     .to_string(),
//                 );
//             }
//         });
//     }

//     let mut res_vec: Vec<(u64, u32, u32, u32)> = Vec::new();
//     for (i, el) in timeserieses.first().unwrap().values.iter().enumerate() {
//         res_vec.push((el.0, el.1, timeserieses[1].values[i].1, timeserieses[2].values[i].1));
//     }

//     HttpResponse::Ok().body(
//         json!({
//             "error": Value::Null,
//             "result": res_vec
//         })
//         .to_string(),
//     )
// }
