use std::fmt;

pub mod ffi {

    #[derive(Debug)]
    pub enum Prefix {
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

        PENDING_ORPHANED = 0b101,
    }
    #[derive(serde::Deserialize)]
    pub struct StatsConfig {
        pub hashrate_interval_seconds: u32,
        pub effort_interval_seconds: u32,
        pub average_hashrate_interval_seconds: u32,
        pub mined_blocks_interval: u32,
        pub diff_adjust_seconds: u32,
    }
}

fn x() -> String {
    format!("{:?}", ffi::Prefix::ACTIVE_IDS)
}
