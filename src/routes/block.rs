use std::fmt::UpperHex;

use crate::redis_interop::ffi;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Block {
    #[serde(flatten)]
    pub raw: ffi::BlockSubmission,
    #[serde(rename = "hash")]
    pub hash_hex: String,
    pub solver: String
}

impl redis::FromRedisValue for Block {
    fn from_redis_value(value: &redis::Value) -> redis::RedisResult<Self> {
        let bytes: Vec<u8> = redis::from_redis_value(&value)?;
        
        if bytes.len() == std::mem::size_of::<ffi::BlockSubmission>() {
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
        } else {
            Err(redis::RedisError::from((
                redis::ErrorKind::TypeError,
                "Failed to deserialize block",
                format!("response was of wrong length"),
            )))
            // eprintln!("Failed to serialize block, wrong size.");
        }
    }
}
use hex;
fn parse_block(bytes: &Vec<u8>) -> std::result::Result<Block, std::string::FromUtf8Error> {
    unsafe {
        let block_raw: ffi::BlockSubmission = std::ptr::read(bytes.as_ptr() as *const _);

        let block = Block {
            hash_hex: hex::encode(block_raw.hash_bin),
            raw: block_raw,
            solver: String::from("SOLVRRRRRRRRRRRRRRRRRRRRR")
        };
        // println!("BLOCK {:?}", block);
        // results.push(block);
        return Ok(block);
    }
}