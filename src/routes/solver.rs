use serde::Serialize;
use actix_web::web;

#[derive(Debug, Serialize)]
pub struct Solver {
    pub address: String,
    pub hashrate: f64,
    pub worker_count: u32,
    pub balance: f64,
    pub joined: i64,
}

pub fn solver_route(cfg: &mut web::ServiceConfig){

}