extern crate redis;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct TableRes<T: redis::FromRedisValue> {
    pub total: i64,
    pub result: Vec<T>,
}


impl<T: redis::FromRedisValue> redis::FromRedisValue for TableRes<T> {
    fn from_redis_value(value: &redis::Value) -> redis::RedisResult<Self> {
        let items: Vec<redis::Value> = redis::from_redis_value(value)?;
        if items.len() < 2 {
            return Ok(TableRes {
                result: vec![],
                total: 0,
            });
        }
        let total_res_count: i64 = redis::from_redis_value(items.first().unwrap())?;

        let block_arr: Vec<redis::Value> = redis::from_redis_value(&items[1])?;
        let mut results: Vec<T> = Vec::new();

        for block in block_arr {
            let item: T = redis::from_redis_value(&block)?;
            results.push(item);
        }
        Ok(TableRes {
            result: results,
            total: total_res_count,
        })
    }
}