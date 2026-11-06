use leptos::prelude::*;

use super::{ErrorMessage, use_recipe_id};
use crate::recipe::{RecipeComponent, get_recipe_view};

#[component]
pub fn RecipePage() -> impl IntoView {
    let id = use_recipe_id();
    let recipe = Resource::new(
        move || id.get(),
        |id| async move {
            match id {
                Some(id) => get_recipe_view(id).await,
                None => Ok(None),
            }
        },
    );

    view! {
        <Suspense fallback=|| view! { <p>"Recept aan het laden..."</p> }>
            {move || Suspend::new(async move {
                match recipe.await {
                    Ok(Some(recipe)) => view! { <RecipeComponent recipe with_edit=true/> }.into_any(),
                    Ok(None) => view! { <h2>"Onbekend recept"</h2> }.into_any(),
                    Err(error) => view! { <ErrorMessage error/> }.into_any(),
                }
            })}
        </Suspense>
    }
}
