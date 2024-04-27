use poise::{
    serenity_prelude,
    serenity_prelude::{
        constants::MESSAGE_CODE_LIMIT, ButtonStyle, ComponentInteraction, CreateActionRow,
        CreateButton, CreateEmbed, EditInteractionResponse, GuildId, UserId,
    },
    CreateReply,
};
use serde::{Deserialize, Serialize};

use crate::{
    consts::THEME_COLOR,
    models::sound::{Sound, SoundCtx},
    Context, Data, Error,
};

fn format_search_results(search_results: Vec<Sound>) -> CreateReply {
    let builder = CreateReply::default();

    let mut current_character_count = 0;
    let title = "Public sounds matching filter:";

    let field_iter = search_results
        .iter()
        .take(25)
        .map(|item| (&item.name, format!("ID: {}", item.id), true))
        .filter(|item| {
            current_character_count += item.0.len() + item.1.len();

            current_character_count <= MESSAGE_CODE_LIMIT - title.len()
        });

    builder.embed(CreateEmbed::default().title(title).fields(field_iter))
}

/// Show uploaded sounds
#[poise::command(slash_command, rename = "list", guild_only = true)]
pub async fn list_sounds(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[derive(Serialize, Deserialize, Clone, Copy)]
enum ListContext {
    User = 0,
    Guild = 1,
    Favorite = 2,
}

impl ListContext {
    pub fn title(&self) -> &'static str {
        match self {
            ListContext::User => "Your sounds",
            ListContext::Favorite => "Your favorite sounds",
            ListContext::Guild => "Server sounds",
        }
    }
}

/// Show the sounds uploaded to this server
#[poise::command(slash_command, rename = "server", guild_only = true)]
pub async fn list_guild_sounds(ctx: Context<'_>) -> Result<(), Error> {
    let pager = SoundPager {
        nonce: 0,
        page: 0,
        context: ListContext::Guild,
    };

    pager.reply(ctx).await?;

    Ok(())
}

/// Show all sounds you have uploaded
#[poise::command(slash_command, rename = "user", guild_only = true)]
pub async fn list_user_sounds(ctx: Context<'_>) -> Result<(), Error> {
    let pager = SoundPager {
        nonce: 0,
        page: 0,
        context: ListContext::User,
    };

    pager.reply(ctx).await?;

    Ok(())
}

/// Show sounds you have favorited
#[poise::command(slash_command, rename = "favorite", guild_only = true)]
pub async fn list_favorite_sounds(ctx: Context<'_>) -> Result<(), Error> {
    let pager = SoundPager {
        nonce: 0,
        page: 0,
        context: ListContext::Favorite,
    };

    pager.reply(ctx).await?;

    Ok(())
}

#[derive(Serialize, Deserialize)]
pub struct SoundPager {
    nonce: u64,
    page: u64,
    context: ListContext,
}

impl SoundPager {
    async fn get_page(
        &self,
        data: &Data,
        user_id: UserId,
        guild_id: GuildId,
    ) -> Result<Vec<Sound>, sqlx::Error> {
        match self.context {
            ListContext::User => data.user_sounds(user_id, Some(self.page)).await,
            ListContext::Favorite => data.favorite_sounds(user_id, Some(self.page)).await,
            ListContext::Guild => data.guild_sounds(guild_id, Some(self.page)).await,
        }
    }

    fn create_action_row(&self, max_page: u64) -> CreateActionRow {
        let row = CreateActionRow::Buttons(vec![
            CreateButton::new(
                serde_json::to_string(&SoundPager {
                    nonce: 0,
                    page: 0,
                    context: self.context,
                })
                .unwrap(),
            )
            .style(ButtonStyle::Primary)
            .label("⏪")
            .disabled(self.page == 0),
            CreateButton::new(
                serde_json::to_string(&SoundPager {
                    nonce: 1,
                    page: self.page.saturating_sub(1),
                    context: self.context,
                })
                .unwrap(),
            )
            .style(ButtonStyle::Secondary)
            .label("◀️")
            .disabled(self.page == 0),
            CreateButton::new("pid")
                .style(ButtonStyle::Success)
                .label(format!("Page {}", self.page + 1))
                .disabled(true),
            CreateButton::new(
                serde_json::to_string(&SoundPager {
                    nonce: 2,
                    page: self.page.saturating_add(1),
                    context: self.context,
                })
                .unwrap(),
            )
            .style(ButtonStyle::Secondary)
            .label("▶️")
            .disabled(self.page == max_page),
            CreateButton::new(
                serde_json::to_string(&SoundPager {
                    nonce: 3,
                    page: max_page,
                    context: self.context,
                })
                .unwrap(),
            )
            .style(ButtonStyle::Primary)
            .label("⏩")
            .disabled(self.page == max_page),
        ]);

        row
    }

    fn embed(&self, sounds: &[Sound], count: u64) -> CreateEmbed {
        CreateEmbed::default()
            .color(THEME_COLOR)
            .title(self.context.title())
            .description(format!("**{}** sounds:", count))
            .fields(sounds.iter().map(|s| {
                (
                    s.name.as_str(),
                    format!(
                        "ID: `{}`\n{}",
                        s.id,
                        if s.public { "*Public*" } else { "*Private*" }
                    ),
                    true,
                )
            }))
    }

    pub async fn handle_interaction(
        ctx: &serenity_prelude::Context,
        data: &Data,
        interaction: &ComponentInteraction,
    ) -> Result<(), Error> {
        let user_id = interaction.user.id;
        let guild_id = interaction.guild_id.unwrap();

        let pager = serde_json::from_str::<Self>(&interaction.data.custom_id)?;
        let sounds = pager.get_page(data, user_id, guild_id).await?;
        let count = match pager.context {
            ListContext::User => data.count_user_sounds(user_id).await?,
            ListContext::Favorite => data.count_favorite_sounds(user_id).await?,
            ListContext::Guild => data.count_guild_sounds(guild_id).await?,
        };

        interaction
            .edit_response(
                &ctx,
                EditInteractionResponse::default()
                    .add_embed(pager.embed(&sounds, count))
                    .components(vec![pager.create_action_row(count / 25)]),
            )
            .await?;

        Ok(())
    }

    async fn reply(&self, ctx: Context<'_>) -> Result<(), Error> {
        let sounds = self
            .get_page(ctx.data(), ctx.author().id, ctx.guild_id().unwrap())
            .await?;
        let count = match self.context {
            ListContext::User => ctx.data().count_user_sounds(ctx.author().id).await?,
            ListContext::Favorite => ctx.data().count_favorite_sounds(ctx.author().id).await?,
            ListContext::Guild => {
                ctx.data()
                    .count_guild_sounds(ctx.guild_id().unwrap())
                    .await?
            }
        };

        ctx.send(
            CreateReply::default()
                .ephemeral(true)
                .embed(self.embed(&sounds, count))
                .components(vec![self.create_action_row(count / 25)]),
        )
        .await?;

        Ok(())
    }
}

/// Search for sounds
#[poise::command(
    slash_command,
    rename = "search",
    category = "Search",
    guild_only = true
)]
pub async fn search_sounds(
    ctx: Context<'_>,
    #[description = "Sound name to search for"] query: String,
) -> Result<(), Error> {
    let search_results = ctx
        .data()
        .search_for_sound(&query, ctx.guild_id().unwrap(), ctx.author().id, false)
        .await?;

    ctx.send(format_search_results(search_results)).await?;

    Ok(())
}
