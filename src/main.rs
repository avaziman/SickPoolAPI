use actix_web::{get, middleware, web, App, HttpResponse, HttpServer, Responder};
extern crate redis;
use std::sync::Mutex;
// use redis::Commands;
use env_logger::Env;
use std::collections::HashSet;

use redis::AsyncCommands;

mod routes;
use routes::pool;

mod api_data;
use api_data::SickApiData;

mod pool_events;
use pool_events::listen_redis;

static COIN: &str = "VRSC";

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
    let mut con_manager: redis::aio::ConnectionManager = client
        .get_tokio_connection_manager()
        .await
        .expect("Can't create Redis connection manager");

    println!("RedisDB connection successful!");

    env_logger::init_from_env(Env::default().default_filter_or("info"));
    // allow all origins
    std::thread::spawn(move || {
        listen_redis(&client);
    });

    // create search indexes that the api consumes
    // no need to index time as its same as number
    // let mut block_index_cmd =
    let mut block_index_cmd = redis::cmd("FT.CREATE");

    block_index_cmd
        .arg("SP:block_index")
        .arg("ON")
        .arg("HASH")
        .arg("PREFIX")
        .arg("1")
        .arg("VRSCTEST:block:")
        .arg("SCHEMA");
    //todo: change to block:chain:height

    let indexes: HashSet<(&str, &str, bool)> = HashSet::from([
        ("accepted", "TAG", false),
        ("solver", "TAG", false),
        ("type", "TAG", false),
        ("chain", "TAG", false),
        ("number", "NUMERIC", true),
        ("height", "NUMERIC", true),
        ("duration", "NUMERIC", true),
        ("reward", "NUMERIC", true),
        ("difficulty", "NUMERIC", true),
        ("effort_percent", "NUMERIC", true),
    ]);

    for (name, field_type, sortable) in indexes {
        block_index_cmd.arg(name).arg(field_type);

        if sortable {
            block_index_cmd.arg("SORTABLE");
        }
    }

    let res : redis::RedisResult<redis::Value> = block_index_cmd.query_async(&mut con_manager).await;
    match res {
        Ok(res) => {
            println!("Block index created: {:?}", res);
        }
        Err(err) => {
            println!("Failed to create block index: {:?}", err);
        }
    }

    // redisAppendCommand(rc,
    //                "FT.CREATE " COIN_SYMBOL
    //                ":block_index ON HASH PREFIX 1 " COIN_SYMBOL
    //                ":block: SCHEMA worker TAG SORTABLE height "
    //                "NUMERIC SORTABLE difficulty NUMERIC SORTABLE reward"
    //                " NUMERIC SORTABLE chain TAG SORTABLE number "
    //                "NUMERIC SORTABLE effort_percent NUMERIC SORTABLE");

    HttpServer::new(move || {
        let cors = actix_cors::Cors::permissive();

        App::new()
            .service(
                web::scope("/verus")
                    .app_data(web::Data::new(SickApiData {
                        redis: con_manager.clone(),
                    }))
                    .service(web::scope("/pool").configure(pool::pool_route)),
            )
            .wrap(cors)
        // .wrap(middleware::Logger::default())
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
