use crate::routes::redis::get_range_params;
use crate::SickApiData;

use super::solver::OverviewQuery;
use crate::ffi::Prefix;
use crate::routes::redis::fill_gaps;
use crate::routes::redis::key_format;
use crate::solver::miner_id_filter;
use actix_web::{get, web, HttpRequest, HttpResponse, Responder};
use redis::aio::ConnectionManager;
use redis_ts::{AsyncTsCommands, TsAggregationOptions, TsFilterOptions, TsMget, TsMrange, TsRange};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub fn miner_route(cfg: &mut web::ServiceConfig) {
    cfg.service(stats_history);
    cfg.service(worker_history);
    cfg.service(workers);
}

#[repr(C)]
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct HashrateEntry {
    average_hr: f64,
    current_hr: f64,
    invalid_shares: u64,
    stale_shares: u64,
    time: u64,
    valid_shares: u64,
}

#[derive(Serialize, Clone)]
struct WorkerStatsEntry {
    worker: String,
    stats: HashrateEntry,
}

#[derive(Serialize, Clone)]
struct WorkerTsEntry {
    time: u64,
    workers: u32,
}

#[get("/statsHistory")]
async fn stats_history(
    info: web::Query<OverviewQuery>,
    api_data: web::Data<SickApiData>,
) -> impl Responder {
    let mut con = api_data.redis.clone();

    let ts_type = format!(
        "({},\
                {},\
                {},\
                {},\
                {})",
        &Prefix::HASHRATE.to_string(),
        &key_format(&[&Prefix::HASHRATE.to_string(), &Prefix::AVERAGE.to_string()]),
        &key_format(&[&Prefix::SHARES.to_string(), &Prefix::VALID.to_string()]),
        &key_format(&[&Prefix::SHARES.to_string(), &Prefix::STALE.to_string()]),
        &key_format(&[&Prefix::SHARES.to_string(), &Prefix::INVALID.to_string()])
    );

    let filter: TsFilterOptions = miner_id_filter(&info.address.to_lowercase())
        .equals("prefix", Prefix::MINER.to_string())
        .equals("type", ts_type);

    let tms: TsMrange<u64, f64> = match con
        .ts_mrange(0, "+", None::<usize>, None::<TsAggregationOptions>, filter)
        .await
    {
        Ok(res) => res,
        Err(err) => {
            eprintln!("range query error: {}", err);
            return HttpResponse::NotFound().body(
                json!({
                    "error": "Key not found",
                    "result": Value::Null
                })
                .to_string(),
            );
        }
    };

    if tms.values.len() != 5 {
        return HttpResponse::NotFound().body(
            json!({
                "error": "Missing data",
                "result": Value::Null
            })
            .to_string(),
        );
    }

    let (first_timestamp, last_timestamp, points_amount) =
        get_range_params(&api_data.hashrate_interval);

    let mut results: Vec<Vec<(u64, f64)>> = Vec::new();
    results.reserve(5);

    for (i, el) in tms.values.iter().enumerate() {
        results.push(fill_gaps(
            el.values.clone(),
            first_timestamp,
            api_data.hashrate_interval.interval,
            points_amount,
        ));
    }
    let mut res_vec: Vec<HashrateEntry> = Vec::new();
    res_vec.reserve(points_amount as usize);

    // timeserieses are sorted by alphabetical order
    for (i, el) in results.first().unwrap().iter().enumerate() {
        res_vec.push(HashrateEntry {
            average_hr: el.1,
            current_hr: results[1][i].1,
            invalid_shares: results[2][i].1 as u64,
            stale_shares: results[3][i].1 as u64,
            time: el.0, // seconds not ms
            valid_shares: results[4][i].1 as u64,
        });
    }

    HttpResponse::Ok().body(
        json!({
            "error": Value::Null,
            "result": res_vec
        })
        .to_string(),
    )
}

#[get("/workerHistory")]
async fn worker_history(
    req: HttpRequest,
    info: web::Query<OverviewQuery>,
    api_data: web::Data<SickApiData>,
) -> impl Responder {
    let mut con = api_data.redis.clone();

    let ts_type = "worker-count";

    let filter: TsFilterOptions = miner_id_filter(&info.address).equals("type", ts_type);

    let tms: TsMrange<u64, f64> = match con
        .ts_mrange(0, "+", None::<usize>, None::<TsAggregationOptions>, filter)
        .await
    {
        Ok(res) => res,
        Err(err) => {
            eprintln!("range query error: {}", err);
            return HttpResponse::NotFound().body(
                json!({
                    "error": "Key not found",
                    "result": Value::Null
                })
                .to_string(),
            );
        }
    };

    let ts = match tms.values.first() {
        Some(res) => res,
        None => {
            return HttpResponse::NotFound().body(
                json!({
                    "error": "Key not found",
                    "result": Value::Null
                })
                .to_string(),
            );
        }
    };

    let mut res_vec: Vec<WorkerTsEntry> = Vec::new();
    res_vec.reserve(ts.values.len());

    // timeserieses are sorted by alphabetical order
    for (i, el) in ts.values.iter().enumerate() {
        res_vec.push(WorkerTsEntry {
            time: el.0,
            workers: el.1 as u32,
        });
    }

    HttpResponse::Ok().body(
        json!({
            "error": Value::Null,
            "result": res_vec
        })
        .to_string(),
    )
}

#[get("/workers")]
async fn workers(
    req: HttpRequest,
    info: web::Query<OverviewQuery>,
    api_data: web::Data<SickApiData>,
) -> impl Responder {
    let mut con = api_data.redis.clone();

    let ts_type = format!(
        "({},\
            {},\
            {},\
            {},\
            {})",
        &Prefix::HASHRATE.to_string(),
        &key_format(&[&Prefix::HASHRATE.to_string(), &Prefix::AVERAGE.to_string()]),
        &key_format(&[&Prefix::SHARES.to_string(), &Prefix::VALID.to_string()]),
        &key_format(&[&Prefix::SHARES.to_string(), &Prefix::STALE.to_string()]),
        &key_format(&[&Prefix::SHARES.to_string(), &Prefix::INVALID.to_string()])
    );

    // TODO: make selected labels support
    let filter: TsFilterOptions;
    filter = miner_id_filter(&info.address)
        .equals("prefix", Prefix::WORKER.to_string())
        .equals("type", ts_type)
        .with_labels(true);

    let tms: TsMget<u64, f64> = match con.ts_mget(filter).await {
        Ok(res) => res,
        Err(err) => {
            eprintln!("tsget query error: {}", err);
            return HttpResponse::NotFound().body(
                json!({
                    "error": "Key not found",
                    "result": Value::Null
                })
                .to_string(),
            );
        }
    };
    // println!("{:?}", tms);
    let worker_count = tms.values.len() / 5;
    let mut res_vec: Vec<WorkerStatsEntry> = Vec::new();
    res_vec.reserve(worker_count);

    // timeserieses are sorted by alphabetical order
    for i in 0..worker_count {
        let worker_name_label: &(String, String) = &tms.values[i * 5].labels[4];

        if tms.values[i].value.is_none() {
            // the worker hasn't gotten any statistics yet
            continue;
        };

        // if (tms.values[i].value.unwrap().1 < ){

        // }

        res_vec.push(WorkerStatsEntry {
            worker: worker_name_label.1.clone(),
            stats: HashrateEntry {
                average_hr: tms.values[i].value.unwrap().1,
                current_hr: tms.values[i + 1 * worker_count].value.unwrap().1,
                invalid_shares: tms.values[i + worker_count * 2].value.unwrap().1 as u64,
                stale_shares: tms.values[i + worker_count + 3].value.unwrap().1 as u64,
                time: tms.values[i].value.unwrap().0,
                valid_shares: tms.values[i + worker_count * 4].value.unwrap().1 as u64,
            },
        });
    }

    HttpResponse::Ok().body(
        json!({
            "error": Value::Null,
            "result": {
                "total": res_vec.len(),
                "entries": res_vec,
            }
        })
        .to_string(),
    )
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
