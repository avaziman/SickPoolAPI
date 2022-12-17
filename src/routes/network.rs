use super::history;
use actix_web::web;

pub fn network_route(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/history").configure(history::network_history_route));
}