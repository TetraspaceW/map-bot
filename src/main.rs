mod services;

use std::{collections::HashSet, error::Error};

use derive_more::Display;
use thiserror::Error;

use log::*;

use serenity::{
    async_trait,
    framework::{
        standard::{
            help_commands::with_embeds,
            macros::{command, group, help},
            Args, CommandGroup, CommandResult, HelpOptions,
        },
        StandardFramework,
    },
    http::Http,
    model::{
        id::UserId,
        prelude::{Message, Ready},
    },
    prelude::{Client, Context, EventHandler, GatewayIntents},
};

use crate::services::{
    database::{LocationStorageService, SupabaseService},
    geocoding::{GeocodingService, GoogleMapsService},
};

type GenericError = Box<dyn Error + Send + Sync>;

#[derive(Error, Debug, Display)]
enum MapBotError {
    UserNotFound(),
    LocationNotFound(),
}

#[group]
#[commands(location, clear)]
struct General;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        debug!("{} is connected!", ready.user.name);
    }
}

#[help]
async fn help(
    context: &Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    let _ = with_embeds(context, msg, args, &help_options, groups, owners).await?;
    Ok(())
}

#[command]
#[description("Add your location to the table.")]
#[usage("[location]")]
#[example("London")]
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
#[description = "Remove your location from the table."]
async fn clear(_: &Context, msg: &Message) -> CommandResult {
    let author_id = format!("{}", msg.author.id.0);

    let storage_service: SupabaseService = LocationStorageService::new()?;
    storage_service.delete_location(author_id).await?;

    Ok(())
}

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter_module("map_bot", log::LevelFilter::Info)
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
        .group(&GENERAL_GROUP)
        .help(&HELP);

    let intents = GatewayIntents::all();
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("Error creating client.");

    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    }
}
