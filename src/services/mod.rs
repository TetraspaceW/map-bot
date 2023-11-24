use rust_decimal::Decimal;
use serde::Serialize;

pub mod database;
pub mod geocoding;

#[derive(Debug, Serialize)]
pub enum Location {
    Coordinates(Coordinates),
    Flight(Flight),
    Journey(Journey),
}

#[derive(Debug, Serialize)]
pub struct Coordinates {
    lat: Decimal,
    lng: Decimal,
}

#[derive(Debug, Serialize)]
pub struct Flight {
    pub id: String,
}

#[derive(Debug, Serialize)]
pub struct Journey {
    flights: Vec<Flight>,
    destination: Coordinates,
}
