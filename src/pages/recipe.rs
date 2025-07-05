use leptos::prelude::*;
use leptos_router::hooks::use_params;
use leptos_router::params::Params;

use crate::recipe::{Recipe, RecipeComponent, get_recipe};

#[derive(Debug, Params, PartialEq)]
struct RecipeArgs {
    id: Option<String>,
}

#[component]
pub fn RecipePage() -> impl IntoView {
    let id = move || {
        use_params::<RecipeArgs>()
            .read()
            .as_ref()
            .ok()
            .and_then(|params| params.id.clone())
            .unwrap()
    };

    let recipe_resource = Resource::new(id, async |id| {
        if id == "test" {
            Some(Recipe::test())
        } else {
            get_recipe(id.parse().unwrap()).await.unwrap()
        }
    });

    let render_recipe = move || {
        recipe_resource.get().map(|recipe| match recipe {
            Some(recipe) => view! {<RecipeComponent recipe={recipe}/> }.into_any(),
            None => view! { <h2>"Onbekend recept"</h2>}.into_any(),
        })
    };

    view! {
        <h1>"Het NomNomNom Receptenboek " {id}</h1>
        <Suspense fallback=move || view!{ <p>"Recept aan het laden..."</p>}>
            {render_recipe}
        </Suspense>
    }
}
