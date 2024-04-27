#[macro_use]
extern crate lazy_static;

mod cmds;
mod consts;
mod error;
mod event_handlers;
#[cfg(feature = "metrics")]
mod metrics;
mod models;
mod utils;

use std::{env, path::Path, sync::Arc};

use dashmap::DashMap;
use poise::serenity_prelude::{
    model::{
        gateway::GatewayIntents,
        id::{GuildId, UserId},
    },
    ClientBuilder,
};
use songbird::SerenityInit;
use sqlx::{MySql, Pool};
use tokio::sync::RwLock;

use crate::{event_handlers::listener, models::guild_data::GuildData};

type Database = MySql;

pub struct Data {
    database: Pool<Database>,
    guild_data_cache: DashMap<GuildId, Arc<RwLock<GuildData>>>,
    join_sound_cache: DashMap<UserId, DashMap<Option<GuildId>, Option<u32>>>,
}

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if Path::new("/etc/soundfx-rs/config.env").exists() {
        dotenv::from_path("/etc/soundfx-rs/config.env").unwrap();
    }

    env_logger::init();

    let discord_token = env::var("DISCORD_TOKEN").expect("Missing DISCORD_TOKEN from environment");

    let options = poise::FrameworkOptions {
        commands: vec![
            cmds::info::help(),
            cmds::info::info(),
            cmds::manage::change_public(),
            cmds::manage::upload_new_sound(),
            cmds::manage::download_file(),
            cmds::manage::delete_sound(),
            cmds::play::play(),
            cmds::play::play_random(),
            cmds::play::queue_play(),
            cmds::play::loop_play(),
            cmds::play::soundboard(),
            poise::Command {
                subcommands: vec![
                    cmds::search::list_guild_sounds(),
                    cmds::search::list_user_sounds(),
                    cmds::search::list_favorite_sounds(),
                ],
                ..cmds::search::list_sounds()
            },
            poise::Command {
                subcommands: vec![
                    cmds::favorite::add_favorite(),
                    cmds::favorite::remove_favorite(),
                ],
                ..cmds::favorite::favorites()
            },
            cmds::search::search_sounds(),
            cmds::stop::stop_playing(),
            cmds::stop::disconnect(),
            cmds::settings::change_volume(),
            poise::Command {
                subcommands: vec![
                    poise::Command {
                        subcommands: vec![
                            cmds::settings::set_guild_greet_sound(),
                            cmds::settings::unset_guild_greet_sound(),
                            cmds::settings::enable_guild_greet_sound(),
                        ],
                        ..cmds::settings::guild_greet_sound()
                    },
                    poise::Command {
                        subcommands: vec![
                            cmds::settings::set_user_greet_sound(),
                            cmds::settings::unset_user_greet_sound(),
                        ],
                        ..cmds::settings::user_greet_sound()
                    },
                    cmds::settings::disable_greet_sound(),
                    cmds::settings::enable_greet_sound(),
                ],
                ..cmds::settings::greet_sound()
            },
        ],
        allowed_mentions: None,
        event_handler: |ctx, event, _framework, data| Box::pin(listener(ctx, event, data)),
        ..Default::default()
    };

    let database = Pool::connect(&env::var("DATABASE_URL").expect("No database URL provided"))
        .await
        .unwrap();

    sqlx::migrate!().run(&database).await?;

    #[cfg(feature = "metrics")]
    {
        metrics::init_metrics();
        tokio::spawn(async { metrics::serve().await });
    }

    let framework = poise::Framework::builder()
        .setup(move |ctx, _bot, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;

                Ok(Data {
                    database,
                    guild_data_cache: Default::default(),
                    join_sound_cache: Default::default(),
                })
            })
        })
        .options(options)
        .build();

    let mut client = ClientBuilder::new(
        &discord_token,
        GatewayIntents::GUILD_VOICE_STATES | GatewayIntents::GUILDS,
    )
    .framework(framework)
    .register_songbird()
    .await?;

    client.start_autosharded().await.unwrap();

    Ok(())
}
