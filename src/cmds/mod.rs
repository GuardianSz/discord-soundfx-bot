use poise::serenity_prelude::AutocompleteChoice;

use crate::{models::sound::SoundCtx, Context};

pub mod favorite;
pub mod info;
pub mod manage;
pub mod play;
pub mod search;
pub mod settings;
pub mod stop;

pub async fn autocomplete_sound(ctx: Context<'_>, partial: &str) -> Vec<AutocompleteChoice> {
    ctx.data()
        .autocomplete_user_sounds(&partial, ctx.author().id, ctx.guild_id().unwrap())
        .await
        .unwrap_or(vec![])
        .iter()
        .map(|s| AutocompleteChoice::new(s.name.clone(), s.id.to_string()))
        .collect()
}

pub async fn autocomplete_favorite(ctx: Context<'_>, partial: &str) -> Vec<AutocompleteChoice> {
    ctx.data()
        .autocomplete_favorite_sounds(&partial, ctx.author().id)
        .await
        .unwrap_or(vec![])
        .iter()
        .map(|s| AutocompleteChoice::new(s.name.clone(), s.id.to_string()))
        .collect()
}
