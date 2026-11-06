use leptos::prelude::*;

use super::loaded;
use crate::recipe::{Recipe, get_random_recipe_view};

/// A random recipe, for a TRMNL e-ink display. No navbar or edit button.
#[component]
pub fn TrmnlPage() -> impl IntoView {
    let recipe = OnceResource::new(get_random_recipe_view());

    view! {
        // Scopes the e-ink styles: pure black on white, sized for 800×480
        <div class="trmnl">
            <Suspense>
                {move || Suspend::new(async move {
                    loaded(recipe.await, "Nog geen recepten", |recipe| {
                        view! { <Recipe recipe/> }.into_any()
                    })
                })}
            </Suspense>
        </div>
    }
}
