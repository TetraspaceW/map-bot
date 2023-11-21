use rust_decimal::Decimal;
use serde::Serialize;

pub mod database;
pub mod geocoding;

#[derive(Debug, Serialize)]
pub struct Location {
    lat: Decimal,
    lng: Decimal,
}
