#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]

use actix_web::{get, web, App, HttpServer, Responder, middleware};
extern crate redis;
// use redis::Commands;
use env_logger::Env;

mod routes;
use routes::pool;
use routes::miner;
use routes::solver;

mod api_data;
use api_data::SickApiData;

mod redis_interop;
use redis_interop::ffi;


mod pool_events;
use pool_events::listen_redis;

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

    env_logger::init_from_env(Env::default().default_filter_or("info"));
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
                    }))
                    .service(web::scope("/pool").configure(pool::pool_route))
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
