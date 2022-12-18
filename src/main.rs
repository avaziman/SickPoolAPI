#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]

use actix_web::{get, middleware, web, App, HttpServer, Responder};
extern crate redis;
// use redis::Commands;
use env_logger::Env;

mod routes;
use routes::miner;
use routes::network;
use routes::pool;
use routes::solver;

mod api_data;
use api_data::SickApiData;

mod redis_interop;
use redis_interop::ffi;

use mysql::Pool;
mod pool_events;
use pool_events::listen_redis;

use std::fs::File;
use std::io::Read;

use crate::routes::history::TimeSeriesInterval;
use serde_json::json;

#[get("/hello/{name}")]
async fn greet(name: web::Path<String>) -> impl Responder {
    format!("Hello {name}!")
}

// pub enum FieldType {
//     Numeric,
//     Tag,
// }

// pub struct IndexField {
//     pub name: String,
//     pub field_type: FieldType,
//     pub sortable: bool,
// }

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Starting SickPool API!");

    println!("Connecting to redisDB...");
    let client = redis::Client::open("redis://127.0.0.1/").expect("Can't create Redis client");
    let con_manager: redis::aio::ConnectionManager = client
        .get_tokio_connection_manager()
        .await
        .expect("Can't create Redis connection manager");
    println!("RedisDB connection successful!");

    println!("Connecting to MySqlDB...");
    let pool =
        Pool::new("mysql://root:password@127.0.0.1:3306/ZANO").expect("Can't connect to MySql DB");

    env_logger::init_from_env(Env::default().default_filter_or("info"));

    let file =
        File::open("/home/sickguy/Documents/Projects/SickPool/server/config/coins/ZANOTEST.json")?;

    let json: serde_json::Value = serde_json::from_reader(file).expect("Config file is broken");

    let hr_interval: u64 = json
        .get("stats")
        .unwrap()
        .get("hashrate_interval_seconds")
        .unwrap()
        .as_u64()
        .unwrap();

    let hr_retention: u64 = json
        .get("redis")
        .unwrap()
        .get("hashrate_ttl")
        .unwrap()
        .as_u64()
        .unwrap();

    let block_interval: u64 = json
        .get("stats")
        .unwrap()
        .get("mined_blocks_interval")
        .unwrap()
        .as_u64()
        .unwrap();

    println!("Hashrate interval: {}", hr_interval);
    println!("Hashrate ttl: {}", hr_retention);

    let hr_timeseries = TimeSeriesInterval {
        interval: hr_interval,
        retention: hr_retention,
    };

    let block_timeseries = TimeSeriesInterval {
        interval: block_interval,
        retention: block_interval * 30,
    };
    // allow all origins
    std::thread::spawn(move || {
        listen_redis(&client);
    });

    HttpServer::new(move || {
        let cors = actix_cors::Cors::permissive();

        App::new()
            .service(
                web::scope("/api")
                    .app_data(web::Data::new(SickApiData {
                        redis: con_manager.clone(),
                        mysql: pool.clone(),
                        hashrate_interval: hr_timeseries.clone(),
                        block_interval: block_timeseries.clone(),
                    }))
                    .service(web::scope("/pool").configure(pool::pool_route))
                    .service(web::scope("/network").configure(network::network_route))
                    .service(web::scope("/miner").configure(miner::miner_route))
                    .service(web::scope("/solver").configure(solver::solver_route)),
            )
            .wrap(cors)
        // .wrap(middleware::Logger::default())
    })
    .bind(("0.0.0.0", 2222))?
    .run()
    .await
}
