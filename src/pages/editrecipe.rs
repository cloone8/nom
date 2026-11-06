use leptos::ev::SubmitEvent;
use leptos::form::ActionForm;
use leptos::prelude::*;

use super::recipeform::RecipeForm;
use super::{ErrorMessage, use_recipe_id};
use crate::recipe::{DeleteRecipe, get_recipe_source};

#[component]
pub fn EditRecipePage() -> impl IntoView {
    let id = use_recipe_id();
    let recipe = Resource::new(
        move || id.get(),
        |id| async move {
            match id {
                Some(id) => get_recipe_source(id).await,
                None => Ok(None),
            }
        },
    );

    view! {
        <h1>"Recept aanpassen"</h1>
        <Suspense fallback=|| view! { <p>"Recept aan het laden..."</p> }>
            {move || Suspend::new(async move {
                match recipe.await {
                    Ok(Some(recipe)) => {
                        let id = recipe.id;
                        view! {
                            <RecipeForm recipe=Some(recipe) submit_label="Pas aan"/>
                            <DeleteButton id/>
                        }
                            .into_any()
                    }
                    Ok(None) => view! { <p>"Onbekend recept"</p> }.into_any(),
                    Err(error) => view! { <ErrorMessage error/> }.into_any(),
                }
            })}
        </Suspense>
    }
}

#[component]
fn DeleteButton(id: i64) -> impl IntoView {
    let delete = ServerAction::<DeleteRecipe>::new();

    // Capture phase, so this runs before `ActionForm` submits (it skips prevented events)
    let confirm = |ev: SubmitEvent| {
        let confirmed = window()
            .confirm_with_message("Weet je zeker dat je het recept wilt verwijderen?")
            .unwrap_or(false);

        if !confirmed {
            ev.prevent_default();
        }
    };

    view! {
        <ActionForm action=delete on:submit:capture=confirm attr:class="delete-form">
            <input type="hidden" name="id" value=id/>
            <input class="link-button button-negative" type="submit" value="Verwijder"/>
        </ActionForm>
    }
}
