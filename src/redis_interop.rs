use serde::{Serialize, Serializer};
use std::fmt;
use serde_repr::*;

#[cxx::bridge]
pub mod ffi {
    
    #[derive(Debug)]
    enum Prefix {
        POW,
        PAYOUT,
        PAYOUT_FEELESS,
        PAYOUTS,
        ADDRESS,
        ADDRESS_ID_MAP,
        ALIAS,
        PAYOUT_THRESHOLD,
        IDENTITY,
        ROUND,
        EFFORT,
        WORKER_COUNT,
        MINER_COUNT,
        TOTAL_EFFORT,
        ESTIMATED_EFFORT,
        START_TIME,

        MATURE_BALANCE,
        IMMATURE_BALANCE,
        MATURE,
        IMMATURE,
        REWARD,

        HASHRATE,
        SHARES,

        BLOCK,
        MINED_BLOCK,

        NETWORK,
        POOL,

        AVERAGE,
        VALID,
        INVALID,
        STALE,

        EFFORT_PERCENT,

        SOLVER,
        INDEX,
        DURATION,
        DIFFICULTY,
        ROUND_EFFORT,

        PAYEES,
        FEE_PAYEES,
        PENDING_AMOUNT,
        PENDING_AMOUNT_FEE,
        PENDING,
        FEELESS,
        MINER,
        WORKER,
        TYPE,
        NUMBER,
        CHAIN,
        ACTIVE_IDS,
        STATS,
        COMPACT,
    }

    #[derive(Debug)]
    pub enum BlockType {
        POW = 0b1,
        PAYMENT = 0b01,
    }

    //#[repr(C)] // don't modify order
    #[derive(Debug, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct BlockSubmission {
        pub confirmations: i32,
        pub block_type: u8,
        pub chain: u8,
        pub reward: u64,
        pub time_ms: u64,
        pub duration_ms: u64,
        pub height: u32,
        pub number: u32,
        pub difficulty: f64,
        pub effort_percent: f64,

        #[serde(skip_serializing)]
        pub miner_id: u32,
        #[serde(skip_serializing)]
        pub worker_id: u32,
        #[serde(skip_serializing)]
        pub hash_bin: [u8; 32],
    }
}


fn x() -> String {
    format!("{:?}", ffi::Prefix::ACTIVE_IDS)
}