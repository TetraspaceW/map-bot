use std::{collections::HashMap, error::Error, sync::Arc};

use derive_more::Display;
use thiserror::Error;

use log::{debug, error, trace, warn};

use google_maps::GoogleMapsClient;

use postgrest::Postgrest;
use serde_json::{json, Value};
use serenity::{
    async_trait,
    client::bridge::gateway::ShardManager,
    framework::{
        standard::{
            macros::{command, group},
            Args, CommandResult,
        },
        StandardFramework,
    },
    http::Http,
    model::prelude::{Message, Ready},
    prelude::{Client, Context, EventHandler, GatewayIntents, TypeMapKey},
};

use serde::Serialize;
use tokio::sync::Mutex;

use rust_decimal::Decimal;

type GenericError = Box<dyn Error + Send + Sync>;

#[derive(Error, Debug, Display)]
enum MapBotError {
    UserNotFound(),
}

#[derive(Debug, Serialize)]
struct Location {
    lat: Decimal,
    lng: Decimal,
}

#[async_trait]
trait GeocodingService {
    fn new() -> Result<Self, GenericError>
    where
        Self: Sized;
    async fn geocode(&self, location: String) -> Result<Location, GenericError>;
}

struct GoogleMapsService {
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
        let coordinates = &response.results.first().unwrap().geometry.location;
        trace!("Received coordinates from Google Maps geocoding API.");
        Ok(Location {
            lat: coordinates.lat,
            lng: coordinates.lng,
        })
    }
}

#[async_trait]
trait LocationStorageService {
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

struct SupabaseService {
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
        let result = response.as_array().unwrap();
        if let Some(location) = result.first() {
            Ok(location.to_string())
        } else {
            Err(MapBotError::UserNotFound().into())
        }
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

#[group]
#[commands(location, clear)]
struct General;

struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

struct CommandCounter;
impl TypeMapKey for CommandCounter {
    type Value = HashMap<String, u64>;
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        debug!("{} is connected!", ready.user.name);
    }
}

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter_module("map_bot", log::LevelFilter::Trace)
        .init();

    let token = dotenv::var("DISCORD_TOKEN").expect("DISCORD_TOKEN not set.");
    let http = Http::new(&token);
    let bot_id = http
        .get_current_user()
        .await
        .expect("Current user not found")
        .id;

    let framework = StandardFramework::new()
        .configure(|c| {
            c.with_whitespace(true)
                .on_mention(Some(bot_id))
                .prefix("!tetramap")
        })
        .group(&GENERAL_GROUP);

    let intents = GatewayIntents::all();
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .framework(framework)
        .type_map_insert::<CommandCounter>(HashMap::default())
        .await
        .expect("Error creating client.");

    {
        let mut data = client.data.write().await;
        data.insert::<ShardManagerContainer>(Arc::clone(&client.shard_manager))
    }

    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    }
}

#[command]
async fn location(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let location = args.rest();

    let geocoding_service: GoogleMapsService = GeocodingService::new()?;
    let coords = geocoding_service.geocode(location.to_string()).await?;

    let author_id = format!("{}", msg.author.id.0);
    let author_name = format!("{}", msg.author.name);

    let storage_service: SupabaseService = LocationStorageService::new()?;
    storage_service
        .save_location(&author_id, &coords, &author_name)
        .await?;

    if let Err(why) = msg
        .channel_id
        .say(&ctx.http, format!("Location {:?} received.", coords))
        .await
    {
        warn!("Error sending message: {:?}", why);
    }

    Ok(())
}

#[command]
async fn clear(_: &Context, msg: &Message) -> CommandResult {
    let author_id = format!("{}", msg.author.id.0);

    let storage_service: SupabaseService = LocationStorageService::new()?;
    storage_service.delete_location(author_id).await?;

    Ok(())
}
