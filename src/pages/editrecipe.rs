use leptos::form::ActionForm;
use leptos::prelude::*;

use super::recipeform::RecipeForm;
use super::{loaded, use_recipe_id};
use crate::recipe::{DeleteRecipe, get_recipe_source};

#[component]
pub fn EditRecipePage() -> impl IntoView {
    let recipe = Resource::new(use_recipe_id(), |id| async move {
        match id {
            Some(id) => get_recipe_source(id).await,
            None => Ok(None),
        }
    });

    view! {
        <h1>"Recept aanpassen"</h1>
        <Suspense fallback=|| view! { <p>"Recept aan het laden..."</p> }>
            {move || Suspend::new(async move {
                loaded(recipe.await, "Onbekend recept", |recipe| {
                    let id = recipe.id;
                    view! {
                        <RecipeForm recipe=Some(recipe) submit_label="Pas aan"/>
                        <DeleteButton id/>
                    }
                        .into_any()
                })
            })}
        </Suspense>
    }
}

#[component]
fn DeleteButton(id: i64) -> impl IntoView {
    let delete = ServerAction::<DeleteRecipe>::new();

    view! {
        <ActionForm action=delete attr:class="delete-form">
            <input type="hidden" name="id" value=id/>
            // Plain JS, so it also asks before the WASM has loaded. A cancelled click doesn't
            // submit the form at all, so this works whatever handles the submit.
            <input
                class="link-button button-negative"
                type="submit"
                value="Verwijder"
                onclick="return confirm('Weet je zeker dat je het recept wilt verwijderen?')"
            />
        </ActionForm>
    }
}
