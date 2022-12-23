extern crate redis;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct TableRes<T> {
    pub total: u64,
    pub entries: Vec<T>,
}


impl<T: redis::FromRedisValue> redis::FromRedisValue for TableRes<T> {
    fn from_redis_value(value: &redis::Value) -> redis::RedisResult<Self> {
        let items: Vec<redis::Value> = redis::from_redis_value(value)?;
        if items.len() < 2 {
            return Ok(TableRes {
                entries: vec![],
                total: 0,
            });
        }
        let total_res_count: u64 = redis::from_redis_value(items.first().unwrap())?;

        let item_arr: Vec<redis::Value> = redis::from_redis_value(&items[1])?;
        let mut results: Vec<T> = Vec::new();

        for item in item_arr {
            match redis::from_redis_value(&item){
                Ok(res) => results.push(res),
                Err(err) => {
                    eprintln!("Failed to parse item {:?}, err: {}", item, err);
                }
            }
        }
        Ok(TableRes {
            entries: results,
            total: total_res_count,
        })
    }
}