use poise::serenity_prelude::{
    ActionRowComponent, ActivityData, ButtonKind, Context, CreateActionRow, CreateButton,
    EditInteractionResponse, FullEvent, Interaction,
};

#[cfg(feature = "metrics")]
use crate::metrics::GREET_COUNTER;
use crate::{
    cmds::search::SoundPager,
    models::{
        guild_data::{AllowGreet, CtxGuildData},
        join_sound::JoinSoundCtx,
        sound::Sound,
    },
    utils::{join_channel, play_audio, play_from_query},
    Data, Error,
};

pub async fn listener(ctx: &Context, event: &FullEvent, data: &Data) -> Result<(), Error> {
    match event {
        FullEvent::Ready { .. } => {
            ctx.set_activity(Some(ActivityData::watching("for /play")));
        }
        FullEvent::VoiceStateUpdate { old, new, .. } => {
            if let Some(past_state) = old {
                if let (Some(guild_id), None) = (past_state.guild_id, new.channel_id) {
                    if let Some(channel_id) = past_state.channel_id {
                        let is_okay = ctx
                            .cache
                            .channel(channel_id)
                            .map(|c| c.members(&ctx).ok().map(|m| m.len()))
                            .flatten()
                            .unwrap_or(0)
                            <= 1;

                        if is_okay {
                            let songbird = songbird::get(ctx).await.unwrap();

                            songbird.remove(guild_id).await?;
                        }
                    }
                }
            } else if let (Some(guild_id), Some(user_channel)) = (new.guild_id, new.channel_id) {
                let guild_data_opt = data.guild_data(guild_id).await;

                if let Ok(guild_data) = guild_data_opt {
                    let volume;
                    let allowed_greets;

                    {
                        let read = guild_data.read().await;

                        volume = read.volume;
                        allowed_greets = read.allow_greets;
                    }

                    if allowed_greets != AllowGreet::Disabled {
                        if let Some(join_id) = data
                            .join_sound(
                                new.user_id,
                                new.guild_id,
                                allowed_greets == AllowGreet::GuildOnly,
                            )
                            .await
                        {
                            let mut sound = sqlx::query_as_unchecked!(
                                Sound,
                                "
                                    SELECT name, id, public, server_id, uploader_id
                                        FROM sounds
                                        WHERE id = ?",
                                join_id
                            )
                            .fetch_one(&data.database)
                            .await
                            .unwrap();

                            let call = join_channel(&ctx, guild_id, user_channel).await?;

                            #[cfg(feature = "metrics")]
                            GREET_COUNTER.inc();

                            play_audio(
                                &mut sound,
                                volume,
                                &mut call.lock().await,
                                &data.database,
                                false,
                            )
                            .await
                            .unwrap();
                        }
                    }
                }
            }
        }
        FullEvent::InteractionCreate { interaction } => match interaction {
            Interaction::Component(component) => {
                if let Some(guild_id) = component.guild_id {
                    if let Ok(()) = SoundPager::handle_interaction(ctx, &data, component).await {
                    } else {
                        let mode = component.data.custom_id.as_str();
                        match mode {
                            "#stop" => {
                                component.defer(&ctx).await.unwrap();

                                let songbird = songbird::get(ctx).await.unwrap();
                                let call_opt = songbird.get(guild_id);

                                if let Some(call) = call_opt {
                                    let mut lock = call.lock().await;

                                    lock.stop();
                                }
                            }

                            "#loop" | "#queue" | "#instant" => {
                                let components = {
                                    let mut c = vec![];

                                    for action_row in &component.message.components {
                                        let mut row = vec![];
                                        // These are always buttons
                                        for component in &action_row.components {
                                            match component {
                                                ActionRowComponent::Button(button) => match &button
                                                    .data
                                                {
                                                    ButtonKind::Link { .. } => {}
                                                    ButtonKind::NonLink { custom_id, style } => row
                                                        .push(
                                                            CreateButton::new(
                                                                if custom_id.starts_with('#') {
                                                                    custom_id.to_string()
                                                                } else {
                                                                    format!(
                                                                        "{}{}",
                                                                        custom_id
                                                                            .split('#')
                                                                            .next()
                                                                            .unwrap(),
                                                                        mode
                                                                    )
                                                                },
                                                            )
                                                            .label(button.label.clone().unwrap())
                                                            .emoji(button.emoji.clone().unwrap())
                                                            .disabled(
                                                                custom_id == "#mode"
                                                                    || custom_id == mode,
                                                            )
                                                            .style(*style),
                                                        ),
                                                },
                                                _ => {}
                                            }
                                        }

                                        c.push(CreateActionRow::Buttons(row));
                                    }
                                    c
                                };

                                let response =
                                    EditInteractionResponse::default().components(components);

                                component.edit_response(&ctx, response).await.unwrap();
                            }

                            id_mode => {
                                component.defer(&ctx).await.unwrap();

                                let mut it = id_mode.split('#');
                                let id = it.next().unwrap();
                                let mode = it.next().unwrap_or("instant");

                                let guild =
                                    guild_id.to_guild_cached(&ctx).map(|g| g.clone()).unwrap();

                                play_from_query(
                                    &ctx,
                                    &data,
                                    &guild,
                                    component.user.id,
                                    None,
                                    id.split('#').next().unwrap(),
                                    mode == "loop",
                                )
                                .await;
                            }
                        }
                    }
                }
            }
            _ => {}
        },
        _ => {}
    }

    Ok(())
}
