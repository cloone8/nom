use leptos::prelude::*;

use super::ErrorMessage;
use crate::recipe::{RecipeComponent, get_random_recipe_view};

/// A random recipe, for a TRMNL e-ink display. No navbar or edit button.
#[component]
pub fn TrmnlPage() -> impl IntoView {
    let recipe = OnceResource::new(get_random_recipe_view());

    view! {
        // Scopes the e-ink styles: pure black on white, sized for 800×480
        <div class="trmnl">
        <Suspense>
            {move || Suspend::new(async move {
                match recipe.await {
                    Ok(Some(recipe)) => view! { <RecipeComponent recipe with_edit=false/> }.into_any(),
                    Ok(None) => view! { <h2>"Nog geen recepten"</h2> }.into_any(),
                    Err(error) => view! { <ErrorMessage error/> }.into_any(),
                }
            })}
        </Suspense>
        </div>
    }
}
