use poise::serenity_prelude::async_trait;
use songbird::input::Input;
use sqlx::Executor;
use tokio::process::Command;

use crate::{consts::UPLOAD_MAX_SIZE, error::ErrorTypes, Data, Database};

#[derive(Clone)]
pub struct Sound {
    pub name: String,
    pub id: u32,
    pub public: bool,
    pub server_id: u64,
    pub uploader_id: Option<u64>,
}

impl PartialEq for Sound {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[async_trait]
pub trait SoundCtx {
    async fn search_for_sound<G: Into<u64> + Send, U: Into<u64> + Send>(
        &self,
        query: &str,
        guild_id: G,
        user_id: U,
        strict: bool,
    ) -> Result<Vec<Sound>, sqlx::Error>;
    async fn autocomplete_user_sounds<U: Into<u64> + Send, G: Into<u64> + Send>(
        &self,
        query: &str,
        user_id: U,
        guild_id: G,
    ) -> Result<Vec<Sound>, sqlx::Error>;
    async fn autocomplete_favorite_sounds<U: Into<u64> + Send>(
        &self,
        query: &str,
        user_id: U,
    ) -> Result<Vec<Sound>, sqlx::Error>;
    async fn user_sounds<U: Into<u64> + Send>(
        &self,
        user_id: U,
        page: Option<u64>,
    ) -> Result<Vec<Sound>, sqlx::Error>;
    async fn favorite_sounds<U: Into<u64> + Send>(
        &self,
        user_id: U,
        page: Option<u64>,
    ) -> Result<Vec<Sound>, sqlx::Error>;
    async fn guild_sounds<G: Into<u64> + Send>(
        &self,
        guild_id: G,
        page: Option<u64>,
    ) -> Result<Vec<Sound>, sqlx::Error>;
    async fn count_user_sounds<U: Into<u64> + Send>(&self, user_id: U) -> Result<u64, sqlx::Error>;
    async fn count_favorite_sounds<U: Into<u64> + Send>(
        &self,
        user_id: U,
    ) -> Result<u64, sqlx::Error>;
    async fn count_guild_sounds<G: Into<u64> + Send>(
        &self,
        guild_id: G,
    ) -> Result<u64, sqlx::Error>;
}

#[async_trait]
impl SoundCtx for Data {
    async fn search_for_sound<G: Into<u64> + Send, U: Into<u64> + Send>(
        &self,
        query: &str,
        guild_id: G,
        user_id: U,
        strict: bool,
    ) -> Result<Vec<Sound>, sqlx::Error> {
        let guild_id = guild_id.into();
        let user_id = user_id.into();
        let db_pool = self.database.clone();

        fn extract_id(s: &str) -> Option<u32> {
            if s.len() > 3 && s.to_lowercase().starts_with("id:") {
                match s[3..].parse::<u32>() {
                    Ok(id) => Some(id),

                    Err(_) => None,
                }
            } else if let Ok(id) = s.parse::<u32>() {
                Some(id)
            } else {
                None
            }
        }

        if let Some(id) = extract_id(&query) {
            let sound = sqlx::query_as_unchecked!(
                Sound,
                "
                SELECT name, id, public, server_id, uploader_id
                    FROM sounds
                    WHERE id = ? AND (
                        public = 1 OR
                        uploader_id = ? OR
                        server_id = ?
                    )",
                id,
                user_id,
                guild_id
            )
            .fetch_all(&db_pool)
            .await?;

            Ok(sound)
        } else {
            let name = query;
            let sound;

            if strict {
                sound = sqlx::query_as_unchecked!(
                    Sound,
                    "
                    SELECT name, id, public, server_id, uploader_id
                        FROM sounds
                        WHERE name = ? AND (
                            public = 1 OR
                            uploader_id = ? OR
                            server_id = ?
                        )
                        ORDER BY
                            uploader_id = ? DESC,
                            EXISTS(
                                SELECT 1
                                FROM favorite_sounds
                                WHERE sound_id = id AND user_id = ?
                            ) DESC,
                            server_id = ? DESC,
                            public = 1 DESC,
                            rand()",
                    name,
                    user_id,
                    guild_id,
                    user_id,
                    user_id,
                    guild_id
                )
                .fetch_all(&db_pool)
                .await?;
            } else {
                sound = sqlx::query_as_unchecked!(
                    Sound,
                    "
                    SELECT name, id, public, server_id, uploader_id
                        FROM sounds
                        WHERE name LIKE CONCAT('%', ?, '%') AND (
                            public = 1 OR
                            uploader_id = ? OR
                            server_id = ?
                        )
                        ORDER BY
                            uploader_id = ? DESC,
                            EXISTS(
                                SELECT 1
                                FROM favorite_sounds
                                WHERE sound_id = id AND user_id = ?
                            ) DESC,
                            server_id = ? DESC,
                            public = 1 DESC,
                            rand()",
                    name,
                    user_id,
                    guild_id,
                    user_id,
                    user_id,
                    guild_id
                )
                .fetch_all(&db_pool)
                .await?;
            }

            Ok(sound)
        }
    }

    async fn autocomplete_user_sounds<U: Into<u64> + Send, G: Into<u64> + Send>(
        &self,
        query: &str,
        user_id: U,
        guild_id: G,
    ) -> Result<Vec<Sound>, sqlx::Error> {
        let db_pool = self.database.clone();
        let user_id = user_id.into();

        sqlx::query_as_unchecked!(
            Sound,
            "
            SELECT name, id, public, server_id, uploader_id
            FROM sounds
            WHERE name LIKE CONCAT(?, '%') AND (uploader_id = ? OR server_id = ? OR EXISTS(
                SELECT 1
                FROM favorite_sounds
                WHERE sound_id = id AND user_id = ?
            ))
            LIMIT 25",
            query,
            user_id,
            guild_id.into(),
            user_id,
        )
        .fetch_all(&db_pool)
        .await
    }

    async fn autocomplete_favorite_sounds<U: Into<u64> + Send>(
        &self,
        query: &str,
        user_id: U,
    ) -> Result<Vec<Sound>, sqlx::Error> {
        let db_pool = self.database.clone();

        sqlx::query_as_unchecked!(
            Sound,
            "
            SELECT name, id, public, server_id, uploader_id
            FROM sounds
            WHERE name LIKE CONCAT(?, '%') AND EXISTS(
                SELECT 1
                FROM favorite_sounds
                WHERE sound_id = id AND user_id = ?
            )
            LIMIT 25",
            query,
            user_id.into(),
        )
        .fetch_all(&db_pool)
        .await
    }

    async fn user_sounds<U: Into<u64> + Send>(
        &self,
        user_id: U,
        page: Option<u64>,
    ) -> Result<Vec<Sound>, sqlx::Error> {
        let sounds = match page {
            Some(page) => {
                sqlx::query_as_unchecked!(
                    Sound,
                    "
                    SELECT name, id, public, server_id, uploader_id
                        FROM sounds
                        WHERE uploader_id = ?
                        ORDER BY id DESC
                        LIMIT ?, ?",
                    user_id.into(),
                    page * 25,
                    (page + 1) * 25
                )
                .fetch_all(&self.database)
                .await?
            }
            None => {
                sqlx::query_as_unchecked!(
                    Sound,
                    "
                    SELECT name, id, public, server_id, uploader_id
                        FROM sounds
                        WHERE uploader_id = ?
                        ORDER BY id DESC",
                    user_id.into()
                )
                .fetch_all(&self.database)
                .await?
            }
        };

        Ok(sounds)
    }

    async fn favorite_sounds<U: Into<u64> + Send>(
        &self,
        user_id: U,
        page: Option<u64>,
    ) -> Result<Vec<Sound>, sqlx::Error> {
        let sounds = match page {
            Some(page) => {
                sqlx::query_as_unchecked!(
                    Sound,
                    "
                    SELECT name, id, public, server_id, uploader_id
                        FROM sounds
                        INNER JOIN favorite_sounds f ON sounds.id = f.sound_id
                        WHERE f.user_id = ?
                        ORDER BY id DESC
                        LIMIT ?, ?",
                    user_id.into(),
                    page * 25,
                    (page + 1) * 25
                )
                .fetch_all(&self.database)
                .await?
            }
            None => {
                sqlx::query_as_unchecked!(
                    Sound,
                    "
                    SELECT name, id, public, server_id, uploader_id
                        FROM sounds
                        INNER JOIN favorite_sounds f ON sounds.id = f.sound_id
                        WHERE f.user_id = ?
                        ORDER BY id DESC",
                    user_id.into()
                )
                .fetch_all(&self.database)
                .await?
            }
        };

        Ok(sounds)
    }

    async fn guild_sounds<G: Into<u64> + Send>(
        &self,
        guild_id: G,
        page: Option<u64>,
    ) -> Result<Vec<Sound>, sqlx::Error> {
        let sounds = match page {
            Some(page) => {
                sqlx::query_as_unchecked!(
                    Sound,
                    "
                    SELECT name, id, public, server_id, uploader_id
                        FROM sounds
                        WHERE server_id = ?
                        ORDER BY id DESC
                        LIMIT ?, ?",
                    guild_id.into(),
                    page * 25,
                    (page + 1) * 25
                )
                .fetch_all(&self.database)
                .await?
            }

            None => {
                sqlx::query_as_unchecked!(
                    Sound,
                    "
                    SELECT name, id, public, server_id, uploader_id
                        FROM sounds
                        WHERE server_id = ?
                        ORDER BY id DESC",
                    guild_id.into()
                )
                .fetch_all(&self.database)
                .await?
            }
        };

        Ok(sounds)
    }

    async fn count_user_sounds<U: Into<u64> + Send>(&self, user_id: U) -> Result<u64, sqlx::Error> {
        Ok(sqlx::query!(
            "SELECT COUNT(1) as count FROM sounds WHERE uploader_id = ?",
            user_id.into()
        )
        .fetch_one(&self.database)
        .await?
        .count as u64)
    }

    async fn count_favorite_sounds<U: Into<u64> + Send>(
        &self,
        user_id: U,
    ) -> Result<u64, sqlx::Error> {
        Ok(sqlx::query!(
            "SELECT COUNT(1) as count FROM favorite_sounds WHERE user_id = ?",
            user_id.into()
        )
        .fetch_one(&self.database)
        .await?
        .count as u64)
    }

    async fn count_guild_sounds<G: Into<u64> + Send>(
        &self,
        guild_id: G,
    ) -> Result<u64, sqlx::Error> {
        Ok(sqlx::query!(
            "SELECT COUNT(1) as count FROM sounds WHERE server_id = ?",
            guild_id.into()
        )
        .fetch_one(&self.database)
        .await?
        .count as u64)
    }
}

impl Sound {
    pub(crate) async fn src(&self, db_pool: impl Executor<'_, Database = Database>) -> Vec<u8> {
        struct Src {
            src: Vec<u8>,
        }

        let record = sqlx::query_as_unchecked!(
            Src,
            "
            SELECT src
                FROM sounds
                WHERE id = ?
                LIMIT 1",
            self.id
        )
        .fetch_one(db_pool)
        .await
        .unwrap();

        record.src
    }

    pub async fn playable(
        &self,
        db_pool: impl Executor<'_, Database = Database>,
    ) -> Result<Input, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Input::from(self.src(db_pool).await))
    }

    pub async fn count_user_sounds<U: Into<u64>>(
        user_id: U,
        db_pool: impl Executor<'_, Database = Database>,
    ) -> Result<u32, sqlx::Error> {
        let user_id = user_id.into();

        let c = sqlx::query!(
            "
            SELECT COUNT(1) as count
                FROM sounds
                WHERE uploader_id = ?",
            user_id
        )
        .fetch_one(db_pool)
        .await?
        .count;

        Ok(c as u32)
    }

    pub async fn count_named_user_sounds<U: Into<u64>>(
        user_id: U,
        name: &String,
        db_pool: impl Executor<'_, Database = Database>,
    ) -> Result<u32, sqlx::Error> {
        let user_id = user_id.into();

        let c = sqlx::query!(
            "
            SELECT COUNT(1) as count
                FROM sounds
                WHERE
                    uploader_id = ? AND
                    name = ?",
            user_id,
            name
        )
        .fetch_one(db_pool)
        .await?
        .count;

        Ok(c as u32)
    }

    pub async fn commit(
        &self,
        db_pool: impl Executor<'_, Database = Database>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        sqlx::query!(
            "
            UPDATE sounds
            SET
                public = ?
            WHERE
                id = ?",
            self.public,
            self.id
        )
        .execute(db_pool)
        .await?;

        Ok(())
    }

    pub async fn delete(
        &self,
        db_pool: impl Executor<'_, Database = Database>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        sqlx::query!("DELETE FROM sounds WHERE id = ?", self.id)
            .execute(db_pool)
            .await?;

        Ok(())
    }

    pub async fn add_favorite<U: Into<u64>>(
        &self,
        user_id: U,
        db_pool: impl Executor<'_, Database = Database>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + Send>> {
        let user_id = user_id.into();

        sqlx::query!(
            "INSERT INTO favorite_sounds (user_id, sound_id) VALUES (?, ?)",
            user_id,
            self.id
        )
        .execute(db_pool)
        .await?;

        Ok(())
    }

    pub async fn remove_favorite<U: Into<u64>>(
        &self,
        user_id: U,
        db_pool: impl Executor<'_, Database = Database>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + Send>> {
        let user_id = user_id.into();

        sqlx::query!(
            "DELETE FROM favorite_sounds WHERE user_id = ? AND sound_id = ?",
            user_id,
            self.id
        )
        .execute(db_pool)
        .await?;

        Ok(())
    }

    pub async fn create_anon<G: Into<u64>, U: Into<u64>>(
        name: &str,
        src_url: &str,
        server_id: G,
        user_id: U,
        db_pool: impl Executor<'_, Database = Database>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + Send>> {
        let server_id = server_id.into();
        let user_id = user_id.into();

        async fn process_src(src_url: &str) -> Option<Vec<u8>> {
            let output = Command::new("ffmpeg")
                .kill_on_drop(true)
                .arg("-i")
                .arg(src_url)
                .arg("-loglevel")
                .arg("error")
                .arg("-f")
                .arg("opus")
                .arg("-fs")
                .arg(UPLOAD_MAX_SIZE.to_string())
                .arg("pipe:1")
                .output()
                .await;

            match output {
                Ok(out) => {
                    if out.status.success() {
                        Some(out.stdout)
                    } else {
                        None
                    }
                }

                Err(_) => None,
            }
        }

        let source = process_src(src_url).await;

        match source {
            Some(data) => {
                match sqlx::query!(
                    "
                    INSERT INTO sounds (name, server_id, uploader_id, public, src)
                        VALUES (?, ?, ?, 1, ?)",
                    name,
                    server_id,
                    user_id,
                    data
                )
                .execute(db_pool)
                .await
                {
                    Ok(_) => Ok(()),

                    Err(e) => Err(Box::new(e)),
                }
            }

            None => Err(Box::new(ErrorTypes::InvalidFile)),
        }
    }
}
