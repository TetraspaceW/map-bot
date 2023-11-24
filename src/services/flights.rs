#[async_trait]
pub trait FlightService {
    fn new() -> Result<Self, GenericError>
    where
        Self: Sized;
    async fn verify_flight_existence(&self, location: String) -> Result<Location, GenericError>;
}
