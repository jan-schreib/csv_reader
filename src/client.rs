use rust_decimal::Decimal;
use serde::{Serialize, Serializer};

use crate::transaction::Transaction;

#[derive(Serialize, Debug)]
pub struct Client {
    #[serde(rename(serialize = "client"))]
    pub client_id: u16,
    #[serde(rename(serialize = "available"))]
    #[serde(serialize_with = "float_precission")]
    pub available_funds: Decimal,
    #[serde(rename(serialize = "held"))]
    #[serde(serialize_with = "float_precission")]
    pub held_funds: Decimal,
    #[serde(rename(serialize = "total"))]
    #[serde(serialize_with = "float_precission")]
    pub total_funds: Decimal,
    #[serde(rename(serialize = "locked"))]
    pub locked: bool,
    #[serde(skip_serializing)]
    pub transactions: Vec<Transaction>,
}

fn float_precission<S>(x: &Decimal, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(&format!("{:.4}", x))
}
