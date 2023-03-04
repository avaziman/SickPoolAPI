use std::fmt::UpperHex;

use crate::redis_interop::ffi;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
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

    #[serde(rename(serialize = "hash"))]
    pub hash_hex: String,
    pub solver: String,
}