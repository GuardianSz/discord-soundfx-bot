use log::warn;

use crate::{cmds::autocomplete_favorite, models::sound::SoundCtx, Context, Error};

#[poise::command(slash_command, rename = "favorites", guild_only = true)]
pub async fn favorites(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Add a sound as a favorite
#[poise::command(
    slash_command,
    rename = "add",
    category = "Favorites",
    guild_only = true
)]
pub async fn add_favorite(
    ctx: Context<'_>,
    #[description = "Name or ID of sound to favorite"] name: String,
) -> Result<(), Error> {
    let sounds = ctx
        .data()
        .search_for_sound(&name, ctx.guild_id().unwrap(), ctx.author().id, true)
        .await;

    match sounds {
        Ok(sounds) => {
            let sound = &sounds[0];

            sound
                .add_favorite(ctx.author().id, &ctx.data().database)
                .await?;
            ctx.say(format!(
                "Sound {} (ID {}) added to favorites.",
                sound.name, sound.id
            ))
            .await?;

            Ok(())
        }

        Err(e) => {
            warn!("Couldn't fetch sounds: {:?}", e);

            ctx.say("Failed to find sound.").await?;

            Ok(())
        }
    }
}

/// Remove a sound from your favorites
#[poise::command(
    slash_command,
    rename = "remove",
    category = "Favorites",
    guild_only = true
)]
pub async fn remove_favorite(
    ctx: Context<'_>,
    #[description = "Name or ID of sound to favorite"]
    #[autocomplete = "autocomplete_favorite"]
    name: String,
) -> Result<(), Error> {
    let sounds = ctx
        .data()
        .search_for_sound(&name, ctx.guild_id().unwrap(), ctx.author().id, true)
        .await;

    match sounds {
        Ok(sounds) => {
            let sound = &sounds[0];

            sound
                .remove_favorite(ctx.author().id, &ctx.data().database)
                .await?;
            ctx.say(format!(
                "Sound {} (ID {}) removed from favorites.",
                sound.name, sound.id
            ))
            .await?;

            Ok(())
        }

        Err(e) => {
            warn!("Couldn't fetch sounds: {:?}", e);

            ctx.say("Failed to find sound.").await?;

            Ok(())
        }
    }
}
