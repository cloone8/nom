use leptos::form::ActionForm;
use leptos::prelude::*;
use leptos_router::components::A;

use super::ErrorMessage;
use crate::recipe::{RecipeSource, SaveRecipe};

/// The form for a new recipe (`recipe` is `None`) or for editing one. Works without WASM too.
#[component]
pub fn RecipeForm(recipe: Option<RecipeSource>, submit_label: &'static str) -> impl IntoView {
    let save = ServerAction::<SaveRecipe>::new();
    let error = move || match save.value().get() {
        Some(Err(error)) => Some(view! { <ErrorMessage error/> }),
        _ => None,
    };

    let id = recipe.as_ref().map(|r| r.id);
    let cancel_href = id.map_or("/".to_string(), |id| format!("/recipe/{id}"));
    let RecipeSource {
        title,
        ingredients,
        instructions,
        ..
    } = recipe.unwrap_or(RecipeSource {
        id: 0,
        title: String::new(),
        ingredients: String::new(),
        instructions: String::new(),
    });
    let ingredient_rows = ingredients.lines().count() + 2;

    view! {
        <ActionForm action=save>
            {id.map(|id| view! { <input type="hidden" name="id" value=id/> })}
            <h3>"Titel"</h3>
            <input type="text" name="title" placeholder="Titel" required value=title/>
            <br/>
            <h3>"Ingrediënten"</h3>
            <textarea name="ingredients" placeholder="- 500 g bloem" rows=ingredient_rows>
                {ingredients}
            </textarea>
            <br/>
            <h3>"Instructies"</h3>
            <textarea name="instructions" placeholder="Instructies">{instructions}</textarea>
            <br/>
            {error}
            <A attr:class="link-button" href=cancel_href>"Annuleer"</A>
            <input
                class="link-button button-positive"
                type="submit"
                value=submit_label
                disabled=save.pending()
            />
        </ActionForm>
    }
}
