use crate::SickApiData;
use actix_web::{get, web, HttpRequest, HttpResponse, Responder};
extern crate redis;
use redis::AsyncCommands;
use redis_ts::{AsyncTsCommands, TsAggregationType, TsFilterOptions, TsMget, TsMrange, TsOptions};
use serde_json::json;
use std::collections::HashMap;

// use redisearch_api::{init, Document, FieldType, Index, TagOptions};
use serde::{Deserialize, Serialize};

#[get("/currentHashrate")]
async fn current_hashrate(req: HttpRequest, api_data: web::Data<SickApiData>) -> impl Responder {
    let con = api_data.redis.clone();
    // redis::cmd("TS.GET")

    HttpResponse::Ok().body("Hey there!")
}

#[get("/workerCount")]
async fn worker_count(req: HttpRequest, api_data: web::Data<SickApiData>) -> impl Responder {
    let mut con = api_data.redis.clone();
    // let dbRes : std::result::Result<redis::Value, redis::RedisError> = redis::cmd("TS.GET").arg("VRSC:worker_count")
    //             .query_async(&mut con).await;
    // let dbRes : std::result::Result<u32, redis::RedisError> = redis::cmd("GET").arg("VRSC:1").query_async(&mut con).await;
    let latest: std::result::Result<Option<(u64, f64)>, redis::RedisError> =
        con.ts_get("my_engine").await;

    match latest {
        Ok(res) => match res {
            Some(val) => return HttpResponse::Ok().body(val.1.to_string()),
            None => {
                return HttpResponse::InternalServerError().body("Empty array");
            }
        },
        Err(err) => {
            eprintln!("err: {}", err);
            return HttpResponse::InternalServerError().body("Data not found");
        }
    }
}

// #[get("/stakingBalance")]
// async fn staking_balance(req: HttpRequest, api_data: web::Data<SickApiData>) -> impl Responder {
//     let mut con = api_data.redis.clone();
//     // let dbRes : std::result::Result<redis::Value, redis::RedisError> = redis::cmd("TS.GET").arg("VRSC:worker_count")
//     //             .query_async(&mut con).await;
//     // let dbRes : std::result::Result<u32, redis::RedisError> = redis::cmd("GET").arg("VRSC:1").query_async(&mut con).await;
//     let latest_balances: std::result::Result<TsMget<u64, f64>, redis::RedisError> = con
//         .ts_mget(
//             TsFilterOptions::default()
//                 .with_labels(true)
//                 .equals("type", "balance"),
//         )
//         .await;

//     match latest_balances {
//        Ok(res) => {
//             let mut sum : f64 = 0.0;

//             for entry in res.values.iter(){
//                 for values in entry.value{
//                     sum += values.1;
//                 }
//             }redis::cmd("FT.SEARCH")
//         Err(err) => {
//             eprintln!("err: {}", err);
//             return HttpResponse::InternalServerError().body("Data not found");
//         }
//     }
// }

#[get("/stakingBalance")]
async fn staking_balance(req: HttpRequest, api_data: web::Data<SickApiData>) -> impl Responder {
    let mut con = api_data.redis.clone();
    // let dbRes : std::result::Result<redis::Value, redis::RedisError> = redis::cmd("TS.GET").arg("VRSC:worker_count")
    //             .query_async(&mut con).await;
    // let dbRes : std::result::Result<u32, redis::RedisError> = redis::cmd("GET").arg("VRSC:1").query_async(&mut con).await;
    let latest_balances: std::result::Result<TsMrange<u64, f64>, redis::RedisError> = con
        .ts_mrange(
            "0",
            "+",
            Some(0),
            None,
            TsFilterOptions::default()
                .with_labels(true)
                .equals("type", "balance"),
        )
        .await;

    match latest_balances {
        Ok(res) => {
            let mut sum: f64 = 0.0;

            for entry in res.values.iter() {
                for values in entry.values.iter() {
                    sum += values.1 * 10.;
                }
            }

            return HttpResponse::Ok().body(sum.to_string());
        }
        Err(err) => {
            eprintln!("err: {}", err);
            return HttpResponse::InternalServerError().body("Data not found");
        }
    }
}

// #[get("/minerCount")]
// async fn current_hashrate(req: HttpRequest, api_data: web::Data<SickApiData>) -> impl Responder {
//     let con = api_data.redis.clone();
//     redis::cmd("TS.GET")

//     HttpResponse::Ok().body("Hey there!")
// }
#[get("/blockNumber")]
async fn block_number(req: HttpRequest, api_data: web::Data<SickApiData>) -> impl Responder {
    let mut con = api_data.redis.clone();

    let block_number: std::result::Result<i32, redis::RedisError> =
        con.get("VRSC:block_number").await;

    match block_number {
        Ok(num) => {
            // println!("num {}", num);
            return HttpResponse::Ok().body(num.to_string());
        }
        Err(err) => {
            eprintln!("err: {}", err);
            return HttpResponse::InternalServerError().body("Data not found");
        }
    }
    // return HttpResponse::Ok();
}

#[derive(Deserialize)]
struct BlocksQuery {
    page: u32,
    limit: u8,
    sortby: String,
    sortdir: String,
    // chains: Vec<String>,
}

#[derive(Debug)]
pub struct SearchRes {
    pub results: Vec<HashMap<String, String>>,
    pub total: i64,
}
#[derive(Debug)]
pub struct BlockRaw {
    pub reward: i64,
    pub time: i64,
    pub duration: i64,
    pub height: u32,
    pub number: u32,
    pub difficulty: f64,
    pub effort_percent: f64,
    pub chain: [u8; 8],
    pub solver: [u8; 34],
    pub worker: [u8; 16],
    pub hash: [u8; 64],
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Block {
    pub reward: i64,
    pub time: i64,
    pub duration: i64,
    pub height: u32,
    pub number: u32,
    pub difficulty: f64,
    pub effort_percent: f64,
    pub chain: String,
    pub solver: String,
    pub worker: String,
    pub hash: String,
}

#[derive(Debug, Serialize)]
pub struct BlockRes {
    pub total: i64,
    pub blocks: Vec<Block>,
}

#[get("/blocks")]
async fn blocks(info: web::Query<BlocksQuery>, api_data: web::Data<SickApiData>) -> impl Responder {
    // let offset : u32 = info.page * info.limit as u32;
    //TODO: check validity of sortby and sortdir and chains

    let mut sort_index = info.sortby.clone();
    sort_index.insert_str(0, "block:");
    println!("{}", sort_index);

    let mut con = api_data.redis.clone();
    let res: BlockRes = redis::cmd("FCALL")
        .arg("getblocksbyindex")
        .arg(1)
        .arg(sort_index)
        .arg(info.page * info.limit as u32) // offset
        .arg(info.limit) // num (limit)
        // .arg(info.sortdir.to_uppercase())
        .query_async(&mut con)
        .await
        .unwrap();

    let json_res = json!(res).to_string();
    // println!("{:?}", res);
    // match res{
    //     Ok(it) => {
    //         let keys : Vec<_> = it.collect();
    //         println!("{:?}", keys);
    return HttpResponse::Ok().body(json_res);

    //     },
    //     Err(err) => {
    //         eprintln!("err: {}", err);
    //         return HttpResponse::InternalServerError().body("Data not found");
    //     }
    // }
}

#[get("/currentEffortPoW")]
async fn current_effort_pow(req: HttpRequest, api_data: web::Data<SickApiData>) -> impl Responder {
    let mut con = api_data.redis.clone();
    let effort_res: std::result::Result<HashMap<String, f64>, redis::RedisError> =
        redis::cmd("HGETALL")
            .arg("VRSCTEST:round_effort_pow")
            .query_async(&mut con)
            .await;

    match effort_res {
        Ok(effort_map) => {
            let effort: f64 = effort_map["total"] / effort_map["estimated"];
            println!("effort: {:?}", effort);

            return HttpResponse::Ok().body(effort.to_string());
        }
        Err(err) => {
            eprintln!("err: {}", err);
            return HttpResponse::InternalServerError().body("Data not found");
        }
    }
}

#[get("/dashboard/{addr}")]
async fn dashboard(
    req: HttpRequest,
    addr: web::Path<String>,
    api_data: web::Data<SickApiData>,
) -> impl Responder {
    let mut con = api_data.redis.clone();
    let addr: String = addr.into_inner();

    let is_id = addr.ends_with('@');
    let filter: TsFilterOptions;

    if is_id {
        filter = TsFilterOptions::default()
            .with_labels(true)
            .equals("type", "hashrate")
            .equals("identity", &addr);
    } else {
        filter = TsFilterOptions::default()
            .with_labels(true)
            .equals("type", "hashrate")
            .equals("address", &addr);
    }

    let workers_hashrate: TsMrange<u64, f64> =
        match con.ts_mrange("0", "+", Some(0), None, filter).await {
            Ok(r) => r,
            Err(_) => return HttpResponse::InternalServerError().body("Data not found"),
        };

    let res = json!({ "hashrate": workers_hashrate.values[0].values});

    println!("res {:?}", workers_hashrate);
    HttpResponse::Ok().body(res.to_string())
    // HttpResponse::Ok().body(json!(workers_hashrate).to_string())
}

pub fn pool_route(cfg: &mut web::ServiceConfig) {
    cfg.service(staking_balance);
    cfg.service(current_hashrate);
    cfg.service(worker_count);
    cfg.service(block_number);
    cfg.service(blocks);
    cfg.service(current_effort_pow);
    cfg.service(dashboard);
}

impl redis::FromRedisValue for BlockRes {
    fn from_redis_value(value: &redis::Value) -> redis::RedisResult<Self> {
        let items: Vec<redis::Value> = redis::from_redis_value(value)?;
        if items.len() < 2 {
            return Ok(BlockRes {
                blocks: vec![],
                total: 0,
            });
        }
        let total_res_count: i64 = redis::from_redis_value(items.first().unwrap())?;

        let block_arr: Vec<redis::Value> = redis::from_redis_value(&items[1])?;
        let mut results: Vec<Block> = Vec::new();

        for block in block_arr {
            let bytes: Vec<u8> = redis::from_redis_value(&block)?;
            println!("{:?}", bytes);
            unsafe {
                let block = Block {
                    reward: std::mem::transmute_copy::<[u8;8], i64>(&bytes[0..8].try_into().unwrap()),
                    time: std::mem::transmute_copy::<[u8;8], i64>(&bytes[8..16].try_into().unwrap()),
                    duration: std::mem::transmute_copy::<[u8;8], i64>(&bytes[16..24].try_into().unwrap()),
                    height: std::mem::transmute_copy::<[u8;4], u32>(&bytes[24..28].try_into().unwrap()),
                    number: std::mem::transmute_copy::<[u8;4], u32>(&bytes[28..32].try_into().unwrap()),
                    difficulty: std::mem::transmute_copy::<[u8;8], f64>(&bytes[32..40].try_into().unwrap()),
                    effort_percent:std::mem::transmute_copy::<[u8;8], f64>(&bytes[40..48].try_into().unwrap()),
                    chain: String::from_utf8((&bytes[48..56]).to_vec())?,
                    solver: String::from_utf8((&bytes[56..90]).to_vec())?,
                    worker: String::from_utf8((&bytes[90..106]).to_vec())?,
                    hash: String::from_utf8((&bytes[106..170]).to_vec())?,
                };
            println!("BLOCK {:?}", block);
            results.push(block);
            }
            // unsafe {
            //     let block_raw: BlockRaw = std::ptr::read(bytes.as_ptr() as *const _);
            //     let solver_str = String::from_utf8(block_raw.solver.to_vec())?;
            //     let worker_str = String::from_utf8(block_raw.worker.to_vec())?;
            //     let chain_str = String::from_utf8(block_raw.chain.to_vec())?;
            //     let hash_str = String::from_utf8(block_raw.hash.to_vec())?;

            //     let block = Block {
            //         reward: block_raw.reward,
            //         time: block_raw.time,
            //         duration: block_raw.duration,
            //         height: block_raw.height,
            //         number: block_raw.number,
            //         difficulty: block_raw.difficulty,
            //         effort_percent: block_raw.effort_percent,
            //         chain: chain_str,
            //         solver: solver_str,
            //         worker: worker_str,
            //         hash: hash_str,
            //     };
            //     println!("BLOCK {:?}", block);
            //     // results.push(block);
            // }
        }
        Ok(BlockRes {
            blocks: results,
            total: total_res_count,
        })
    }
}

// impl redis::FromRedisValue for BlockRes {
//     fn from_redis_value(value: &redis::Value) -> redis::RedisResult<Self> {
//         let search_res: SearchRes = redis::from_redis_value(value).unwrap();
//         println!("{:?}", search_res);
//         let mut blocks_vec: Vec<Block> = Vec::new();
//         // TODO: exception handle
//         for item in search_res.results {
//             // let block = parse_block(item);
//             // match block {
//             //     Ok(block) => blocks_vec.push(block),
//             //     Err(err) => eprintln!("Failed to parse block: {}", err),
//             }
//         }

//         Ok(BlockRes {
//             blocks: blocks_vec,
//             total: search_res.total,
//         })
//     }
// }

// pub enum BlockStatus {
//     Rejected = 0,
//     Accepted = 1,
// }

// pub fn parse_block(hm: HashMap<String, String>) -> Result<Block, String> {
//     let block = Block {
//         number: hm
//             .get("number")
//             .ok_or("No number key")?
//             .parse::<u32>()
//             .unwrap(),
//         height: hm
//             .get("height")
//             .ok_or("No height key")?
//             .parse::<u32>()
//             .unwrap(),
//         chain: hm.get("chain").ok_or("No chain key")?.to_string(),
//         block_type: hm.get("type").ok_or("No type key")?.to_string(),
//         is_accepted: if hm
//             .get("accepted")
//             .ok_or("No accepted key")?
//             .parse::<u8>()
//             .unwrap()
//             == 1
//         {
//             true
//         } else {
//             false
//         },
//         time: hm.get("time").ok_or("No time key")?.parse::<u64>().unwrap(),
//         duration: hm
//             .get("duration")
//             .ok_or("No duration key")?
//             .parse::<u64>()
//             .unwrap(),
//         solver: hm.get("solver").ok_or("No solver key")?.to_string(),
//         reward: hm
//             .get("reward")
//             .ok_or("No reward key")?
//             .parse::<u64>()
//             .unwrap(),
//         difficulty: hm
//             .get("difficulty")
//             .ok_or("No difficulty key")?
//             .parse::<f64>()
//             .unwrap(),
//         effort_percent: hm
//             .get("effort_percent")
//             .ok_or("No effort_percent key")?
//             .parse::<f64>()
//             .unwrap(),
//         hash: hm.get("hash").ok_or("No hash key")?.to_string(),
//     };
//     Ok(block)
// }

// impl<T: redis::FromRedisValue> redis::FromRedisValue for FtQuery<T> {
//     fn from_redis_value(v: &redis::Value) -> redis::RedisResult<Self> {
//         let mut bulk = match v {
//             redis::Value::Bulk(v) => v.into_iter(),
//             _ => {
//                 return Err(redis::RedisError::from((
//                     redis::ErrorKind::TypeError,
//                     "Type error",
//                 )))
//             }
//         };

//         let count = match bulk.next() {
//             Some(redis::Value::Int(v)) => *v,
//             _ => {
//                 return Err(redis::RedisError::from((
//                     redis::ErrorKind::TypeError,
//                     "Type error",
//                 )))
//             }
//         };

//         let mut result = Vec::with_capacity(count.try_into().unwrap());

//         while let Some(v) = bulk.next() {
//             let key = <String as FromRedisValue>::from_redis_value(&v)?;
//             let value = match bulk.next() {
//                 Some(redis::Value::Bulk(v)) => T::from_redis_value(v.last().unwrap())?,
//                 _ => {
//                     return Err(redis::RedisError::from((
//                         redis::ErrorKind::TypeError,
//                         "Type error",
//                     )))
//                 }
//             };
//             result.push((key, value));
//         }

//         Ok(Self { container: result })
//     }
// }
//TODO: make general for search then specify for blocks
