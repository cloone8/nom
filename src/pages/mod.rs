use leptos::prelude::*;
use leptos_router::hooks::use_params;
use leptos_router::params::Params;

pub mod editrecipe;
pub mod home;
pub mod newrecipe;
pub mod recipe;
mod recipeform;
pub mod trmnl;

#[derive(Debug, Clone, PartialEq, Params)]
struct RecipeParams {
    id: Option<i64>,
}

/// The `:id` route parameter, or `None` if it's missing or not a number.
fn use_recipe_id() -> Memo<Option<i64>> {
    let params = use_params::<RecipeParams>();
    Memo::new(move |_| params.read().as_ref().ok().and_then(|p| p.id))
}

#[component]
fn ErrorMessage(error: ServerFnError) -> impl IntoView {
    // `Display` prefixes our own messages with "error running server function: "
    let message = match error {
        ServerFnError::ServerError(message) => message,
        other => other.to_string(),
    };

    view! { <p class="error">"Er ging iets mis: " {message}</p> }
}
