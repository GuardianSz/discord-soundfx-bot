use poise::serenity_prelude::{async_trait, model::id::UserId, GuildId};
use sqlx::Acquire;

use crate::Data;

#[async_trait]
pub trait JoinSoundCtx {
    async fn join_sound<U: Into<UserId> + Send + Sync, G: Into<GuildId> + Send + Sync>(
        &self,
        user_id: U,
        guild_id: Option<G>,
        guild_only: bool,
    ) -> Option<u32>;
    async fn update_join_sound<U: Into<UserId> + Send + Sync, G: Into<GuildId> + Send + Sync>(
        &self,
        user_id: U,
        guild_id: Option<G>,
        join_id: Option<u32>,
    ) -> Result<(), sqlx::Error>;
}

struct JoinSound {
    join_sound_id: u32,
}

#[async_trait]
impl JoinSoundCtx for Data {
    async fn join_sound<U: Into<UserId> + Send + Sync, G: Into<GuildId> + Send + Sync>(
        &self,
        user_id: U,
        guild_id: Option<G>,
        guild_only: bool,
    ) -> Option<u32> {
        let user_id = user_id.into();
        let guild_id = guild_id.map(|g| g.into());

        let cached_join_id = self
            .join_sound_cache
            .get(&user_id)
            .map(|d| d.get(&guild_id).map(|i| i.value().clone()))
            .flatten();

        let x = if let Some(join_sound_id) = cached_join_id {
            join_sound_id
        } else {
            let join_sound_id = {
                let join_id_res = if guild_only {
                    sqlx::query_as!(
                        JoinSound,
                        "
                        SELECT join_sound_id
                            FROM join_sounds
                            WHERE user = ?
                            AND guild = ?
                            ORDER BY guild IS NULL",
                        user_id.get(),
                        guild_id.map(|g| g.get())
                    )
                    .fetch_one(&self.database)
                    .await
                } else {
                    sqlx::query_as!(
                        JoinSound,
                        "
                        SELECT join_sound_id
                            FROM join_sounds
                            WHERE user = ?
                            AND (guild IS NULL OR guild = ?)
                            ORDER BY guild IS NULL",
                        user_id.get(),
                        guild_id.map(|g| g.get())
                    )
                    .fetch_one(&self.database)
                    .await
                };

                if let Ok(row) = join_id_res {
                    Some(row.join_sound_id)
                } else {
                    None
                }
            };

            self.join_sound_cache.entry(user_id).and_modify(|d| {
                d.insert(guild_id, join_sound_id);
            });

            join_sound_id
        };

        x
    }

    async fn update_join_sound<U: Into<UserId> + Send + Sync, G: Into<GuildId> + Send + Sync>(
        &self,
        user_id: U,
        guild_id: Option<G>,
        join_id: Option<u32>,
    ) -> Result<(), sqlx::Error> {
        let user_id = user_id.into();
        let guild_id = guild_id.map(|g| g.into());

        self.join_sound_cache.entry(user_id).and_modify(|d| {
            d.insert(guild_id, join_id);
        });

        let mut transaction = self.database.begin().await?;

        match join_id {
            Some(join_id) => {
                sqlx::query!(
                    "DELETE FROM join_sounds WHERE user = ? AND guild <=> ?",
                    user_id.get(),
                    guild_id.map(|g| g.get())
                )
                .execute(transaction.acquire().await?)
                .await?;

                sqlx::query!(
                    "INSERT INTO join_sounds (user, join_sound_id, guild) VALUES (?, ?, ?)",
                    user_id.get(),
                    join_id,
                    guild_id.map(|g| g.get())
                )
                .execute(transaction.acquire().await?)
                .await?;
            }

            None => {
                sqlx::query!(
                    "DELETE FROM join_sounds WHERE user = ? AND guild <=> ?",
                    user_id.get(),
                    guild_id.map(|g| g.get())
                )
                .execute(transaction.acquire().await?)
                .await?;
            }
        }

        transaction.commit().await?;

        Ok(())
    }
}
