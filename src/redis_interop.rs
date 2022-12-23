use serde::{Serialize, Serializer};
use serde_repr::*;
use std::fmt;

#[cxx::bridge]
pub mod ffi {

    #[derive(Debug)]
    enum Prefix {
        POW,
        ADDRESS,
        ADDRESS_ID_MAP,
        ALIAS,
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
        COUNT,

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
        HEIGHT,
    }

    #[derive(Debug)]
    pub enum BlockStatus {
        PENDING = 0b1,
        CONFIRMED = 0b10,
        ORPHANED = 0b100,
        PAID = 0b1000,
        
        PENDING_ORPHANED = 0b101
    }
}

fn x() -> String {
    format!("{:?}", ffi::Prefix::ACTIVE_IDS)
}
