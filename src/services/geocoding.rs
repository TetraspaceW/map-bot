use super::Location;

use crate::{GenericError, MapBotError};

use async_trait::async_trait;
use google_maps::GoogleMapsClient;
use log::*;

#[async_trait]
pub trait GeocodingService {
    fn new() -> Result<Self, GenericError>
    where
        Self: Sized;
    async fn geocode(&self, location: String) -> Result<Location, GenericError>;
}

pub struct GoogleMapsService {
    client: GoogleMapsClient,
}

#[async_trait]
impl GeocodingService for GoogleMapsService {
    fn new() -> Result<Self, GenericError> {
        Ok(GoogleMapsService {
            client: GoogleMapsClient::new(&dotenv::var("GOOGLE_MAPS_TOKEN")?),
        })
    }

    async fn geocode(&self, location: String) -> Result<Location, GenericError> {
        let response = self
            .client
            .geocoding()
            .with_address(&location)
            .execute()
            .await?;
        let coordinates = &response
            .results
            .first()
            .ok_or(MapBotError::LocationNotFound())?
            .geometry
            .location;
        trace!("Received coordinates from Google Maps geocoding API.");
        Ok(Location {
            lat: coordinates.lat,
            lng: coordinates.lng,
        })
    }
}
