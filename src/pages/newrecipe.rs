use leptos::ev::SubmitEvent;
use leptos::html;
use leptos::prelude::*;
use leptos::reactive::spawn_local;
use leptos_router::NavigateOptions;

use crate::recipe::{RawRecipe, new_recipe};

#[component]
pub fn NewRecipePage() -> impl IntoView {
    let title_elem: NodeRef<html::Input> = NodeRef::new();
    let ingredient_elem: NodeRef<html::Textarea> = NodeRef::new();
    let instruction_elem: NodeRef<html::Textarea> = NodeRef::new();

    let on_submit = move |ev: SubmitEvent| {
        // stop the page from reloading!
        ev.prevent_default();

        let title = title_elem.get().unwrap().value();
        let ingredients = ingredient_elem.get().unwrap().value();
        let instructions = instruction_elem.get().unwrap().value();

        spawn_local(async {
            new_recipe(RawRecipe {
                title,
                ingredients,
                instructions,
            })
            .await
            .unwrap();

            let navigate = leptos_router::hooks::use_navigate();

            navigate("/", NavigateOptions::default());
        });
    };

    view! {
        <h1>"Nieuw recept"</h1>
        <form on:submit=on_submit>
            <h3>Titel</h3>
            <input type="text" placeholder="Titel" node_ref=title_elem/>
            <br/>
            <h3>Ingredienten</h3>
            <textarea placeholder="Ingredienten" node_ref=ingredient_elem/>
            <br/>
            <h3>Instructies</h3>
            <textarea placeholder="Instructies" node_ref=instruction_elem/>
            <br/>
            <input type="submit" value="Maak"/>
        </form>
    }
}
