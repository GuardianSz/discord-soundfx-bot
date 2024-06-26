use poise::{
    serenity_prelude::{GuildId, User},
    CreateReply,
};

use crate::{
    cmds::autocomplete_sound,
    models::{
        guild_data::{AllowGreet, CtxGuildData},
        join_sound::JoinSoundCtx,
        sound::SoundCtx,
    },
    Context, Error,
};

/// Change the bot's volume in this server
#[poise::command(slash_command, rename = "volume", guild_only = true)]
pub async fn change_volume(
    ctx: Context<'_>,
    #[description = "New volume as a percentage"] volume: Option<usize>,
) -> Result<(), Error> {
    let guild_data_opt = ctx.guild_data(ctx.guild_id().unwrap()).await;
    let guild_data = guild_data_opt.unwrap();

    if let Some(volume) = volume {
        guild_data.write().await.volume = volume as u8;

        guild_data.read().await.commit(&ctx.data().database).await?;

        ctx.say(format!("Volume changed to {}%", volume)).await?;
    } else {
        let read = guild_data.read().await;

        ctx.say(format!(
            "Current server volume: {vol}%. Change the volume with `/volume <new volume>`",
            vol = read.volume
        ))
        .await?;
    }

    Ok(())
}

/// Manage greet sounds
#[poise::command(slash_command, rename = "greet", guild_only = true)]
pub async fn greet_sound(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Manage greet sounds in this server
#[poise::command(slash_command, rename = "server")]
pub async fn guild_greet_sound(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Set a user's server-specific join sound
#[poise::command(slash_command, rename = "set")]
pub async fn set_guild_greet_sound(
    ctx: Context<'_>,
    #[description = "Name or ID of sound to set as join sound"]
    #[autocomplete = "autocomplete_sound"]
    name: String,
    #[description = "User to set join sound for"] user: User,
) -> Result<(), Error> {
    if user.id != ctx.author().id {
        let permissions = ctx.author_member().await.unwrap().permissions(&ctx.cache());

        if permissions.map_or(true, |p| !p.manage_guild()) {
            ctx.send(
                CreateReply::default()
                    .ephemeral(true)
                    .content("Only admins can change other user's greet sounds."),
            )
            .await?;

            return Ok(());
        }
    }

    let sound_vec = ctx
        .data()
        .search_for_sound(&name, ctx.guild_id().unwrap(), ctx.author().id, true)
        .await?;

    match sound_vec.first() {
        Some(sound) => {
            ctx.data()
                .update_join_sound(user.id, ctx.guild_id(), Some(sound.id))
                .await?;

            ctx.say(format!(
                "Greet sound has been set to {} (ID {})",
                sound.name, sound.id
            ))
            .await?;
        }

        None => {
            ctx.say("Could not find a sound by that name.").await?;
        }
    }

    Ok(())
}

/// Unset a user's server-specific join sound
#[poise::command(slash_command, rename = "unset", guild_only = true)]
pub async fn unset_guild_greet_sound(
    ctx: Context<'_>,
    #[description = "User to set join sound for"] user: User,
) -> Result<(), Error> {
    if user.id != ctx.author().id {
        let permissions = ctx.author_member().await.unwrap().permissions(&ctx.cache());

        if permissions.map_or(true, |p| !p.manage_guild()) {
            ctx.send(
                CreateReply::default()
                    .ephemeral(true)
                    .content("Only admins can change other user's greet sounds."),
            )
            .await?;

            return Ok(());
        }
    }

    ctx.data()
        .update_join_sound(user.id, ctx.guild_id(), None)
        .await?;

    ctx.say("Greet sound has been unset").await?;

    Ok(())
}

/// Manage your own greet sound
#[poise::command(slash_command, rename = "user")]
pub async fn user_greet_sound(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Set your global join sound
#[poise::command(slash_command, rename = "set")]
pub async fn set_user_greet_sound(
    ctx: Context<'_>,
    #[description = "Name or ID of sound to set as your join sound"]
    #[autocomplete = "autocomplete_sound"]
    name: String,
) -> Result<(), Error> {
    let sound_vec = ctx
        .data()
        .search_for_sound(&name, ctx.guild_id().unwrap(), ctx.author().id, true)
        .await?;

    match sound_vec.first() {
        Some(sound) => {
            ctx.data()
                .update_join_sound(ctx.author().id, None::<GuildId>, Some(sound.id))
                .await?;

            ctx.send(CreateReply::default().ephemeral(true).content(format!(
                "Greet sound has been set to {} (ID {})",
                sound.name, sound.id
            )))
            .await?;
        }

        None => {
            ctx.send(
                CreateReply::default()
                    .ephemeral(true)
                    .content("Could not find a sound by that name."),
            )
            .await?;
        }
    }

    Ok(())
}

/// Unset your global join sound
#[poise::command(slash_command, rename = "unset", guild_only = true)]
pub async fn unset_user_greet_sound(ctx: Context<'_>) -> Result<(), Error> {
    ctx.data()
        .update_join_sound(ctx.author().id, None::<GuildId>, None)
        .await?;

    ctx.send(
        CreateReply::default()
            .ephemeral(true)
            .content("Greet sound has been unset"),
    )
    .await?;

    Ok(())
}

/// Disable all greet sounds on this server
#[poise::command(
    slash_command,
    rename = "disable",
    guild_only = true,
    required_permissions = "MANAGE_GUILD"
)]
pub async fn disable_greet_sound(ctx: Context<'_>) -> Result<(), Error> {
    let guild_data_opt = ctx.guild_data(ctx.guild_id().unwrap()).await;

    if let Ok(guild_data) = guild_data_opt {
        guild_data.write().await.allow_greets = AllowGreet::Disabled;

        guild_data.read().await.commit(&ctx.data().database).await?;
    }

    ctx.say("Greet sounds have been disabled in this server")
        .await?;

    Ok(())
}

/// Enable only server greet sounds on this server
#[poise::command(
    slash_command,
    rename = "enable",
    guild_only = true,
    required_permissions = "MANAGE_GUILD"
)]
pub async fn enable_guild_greet_sound(ctx: Context<'_>) -> Result<(), Error> {
    let guild_data_opt = ctx.guild_data(ctx.guild_id().unwrap()).await;

    if let Ok(guild_data) = guild_data_opt {
        guild_data.write().await.allow_greets = AllowGreet::GuildOnly;

        guild_data.read().await.commit(&ctx.data().database).await?;
    }

    ctx.say("Greet sounds have been partially enable in this server. Use \"/greet server set\" to configure server greet sounds.")
        .await?;

    Ok(())
}

/// Enable all greet sounds on this server
#[poise::command(
    slash_command,
    rename = "enable",
    guild_only = true,
    required_permissions = "MANAGE_GUILD"
)]
pub async fn enable_greet_sound(ctx: Context<'_>) -> Result<(), Error> {
    let guild_data_opt = ctx.guild_data(ctx.guild_id().unwrap()).await;

    if let Ok(guild_data) = guild_data_opt {
        guild_data.write().await.allow_greets = AllowGreet::Enabled;

        guild_data.read().await.commit(&ctx.data().database).await?;
    }

    ctx.say("Greet sounds have been enable in this server")
        .await?;

    Ok(())
}
