use std::collections::HashSet;

use crate::services::{
    database::{LocationStorageService, SupabaseService},
    geocoding::{GeocodingService, GoogleMapsService},
};

use log::*;
use serenity::{
    framework::standard::{
        help_commands::with_embeds,
        macros::{command, group, help},
        Args, CommandGroup, CommandResult, HelpOptions,
    },
    model::{channel::Message, id::UserId},
    prelude::*,
};

#[help]
async fn help(
    context: &Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    let _ = with_embeds(context, msg, args, help_options, groups, owners).await?;
    Ok(())
}

#[group]
#[commands(location, clear)]
struct General;

#[command]
#[description("Reveal your location to amp's sight.")]
#[usage("[location]")]
#[example("London")]
async fn location(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let location = args.rest();

    let geocoding_service: GoogleMapsService = GeocodingService::new()?;
    let coords = geocoding_service.geocode(location.to_string()).await?;

    let author_id = msg.author.id.0.to_string();
    let author_name = msg.author.name.to_string();

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
#[description = "Obscure your location from amp's sight."]
#[usage("")]
async fn clear(_: &Context, msg: &Message) -> CommandResult {
    let author_id = format!("{}", msg.author.id.0);

    let storage_service: SupabaseService = LocationStorageService::new()?;
    storage_service.delete_location(author_id).await?;

    Ok(())
}
