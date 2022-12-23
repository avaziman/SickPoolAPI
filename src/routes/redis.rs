use std::time::SystemTime;

use redis::aio::ConnectionManager;
use redis_ts::{
    AsyncTsCommands, TsAggregationType, TsBucketTimestamp, TsFilterOptions,
    TsMrange, TsRange,
};

use super::history::TimeSeriesInterval;

pub fn key_format<const N: usize>(strs: &[&str; N]) -> String {
    let mut res: String = String::new();
    for item in strs.iter() {
        res += item;
        res += ":";
    }
    res.pop();

    res
}

pub fn get_range_params(interval: &TimeSeriesInterval) -> (u64, u64, u64){
    let curtime = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();

    let last_timestamp = curtime - curtime % interval.interval;
    let first_timestamp = last_timestamp + interval.interval - interval.retention;
    let points_amount: u64 = interval.retention / interval.interval;

    (first_timestamp, last_timestamp, points_amount)
}

pub async fn get_ts_points(
    con: &mut ConnectionManager,
    key: &String,
    interval: &TimeSeriesInterval
) -> Option<Vec<(u64, f64)>> {
    let (first_timestamp, last_timestamp, points_amount) = get_range_params(interval);

    let tms: TsRange<u64, f64> = match con
        .ts_range(key, first_timestamp * 1000, last_timestamp * 1000, Some(points_amount), None::<TsAggregationType>)
        .await
    {
        Ok(res) => res,
        Err(err) => {
            eprintln!("range query error: {}", err);
            return None;
        }
    };

    Some(fill_gaps(tms.values, first_timestamp, interval.interval, points_amount))
}

pub fn fill_gaps(points: Vec<(u64, f64)>, first_timestamp: u64, interval: u64, points_amount: u64) -> Vec<(u64, f64)>{
    if points.len() == points_amount as usize {
        return points;
    }
    

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
