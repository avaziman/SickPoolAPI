use serde::Serialize;

#[repr(C, packed)] // don't modify order
#[derive()]
pub struct FinishedPaymentRaw {
    pub id: u32,
    pub hash_hex: [u8; 64],
    pub paid_amount: i64,
    pub time_ms: i64,
    pub tx_fee: i64,
    pub payee_amount: u32,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinishedPayment {
    pub id: u32,
    pub tx_id: String,
    pub paid_amount: i64,
    pub time_ms: i64,
    pub tx_fee: i64,
    pub payee_amount: u32,
}

impl redis::FromRedisValue for FinishedPayment {
    fn from_redis_value(value: &redis::Value) -> redis::RedisResult<Self> {
        let bytes: Vec<u8> = redis::from_redis_value(&value)?;
        // println!(
        //     "bytes {}, size{}",
        //     bytes.len(),
        //     std::mem::size_of::<FinishedPaymentRaw>()
        // );
        // if bytes.len() == std::mem::size_of::<paymentRaw>() {
        let payment_res = parse_payment(&bytes);

        match payment_res {
            Ok(payment) => Ok(payment),
            Err(err) => {
                Err(redis::RedisError::from((
                    redis::ErrorKind::TypeError,
                    "Failed to parse payment",
                    format!("parse error: {:?}", err),
                )))
                // eprintln!("Failed to parse payment: {}", err);
            }
        }
        // } else {
        //     Err(redis::RedisError::from((
        //         redis::ErrorKind::TypeError,
        //         "Failed to deserialize payment",
        //         format!("response was of wrong length"),
        //     )))
        //     // eprintln!("Failed to serialize payment, wrong size.");
        // }
    }
}

fn parse_payment(
    bytes: &Vec<u8>,
) -> std::result::Result<FinishedPayment, std::string::FromUtf8Error> {
    unsafe {
        let payment_raw: FinishedPaymentRaw = std::ptr::read(bytes.as_ptr() as *const _);
        let hash_str = String::from_utf8(payment_raw.hash_hex.to_vec())?;

        let payment = FinishedPayment {
            id: (payment_raw.id),
            tx_id: (hash_str),
            paid_amount: (payment_raw.paid_amount),
            time_ms: (payment_raw.time_ms),
            tx_fee: (payment_raw.tx_fee),
            payee_amount: (payment_raw.payee_amount),
        };
        // println!("payment {:?}", payment);
        // results.push(payment);
        return Ok(payment);
    }
}
