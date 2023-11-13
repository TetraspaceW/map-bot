use std::{collections::HashMap, sync::Arc};

use log::trace;

use google_maps::GoogleMapsClient;

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

use tokio::sync::Mutex;

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
        println!("{} is connected!", ready.user.name);
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
        println!("Client error: {:?}", why);
    }
}

#[command]
async fn location(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    trace!("Received location command.");
    let location = args.single::<String>()?;
    trace!("Parsed args from command.");
    let google_maps_client = GoogleMapsClient::new(&dotenv::var("GOOGLE_MAPS_TOKEN")?);
    trace!("Read Google Maps token from env.");

    let location = google_maps_client
        .geocoding()
        .with_address(&location)
        .execute()
        .await?;
    trace!("Executed geocoding request.");

    if let Err(why) = msg
        .channel_id
        .say(
            &ctx.http,
            format!(
                "Location {:?} received.",
                location.results.first().unwrap().geometry.location
            ),
        )
        .await
    {
        println!("Error sending message: {:?}", why);
    }

    Ok(())
}

#[command]
async fn clear(ctx: &Context, msg: &Message) -> CommandResult {
    Ok(())
}
