use std::time::SystemTime;

use redis::aio::ConnectionManager;
use redis_ts::{
    AsyncTsCommands, TsAggregationType, TsBucketTimestamp, TsFilterOptions, TsMrange, TsRange, TsRangeQuery,
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

pub fn get_range_params(interval: &TimeSeriesInterval) -> (u64, u64) {
    let curtime = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let last_timestamp = curtime - curtime % interval.interval;
    let first_timestamp = last_timestamp + interval.interval - interval.retention;

    (first_timestamp * 1000, last_timestamp * 1000)
}

pub async fn get_ts_points(
    con: &mut ConnectionManager,
    key: &String,
    interval: &TimeSeriesInterval,
) -> Option<Vec<(u64, f64)>> {
    let (first_timestamp, last_timestamp) = get_range_params(interval);

    let tms: TsRange<u64, f64> = match con
        .ts_range(
            key,
            TsRangeQuery::default()
                .from(first_timestamp)
                .to(last_timestamp)
                .count(interval.amount)
                .aggregation_type(TsAggregationType::Sum(interval.interval * 1000))
                .empty(true),
        )
        .await
    {
        Ok(res) => res,
        Err(err) => {
            eprintln!("range query error: {}", err);
            return None;
        }
    };

    Some(fill_gaps(
        tms.values,
        first_timestamp,
        interval.interval,
        interval.amount,
    ))
}

// USING empty gaps can onl be after the last ts entry
pub fn fill_gaps(
    mut points: Vec<(u64, f64)>,
    first_timestamp: u64,
    interval: u64,
    points_amount: u64,
) -> Vec<(u64, f64)> {

    let ineterval_ms = interval * 1000;

    points.resize_with(points_amount as usize, || {(0, 0.0)});

    points
}
