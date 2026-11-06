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
            <label>
                "Titel"
                <input type="text" name="title" required value=title/>
            </label>
            <label>
                "Ingrediënten"
                <textarea name="ingredients" placeholder="- 500 g bloem" rows=ingredient_rows>
                    {ingredients}
                </textarea>
            </label>
            <label>
                "Bereiding"
                <textarea name="instructions">{instructions}</textarea>
            </label>
            <p class="hint">
                "Opmaak met Markdown: " <code>"- lijstje"</code> ", " <code>"**vet**"</code> ", "
                <code>"## kopje"</code>
            </p>
            {error}
            <div class="form-actions">
                <A attr:class="link-button" href=cancel_href>"Annuleer"</A>
                <input
                    class="link-button button-positive"
                    type="submit"
                    value=submit_label
                    disabled=save.pending()
                />
            </div>
        </ActionForm>
    }
}
