use std::{collections::HashMap, error::Error, sync::Arc};

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

type MapBotError = Box<dyn Error + Send + Sync>;

#[derive(Debug, Serialize)]
struct Location {
    lat: Decimal,
    lng: Decimal,
}

#[async_trait]
trait GeocodingService {
    fn new() -> Result<Self, MapBotError>
    where
        Self: Sized;
    async fn geocode(&self, location: String) -> Result<Location, MapBotError>;
}

struct GoogleMapsService {
    client: GoogleMapsClient,
}

#[async_trait]
impl GeocodingService for GoogleMapsService {
    fn new() -> Result<Self, MapBotError> {
        let service = Ok(GoogleMapsService {
            client: GoogleMapsClient::new(&dotenv::var("GOOGLE_MAPS_TOKEN")?),
        });
        trace!("Successfully initialised Google Maps client.");
        service
    }

    async fn geocode(&self, location: String) -> Result<Location, MapBotError> {
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
    trace!("Logger init with level TRACE.");

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
    trace!("Received location command.");
    let location = args.rest();
    trace!("Parsed args from command.");

    let geocoding_service: GoogleMapsService = GeocodingService::new().unwrap();
    let coords = geocoding_service.geocode(location.to_string()).await?;

    if let Err(why) = msg
        .channel_id
        .say(&ctx.http, format!("Location {:?} received.", coords))
        .await
    {
        warn!("Error sending message: {:?}", why);
    }

    let author_id = msg.author.id.0;

    let supabase_token = dotenv::var("SUPABASE_TOKEN")?;
    let client = Postgrest::new(&dotenv::var("SUPABASE_ENDPOINT")?)
        .insert_header("apikey", format!("{}", supabase_token));

    let raw_resp = client
        .from("location")
        .auth(&supabase_token)
        .eq("user_id", format!("{}", author_id))
        .select("id")
        .execute()
        .await?
        .text()
        .await?;

    let response: Value = serde_json::from_str(&raw_resp)?;
    let array = response.as_array().unwrap();
    if array.is_empty() {
        let json = json!({
            "user_id": author_id,
            "location": coords,
            "user_name": msg.author.name
        })
        .to_string();

        client
            .from("location")
            .auth(&supabase_token)
            .insert(json)
            .execute()
            .await?;
    } else {
        let json = json!({"location": coords, "user_name": msg.author.name}).to_string();
        client
            .from("location")
            .auth(&supabase_token)
            .eq("user_id", format!("{}", author_id))
            .update(json)
            .execute()
            .await?;
    }

    Ok(())
}

#[command]
async fn clear(_: &Context, msg: &Message) -> CommandResult {
    let supabase_token = dotenv::var("SUPABASE_TOKEN")?;
    let client = Postgrest::new(&dotenv::var("SUPABASE_ENDPOINT")?)
        .insert_header("apikey", format!("{}", supabase_token));
    let author_id = format!("{}", msg.author.id.0);

    client
        .from("location")
        .auth(supabase_token)
        .eq("user_id", author_id)
        .delete()
        .execute()
        .await?;

    Ok(())
}
