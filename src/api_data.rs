#[derive(Clone)]
pub struct SickApiData {
    pub redis: redis::aio::ConnectionManager,
}