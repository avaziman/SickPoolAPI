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
pub struct Payout {
    pub id: u32,
    pub txid: String,
    pub payee_amount: u32,
    pub paid_amount: u64,
    pub tx_fee: u64,
    pub time_ms: u64,
}