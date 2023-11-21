mod commands;
mod services;

use commands::{GENERAL_GROUP, HELP};

use derive_more::Display;
use log::*;
use serenity::{
    async_trait, framework::StandardFramework, http::Http, model::prelude::*, prelude::*,
};
use std::error::Error;
use thiserror::Error;

type GenericError = Box<dyn Error + Send + Sync>;

#[derive(Error, Debug, Display)]
enum MapBotError {
    UserNotFound(),
    LocationNotFound(),
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
