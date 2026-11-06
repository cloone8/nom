use leptos::prelude::*;

use super::{loaded, use_recipe_id};
use crate::recipe::{Recipe, get_recipe_view};

#[component]
pub fn RecipePage() -> impl IntoView {
    let recipe = Resource::new(use_recipe_id(), |id| async move {
        match id {
            Some(id) => get_recipe_view(id).await,
            None => Ok(None),
        }
    });

    view! {
        <Suspense fallback=|| view! { <p>"Recept aan het laden..."</p> }>
            {move || Suspend::new(async move {
                loaded(recipe.await, "Onbekend recept", |recipe| {
                    view! { <Recipe recipe editable=true/> }.into_any()
                })
            })}
        </Suspense>
    }
}
