use async_trait::async_trait;
use postgrest::Postgrest;
use serde_json::{json, Value};

use crate::{GenericError, MapBotError};

use super::Location;

#[async_trait]
pub trait LocationStorageService {
    fn new() -> Result<Self, GenericError>
    where
        Self: Sized;
    async fn get_location(&self, user_id: &String) -> Result<String, GenericError>;
    async fn save_location(
        &self,
        user_id: &String,
        location: &Location,
        user_name: &String,
    ) -> Result<(), GenericError>;
    async fn delete_location(&self, user_id: String) -> Result<(), GenericError>;
}

pub struct SupabaseService {
    client: Postgrest,
    supabase_token: String,
}

#[async_trait]
impl LocationStorageService for SupabaseService {
    fn new() -> Result<Self, GenericError> {
        let supabase_token = dotenv::var("SUPABASE_TOKEN")?;
        let client = Postgrest::new(&dotenv::var("SUPABASE_ENDPOINT")?)
            .insert_header("apikey", format!("{}", supabase_token));
        Ok(SupabaseService {
            client,
            supabase_token,
        })
    }

    async fn get_location(&self, user_id: &String) -> Result<String, GenericError> {
        let raw_resp = self
            .client
            .from("location")
            .auth(&self.supabase_token)
            .eq("user_id", &user_id)
            .select("id")
            .execute()
            .await?
            .text()
            .await?;

        let response: Value = serde_json::from_str(&raw_resp)?;
        let result = response
            .as_array()
            .ok_or(MapBotError::UserNotFound())?
            .first()
            .ok_or(MapBotError::UserNotFound())?
            .to_string();
        Ok(result)
    }

    async fn save_location(
        &self,
        user_id: &String,
        coords: &Location,
        user_name: &String,
    ) -> Result<(), GenericError> {
        if let Ok(_) = self.get_location(&user_id).await {
            let json = json!({"location": coords, "user_name": user_name}).to_string();
            self.client
                .from("location")
                .auth(&self.supabase_token)
                .eq("user_id", user_id)
                .update(json)
                .execute()
                .await?;
        } else {
            let json = json!({
                "user_id": user_id,
                "location": coords,
                "user_name": user_name
            })
            .to_string();

            self.client
                .from("location")
                .auth(&self.supabase_token)
                .insert(json)
                .execute()
                .await?;
        }

        Ok(())
    }

    async fn delete_location(&self, user_id: String) -> Result<(), GenericError> {
        self.client
            .from("location")
            .auth(&self.supabase_token)
            .eq("user_id", user_id)
            .delete()
            .execute()
            .await?;

        Ok(())
    }
}
