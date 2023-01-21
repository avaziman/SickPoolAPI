#![crate_type = "bin"]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]

use actix_web::{get, middleware, web, App, HttpServer, Responder};
use mysql::OptsBuilder;
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
extern crate redis;
use env_logger::Env;
use std::env;

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

mod config;
use config::CoinConfig;

use crate::routes::history::TimeSeriesInterval;
use serde_json::json;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Starting SickPool API!");

    let args: Vec<String> = env::args().collect();
    let config_path = args.get(1).expect("No configuration file path provided.");

    println!("Config file path: {}", config_path);

    let file = File::open(config_path)?;

    let config: CoinConfig = serde_json::from_reader(file).expect("Config file is invalid");

    let host_sep = config.mysql.host.find(":").expect("Invalid mysql host");
    let mysql_opts = OptsBuilder::new()
        .ip_or_hostname(Some(&config.mysql.host[0..host_sep]))
        .tcp_port(config.mysql.host[(host_sep + 1)..].parse::<u16>().unwrap())
        .user(Some(config.mysql.user))
        .pass(Some(config.mysql.pass))
        .db_name(Some(config.mysql.db_name));

    println!("Connecting to MySqlDB...");
    let pool = Pool::new(mysql_opts).expect("Can't connect to MySql DB");

    println!("Connecting to redisDB...");
    let client = redis::Client::open(String::from("redis://") + &config.redis.host).expect("Can't create Redis client");
    let con_manager: redis::aio::ConnectionManager = client
        .get_tokio_connection_manager()
        .await
        .expect("Can't create Redis connection manager");
    println!("RedisDB connection successful!");

    env_logger::init_from_env(Env::default().default_filter_or("info"));

    println!(
        "Hashrate interval: {}",
        config.stats.hashrate_interval_seconds
    );
    println!("Hashrate ttl: {}", config.redis.hashrate_ttl_seconds);

    let hr_timeseries = TimeSeriesInterval {
        interval: config.stats.hashrate_interval_seconds as u64,
        retention: config.redis.hashrate_ttl_seconds as u64,
    };

    let block_timeseries = TimeSeriesInterval {
        interval: config.stats.mined_blocks_interval as u64,
        retention: config.stats.mined_blocks_interval as u64 * 30u64,
    };
    // allow all origins
    std::thread::spawn(move || {
        listen_redis(&client);
    });

    HttpServer::new(move || {
        let cors = actix_cors::Cors::default().allow_any_origin();

        App::new()
            .service(
                web::scope("/api")
                    .app_data(web::Data::new(SickApiData {
                        redis: con_manager.clone(),
                        mysql: pool.clone(),
                        hashrate_interval: hr_timeseries.clone(),
                        block_interval: block_timeseries.clone(),
                        min_payout: config.min_payout_threshold,
                        fee: config.pow_fee
                    }))
                    .service(web::scope("/pool").configure(pool::pool_route))
                    .service(web::scope("/network").configure(network::network_route))
                    .service(web::scope("/miner").configure(miner::miner_route))
                    .service(web::scope("/solver").configure(solver::solver_route)),
            )
            .wrap(cors)
        // .wrap(middleware::Logger::default())
    })
    .bind(("127.0.0.1", 2222))?
    .run()
    .await
}
