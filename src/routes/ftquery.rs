// https://raw.githubusercontent.com/Equilibris/twitter-clone/a703e955e38afc6bb23277dc7f6fd71dce4f10db/backend/src/db/ftquery.rs
use redis::FromRedisValue;
pub struct FtQuery<T: redis::FromRedisValue> {
    container: Vec<(String, T)>,
}

impl<T: redis::FromRedisValue> redis::FromRedisValue for FtQuery<T> {
    fn from_redis_value(v: &redis::Value) -> redis::RedisResult<Self> {
        let mut bulk = match v {
            redis::Value::Bulk(v) => v.into_iter(),
            _ => {
                return Err(redis::RedisError::from((
                    redis::ErrorKind::TypeError,
                    "Type error",
                )))
            }
        };

        let count = match bulk.next() {
            Some(redis::Value::Int(v)) => *v,
            _ => {
                return Err(redis::RedisError::from((
                    redis::ErrorKind::TypeError,
                    "Type error",
                )))
            }
        };

        let mut result = Vec::with_capacity(count.try_into().unwrap());

        while let Some(v) = bulk.next() {
            let key = <String as FromRedisValue>::from_redis_value(&v)?;
            let value = match bulk.next() {
                Some(redis::Value::Bulk(v)) => T::from_redis_value(v.last().unwrap())?,
                _ => {
                    return Err(redis::RedisError::from((
                        redis::ErrorKind::TypeError,
                        "Type error",
                    )))
                }
            };
            result.push((key, value));
        }

        Ok(Self { container: result })
    }
}
impl<T: FromRedisValue> FtQuery<T> {
    pub fn values(self) -> Vec<T> {
        self.container.into_iter().map(|(_, v)| v).collect()
    }
}
