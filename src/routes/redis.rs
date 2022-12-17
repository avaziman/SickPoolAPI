use std::time::SystemTime;

use redis::aio::ConnectionManager;
use redis_ts::{
    AsyncTsCommands, TsAggregationOptions, TsAggregationType, TsBucketTimestamp, TsFilterOptions,
    TsMrange, TsRange,
};

pub fn key_format<const N: usize>(strs: &[&str; N]) -> String {
    let mut res: String = String::new();
    for item in strs.iter() {
        res += item;
        res += ":";
    }
    res.pop();

    res
}

pub async fn get_ts_points(
    con: &mut ConnectionManager,
    key: &String,
    interval: u64,
    retention: u64,
) -> Vec<(u64, f64)> {
    let curtime = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
    let last_timestamp = curtime - curtime % interval;
    let first_timestamp = last_timestamp - retention;

    let points_amount: u64 = retention / interval;

    let mut tms: TsRange<u64, f64> = match con
        .ts_range(key, first_timestamp * 1000, last_timestamp * 1000, Some(points_amount), None::<TsAggregationOptions>)
        .await
    {
        Ok(res) => res,
        Err(err) => {
            eprintln!("range query error: {}", err);
            return Vec::<(u64, f64)>::new();
        }
    };

    if tms.values.len() == points_amount as usize {
        tms.values
    } else {
        fill_gaps(&mut tms.values, first_timestamp, interval, points_amount)
    }

}

fn fill_gaps(points: &mut Vec<(u64, f64)>, first_timestamp: u64, interval: u64, points_amount: u64) -> Vec<(u64, f64)>{
    let mut res: Vec<(u64, f64)> = Vec::new();
    let mut j = 0;

    res.reserve(points_amount as usize);

    for i in 0..points_amount {
        let timestamp = first_timestamp + interval * i;
        let val: f64 = if points.len() > j && points[j].0 / 1000 == timestamp {
            j += 1;
            points[j -1].1
        } else {
            0.0
        };

        res.push((timestamp, val));
    }
    res
}
