use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

pub mod editrecipe;
pub mod home;
pub mod newrecipe;
pub mod recipe;
mod recipeform;
pub mod trmnl;

/// The `:id` route parameter, or `None` if it isn't a number.
fn use_recipe_id() -> impl Fn() -> Option<i64> + Copy + Send + Sync + 'static {
    let params = use_params_map();
    move || params.read().get("id")?.parse().ok()
}

/// `show` with the loaded value, `missing` if there's none, or the error.
fn loaded<T>(
    result: Result<Option<T>, ServerFnError>,
    missing: &'static str,
    show: impl FnOnce(T) -> AnyView,
) -> AnyView {
    match result {
        Ok(Some(value)) => show(value),
        Ok(None) => view! { <p>{missing}</p> }.into_any(),
        Err(error) => view! { <ErrorMessage error/> }.into_any(),
    }
}

/// The message of a server function error, without the "error running server function: "
/// that `Display` puts before our own messages.
pub fn error_message(error: ServerFnError) -> String {
    match error {
        ServerFnError::ServerError(message) => message,
        other => other.to_string(),
    }
}

#[component]
fn ErrorMessage(error: ServerFnError) -> impl IntoView {
    view! { <p class="error">"Er ging iets mis: " {error_message(error)}</p> }
}
