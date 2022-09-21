use serde::Serialize;

#[repr(C, packed)] // don't modify order
#[derive()]
pub struct BlockRaw {
    pub confirmations: i32,
    pub block_type: u8,
    pub reward: i64,
    pub time: i64,
    pub duration: i64,
    pub height: u32,
    pub number: u32,
    pub difficulty: f64,
    pub effort_percent: f64,
    pub chain: [u8; 8],
    pub solver: [u8; 34],
    pub worker: [u8; 16],
    pub hash: [u8; 64],
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]

pub struct Block {
    pub confirmations: i32,
    pub block_type: u8,
    pub reward: i64,
    pub time: i64,
    pub duration: i64,
    pub height: u32,
    pub number: u32,
    pub difficulty: f64,
    pub effort_percent: f64,
    pub chain: String,
    pub solver: String,
    pub worker: String,
    pub hash: String,
}

impl redis::FromRedisValue for Block {
    fn from_redis_value(value: &redis::Value) -> redis::RedisResult<Self> {
        let bytes: Vec<u8> = redis::from_redis_value(&value)?;
        // println!(
        //     "bytes {}, size{}",
        //     bytes.len(),
        //     std::mem::size_of::<BlockRaw>()
        // );
        // if bytes.len() == std::mem::size_of::<BlockRaw>() {
            let block_res = parse_block(&bytes);

            match block_res {
                Ok(block) => Ok(block),
                Err(err) => {
                    Err(redis::RedisError::from((
                        redis::ErrorKind::TypeError,
                        "Failed to parse block",
                        format!("parse error: {:?}", err),
                    )))
                    // eprintln!("Failed to parse block: {}", err);
                }
            }
        // } else {
        //     Err(redis::RedisError::from((
        //         redis::ErrorKind::TypeError,
        //         "Failed to deserialize block",
        //         format!("response was of wrong length"),
        //     )))
        //     // eprintln!("Failed to serialize block, wrong size.");
        // }
    }
}

fn parse_block(bytes: &Vec<u8>) -> std::result::Result<Block, std::string::FromUtf8Error> {
    unsafe {
        let block_raw: BlockRaw = std::ptr::read(bytes.as_ptr() as *const _);
        let solver_str = String::from_utf8(block_raw.solver.to_vec())?;
        let worker_str = String::from_utf8(block_raw.worker.to_vec())?;
        let chain_str = String::from_utf8(block_raw.chain.to_vec())?;
        let hash_str = String::from_utf8(block_raw.hash.to_vec())?;

        let block = Block {
            confirmations: block_raw.confirmations,
            block_type: block_raw.block_type,
            reward: block_raw.reward,
            time: block_raw.time,
            duration: block_raw.duration,
            height: block_raw.height,
            number: block_raw.number,
            difficulty: block_raw.difficulty,
            effort_percent: block_raw.effort_percent,
            chain: chain_str.trim_matches(char::from(0)).to_string(),
            solver: solver_str,
            worker: worker_str.trim_matches(char::from(0)).to_string(),
            hash: hash_str,
        };
        // println!("BLOCK {:?}", block);
        // results.push(block);
        return Ok(block);
    }
}