use crate::SickApiData;

use super::solver::OverviewQuery;
use actix_web::{get, web, HttpRequest, HttpResponse, Responder};
use redis_ts::{AsyncTsCommands, TsFilterOptions, TsMget, TsMrange, TsRange};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use crate::solver::miner_id_filter;

pub fn miner_route(cfg: &mut web::ServiceConfig) {
    cfg.service(stats_history);
    cfg.service(worker_history);
    cfg.service(workers);
}

#[repr(C)]
#[derive(Serialize, Clone)]
struct HashrateEntry {
    averageHr: f64,
    currentHr: f64,
    invalidShares: f64,
    staleShares: f64,
    time: u64,
    validShares: f64,
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

    let tsType = "(hashrate,\
                hashrate:average,\
                shares:valid,\
                shares:stale,\
                shares:invalid)";

    let filter: TsFilterOptions = miner_id_filter(&info.address)
            .equals("prefix", "miner")
            .equals("type", tsType);

    let tms: TsMrange<u64, f64> = match con.ts_mrange(0, "+", None::<usize>, None, filter).await {
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
                "error": "Key not found",
                "result": Value::Null
            })
            .to_string(),
        );
    }

    let ts = tms.values.first().unwrap();

    let mut res_vec: Vec<HashrateEntry> = Vec::new();
    res_vec.reserve(ts.values.len());

    // timeserieses are sorted by alphabetical order
    for (i, el) in ts.values.iter().enumerate() {
        res_vec.push(HashrateEntry {
            averageHr: el.1,
            currentHr: tms.values[1].values[i].1,
            invalidShares: tms.values[2].values[i].1,
            staleShares: tms.values[3].values[i].1,
            time: el.0,
            validShares: tms.values[4].values[i].1,
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

    let tsType = "worker-count";

    let filter: TsFilterOptions = 
        miner_id_filter(&info.address)
            .equals("type", tsType);

    let tms: TsMrange<u64, f64> = match con.ts_mrange(0, "+", None::<usize>, None, filter).await {
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

    let keys = "(hashrate,\
                hashrate:average,\
                shares:valid,\
                shares:stale,\
                shares:invalid)";

    let filter: TsFilterOptions;
        filter = miner_id_filter(&info.address)
            .equals("prefix", "worker")
            .equals("type", keys);

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
    println!("{:?}", tms);
    let worker_count = tms.values.len() / 5;
    let mut res_vec: Vec<WorkerStatsEntry> = Vec::new();
    res_vec.reserve(worker_count);

    // timeserieses are sorted by alphabetical order
    for i in 0..worker_count {
        let worker_vec: Vec<&str> = tms.values[i * 5].key.split('.').collect();
        res_vec.push(WorkerStatsEntry {
            worker: String::from(worker_vec[1]),
            stats: HashrateEntry {
                averageHr: tms.values[i].value.unwrap().1,
                currentHr: tms.values[i + 1 * worker_count].value.unwrap().1,
                invalidShares: tms.values[i + worker_count * 2].value.unwrap().1,
                staleShares: tms.values[i + worker_count + 3].value.unwrap().1,
                time: tms.values[i].value.unwrap().0,
                validShares: tms.values[i + worker_count * 4].value.unwrap().1,
            },
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
