use std::{ops::Deref, sync::Arc};

use poise::serenity_prelude::{
    model::{
        guild::Guild,
        id::{ChannelId, UserId},
    },
    ChannelType, EditVoiceState, GuildId,
};
use songbird::{tracks::TrackHandle, Call};
use sqlx::Executor;
use tokio::sync::{Mutex, MutexGuard};

use crate::{
    models::{
        guild_data::CtxGuildData,
        sound::{Sound, SoundCtx},
    },
    Data, Database,
};

pub async fn play_audio(
    sound: &Sound,
    volume: u8,
    call_handler: &mut MutexGuard<'_, Call>,
    db_pool: impl Executor<'_, Database = Database>,
    r#loop: bool,
) -> Result<TrackHandle, Box<dyn std::error::Error + Send + Sync>> {
    let track = sound.playable(db_pool).await?;
    let handle = call_handler.play_input(track);

    handle.set_volume(volume as f32 / 100.0)?;

    if r#loop {
        handle.enable_loop()?;
    } else {
        handle.disable_loop()?;
    }

    Ok(handle)
}

pub async fn queue_audio(
    sounds: &[Sound],
    volume: u8,
    call_handler: &mut MutexGuard<'_, Call>,
    db_pool: impl Executor<'_, Database = Database> + Copy,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    for sound in sounds {
        let track = sound.playable(db_pool).await?;
        let handle = call_handler.enqueue_input(track).await;

        handle.set_volume(volume as f32 / 100.0)?;
    }

    Ok(())
}

pub async fn join_channel(
    ctx: &poise::serenity_prelude::Context,
    guild_id: GuildId,
    channel_id: ChannelId,
) -> Result<Arc<Mutex<Call>>, Box<dyn std::error::Error + Send + Sync>> {
    let songbird = songbird::get(ctx).await.unwrap();
    let current_user = ctx.cache.current_user().id;

    let current_voice_state = ctx
        .cache
        .guild(guild_id)
        .map(|g| {
            g.voice_states
                .get(&current_user)
                .and_then(|voice_state| voice_state.channel_id)
        })
        .flatten();

    let call = if current_voice_state == Some(channel_id) {
        let call_opt = songbird.get(guild_id);

        if let Some(call) = call_opt {
            Ok(call)
        } else {
            songbird.join(guild_id, channel_id).await
        }
    } else {
        songbird.join(guild_id, channel_id).await
    }?;

    {
        call.lock().await.deafen(true).await?;
    }

    if let Some(channel) = ctx.cache.channel(channel_id).map(|c| c.clone()) {
        if channel.kind == ChannelType::Stage {
            let user_id = ctx.cache.current_user().id.clone();

            channel
                .edit_voice_state(&ctx, user_id, EditVoiceState::new().suppress(true))
                .await?;
        }
    }

    Ok(call)
}

pub async fn play_from_query(
    ctx: &poise::serenity_prelude::Context,
    data: &Data,
    guild: impl Deref<Target = Guild> + Send + Sync,
    user_id: UserId,
    channel: Option<ChannelId>,
    query: &str,
    r#loop: bool,
) -> String {
    let guild_id = guild.deref().id;

    let channel_to_join = channel.or_else(|| {
        guild
            .deref()
            .voice_states
            .get(&user_id)
            .and_then(|voice_state| voice_state.channel_id)
    });

    match channel_to_join {
        Some(user_channel) => {
            let mut sound_vec = data
                .search_for_sound(query, guild_id, user_id, true)
                .await
                .unwrap();

            let sound_res = sound_vec.first_mut();

            match sound_res {
                Some(sound) => {
                    {
                        let call_handler = join_channel(ctx, guild_id, user_channel).await.unwrap();

                        let guild_data = data.guild_data(guild_id).await.unwrap();

                        let mut lock = call_handler.lock().await;

                        play_audio(
                            sound,
                            guild_data.read().await.volume,
                            &mut lock,
                            &data.database,
                            r#loop,
                        )
                        .await
                        .unwrap();
                    }

                    format!("Playing sound {} with ID {}", sound.name, sound.id)
                }

                None => "Couldn't find sound by term provided".to_string(),
            }
        }

        None => "You are not in a voice chat!".to_string(),
    }
}
