use crate::SickApiData;

use actix_web::{get, web, HttpRequest, HttpResponse, Responder};
use redis_ts::{AsyncTsCommands, TsRange};
use serde::Deserialize;
use serde_json::{json, Value};

pub fn miner_route(cfg: &mut web::ServiceConfig) {
    cfg.service(hashrate_history);
    cfg.service(share_history);
}

#[derive(Deserialize)]
struct SolverQuery {
    address: String,
}

#[derive(Deserialize)]
struct HashrateEntry {
    time: u64,
    currentHr: f64,
    averageHr: f64,
    validShares: u32,
    staleShares: u32,
    invalidShares: u32,
    workers: u32,
}

#[get("/statsHistory")]
async fn hashrate_history(
    req: HttpRequest,
    info: web::Query<SolverQuery>,
    api_data: web::Data<SickApiData>,
) -> impl Responder {
    let mut con = api_data.redis.clone();

    let hashrate_key = String::from("hashrate:miner:") + &info.address;
    let avg_hashrate_key = String::from("hashrate:average:miner:") + &info.address;

    let valid_shares_key = String::from("shares:valid:miner:") + &info.address;
    let stale_shares_key = String::from("shares:stale:miner:") + &info.address;
    let invalid_shares_key = String::from("shares:invalid:miner:") + &info.address;

    let worker_count_key = String::from("worker-count:") + &info.address;
    let mut timeserieses: Vec<TsRange<u64, f64>> = Vec::new();

    for key in [
        hashrate_key,
        avg_hashrate_key,
        valid_shares_key,
        stale_shares_key,
        invalid_shares_key,
        worker_count_key,
    ] {
        timeserieses.push(match con.ts_range(key, 0, "+", None::<usize>, None).await {
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
        });
    }

    let mut res_vec: Vec<HashrateEntry> = Vec::new();
    for (i, el) in timeserieses.first().unwrap().values.iter().enumerate() {
        res_vec.push(HashrateEntry{
            time: el.0,
            currentHr: el.1,
            averageHr: timeserieses[1].values[i].1,
            validShares: timeserieses[2].values[i].1 as u32,
            staleShares: timeserieses[3].values[i].1 as u32,
            invalidShares: timeserieses[4].values[i].1 as u32,
            workers: timeserieses[5].values[i].1 as u32,
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
//     info: web::Query<SolverQuery>,
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
//     info: web::Query<SolverQuery>,
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
