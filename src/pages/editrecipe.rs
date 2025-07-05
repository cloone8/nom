use leptos::ev::SubmitEvent;
use leptos::html;
use leptos::prelude::*;
use leptos::reactive::spawn_local;
use leptos_router::NavigateOptions;
use leptos_router::hooks::use_params;
use leptos_router::params::Params;

use crate::recipe::{RawRecipe, get_recipe, update_recipe};

#[derive(Debug, Params, PartialEq)]
struct EditRecipeArgs {
    id: Option<String>,
}

#[component]
pub fn EditRecipePage() -> impl IntoView {
    let id = move || {
        use_params::<EditRecipeArgs>()
            .read()
            .as_ref()
            .ok()
            .and_then(|params| params.id.clone())
            .unwrap()
    };

    let recipe_resource = Resource::new(id, async |id| {
        let parsed: i64 = id.parse().unwrap();
        get_recipe(parsed).await.unwrap().map(|rcp| (parsed, rcp))
    });

    let id_elem: NodeRef<html::Input> = NodeRef::new();
    let title_elem: NodeRef<html::Input> = NodeRef::new();
    let ingredient_elem: NodeRef<html::Textarea> = NodeRef::new();
    let instruction_elem: NodeRef<html::Textarea> = NodeRef::new();

    let on_submit = move |ev: SubmitEvent| {
        // stop the page from reloading!
        ev.prevent_default();

        let id = id_elem.get().unwrap().value();
        let title = title_elem.get().unwrap().value();
        let ingredients = ingredient_elem.get().unwrap().value();
        let instructions = instruction_elem.get().unwrap().value();

        spawn_local(async move {
            update_recipe(
                id.parse().expect("Submitted invalid id"),
                RawRecipe {
                    title,
                    ingredients,
                    instructions,
                },
            )
            .await
            .unwrap();

            let navigate = leptos_router::hooks::use_navigate();

            navigate(format!("/recipe/{id}").as_str(), NavigateOptions::default());
        });
    };

    view! {
        <h1>"Recept Aanpassen"</h1>
        <Suspense fallback=move || view!{ <p>"Recept aan het laden..."</p>}> {
            move || {
                let (id, recipe) = match recipe_resource.get().flatten() {
                    Some(rcp) => rcp,
                    None => {
                        return view! {
                            <p>"Onbekend recept"</p>
                        }.into_any()
                    },
                };

                let ingredients = recipe.ingredients.join("\n");

                view! {
                    <form on:submit=on_submit>
                        <input type="hidden" value={id} node_ref=id_elem/>
                        <h3>Titel</h3>
                        <input type="text" placeholder="Titel" value={recipe.title} node_ref=title_elem/>
                        <br/>
                        <h3>Ingredienten</h3>
                        <textarea placeholder="Ingredienten" node_ref=ingredient_elem>{ingredients}</textarea>
                        <br/>
                        <h3>Instructies</h3>
                        <textarea placeholder="Instructies" node_ref=instruction_elem>{recipe.instructions}</textarea>
                        <br/>
                        <input type="submit" value="Pas aan"/>
                    </form>
                }.into_any()
            }
        } </Suspense>
    }
}
