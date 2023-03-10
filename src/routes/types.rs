use std::{collections::HashMap, option, time::SystemTime};

use actix_web::{
    body::BoxBody,
    http::{
        header::{ContentType, HeaderName, HeaderValue},
        StatusCode,
    },
    HttpRequest, HttpResponse, Responder,
};
use serde::{Serialize, Deserialize};
use serde_json::{json, Value};
use ts_rs::TS;

#[derive(Deserialize/*  */)]
pub struct CoinQuery {
    pub coin: String,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct SickResult<T: Serialize> {
    #[serde(skip)]
    pub status_code: StatusCode,
    pub error: Option<String>,
    pub result: Option<T>,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RoundOverview {
    pub effort_percent: f64,
    pub start_time: u64,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BlockOverview {
    pub current_round: RoundOverview,
    pub height: u32,
    pub orphans: u32,
    pub mined: u32,
    pub average_effort: f64,
    pub average_duration: f64,
    pub mined24h: u32,
    pub difficulty: f64,
    pub confirmations: u32,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BlockSubmission {
    pub id: u32,
    pub status: i8,
    pub chain: u8,
    pub reward: u64,
    pub time_ms: u64,
    pub duration_ms: u64,
    pub height: u32,
    pub difficulty: f64,
    pub effort_percent: f64,

    // #[serde(rename(serialize = "hash"))]
    // hex
    pub hash: String,
    pub solver: String,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct CachedSickResult<T: Serialize> {
    pub expire_seconds: u64,
    pub sick_result: SickResult<T>,
}


#[derive(Serialize, TS)]
#[ts(export)]
pub struct TableRes<T> {
    pub total: usize,
    pub entries: Vec<T>,
}


#[derive(Serialize, TS)]
#[ts(export)]
pub struct TimestampInfo {
    pub start: u64,
    pub interval: u64,
    pub amount: u64,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct HistoryResult<T: Serialize + ValuesTrait + TS> {
    pub timestamps: TimestampInfo,
    pub values: T,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PoolOverview {
    pub pool_hashrate: f64,
    pub network_hashrate: f64,
    pub miner_count: u32,
    pub worker_count: u32,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PayoutOverview {
    pub next_payout: i64,
    pub total_paid: u64,
    pub total_payouts: u32,
    pub scheme: String,
    pub fee: f64,
    pub minimum_threshold: u64,
}

#[derive(Serialize, Clone, Default ,TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MinerStats {
    pub average_hashrate: Vec<f64>,
    pub current_hashrate: Vec<f64>,
    pub invalid_shares: Vec<u32>,
    pub stale_shares: Vec<u32>,
    pub valid_shares: Vec<u32>,
}

#[derive(Deserialize, Clone, Default ,TS)]
#[ts(export)]
pub struct TableQuerySort {
    #[serde(skip_deserializing)]
    pub coin: String,
    pub page: u32,
    pub limit: u8,
    pub sortby: String,
    pub sortdir: String,
    // chains: Vec<String>,
}
#[derive(Deserialize, Clone, Default ,TS)]
#[ts(export)]
pub struct TableQuery {
    #[serde(skip_deserializing)]
    pub coin: String,
    pub page: u32,
    pub limit: u8,
}

#[derive(Serialize, Clone, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WorkerStatsEntry {
    pub worker: String,
    pub average_hashrate: f64,
    pub current_hashrate: f64,
    pub invalid_shares: u32,
    pub stale_shares: u32,
    pub time: u64,
    pub valid_shares: u32,
}
pub trait ValueTypeTrait {}
impl ValueTypeTrait for u64 {}
impl ValueTypeTrait for f64 {}

pub trait ValuesTrait {}
impl<ValueTypeTrait> ValuesTrait for HashMap<String, Vec<ValueTypeTrait>> {}
// single value queries
impl<ValueTypeTrait> ValuesTrait for Vec<ValueTypeTrait> {}
impl ValuesTrait for MinerStats {}
impl ValuesTrait for () {}

impl<T: Serialize> Responder for SickResult<T> {
    type Body = BoxBody;

    fn respond_to(self, req: &HttpRequest) -> HttpResponse {
        HttpResponse::build(self.status_code)
            .content_type(ContentType::json())
            .body(json!(self).to_string())
    }
}

impl<T: Serialize> Responder for CachedSickResult<T> {
    type Body = BoxBody;

    fn respond_to(self, req: &HttpRequest) -> HttpResponse {
        let mut res = self.sick_result.respond_to(req);

        res.headers_mut().insert(
            HeaderName::from_lowercase(b"cache-control").unwrap(),
            HeaderValue::from_str(&format!("max-age={}", self.expire_seconds)).unwrap(),
        );
        res
    }
}

impl<T: Serialize + ValuesTrait + TS> Responder for HistoryResult<T> {
    type Body = BoxBody;

    fn respond_to(self, req: &HttpRequest) -> HttpResponse<Self::Body> {
        let curtime = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let next = self.timestamps.start + self.timestamps.amount as u64 * self.timestamps.interval + 1;
        let cache_for = next - curtime;

        CachedSickResult {
            expire_seconds: cache_for,
            sick_result: SickResult {
                status_code: StatusCode::OK,
                error: None,
                result: Some(self),
            },
        }
        .respond_to(req)
    }
}
