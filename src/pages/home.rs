use leptos::prelude::*;
use leptos_router::components::{A, Form};
use leptos_router::hooks::use_query_map;

use super::ErrorMessage;
use crate::recipe::list_recipes;

#[component]
pub fn HomePage() -> impl IntoView {
    // The search lives in the URL (`/?q=soep`), so it survives a reload and can be shared
    let query = use_query_map();
    let search = move || query.read().get("q").unwrap_or_default();
    let recipes = Resource::new(search, list_recipes);

    view! {
        <h1>"Het NomNomNom Receptenboek"</h1>
        // `replace`, so typing doesn't add a history entry per keystroke
        <Form method="GET" action="" replace=true noscroll=true attr:class="search">
            // `oninput` is plain JS: it submits on every keystroke, which the hydrated `Form` turns
            // into a client-side navigation. Without JS, Enter submits.
            <input
                type="search"
                name="q"
                placeholder="Zoek een recept"
                aria-label="Zoek een recept"
                // The browser's suggestions of earlier searches would cover the results
                autocomplete="off"
                value=search
                oninput="this.form.requestSubmit()"
            />
        </Form>
        // Unlike `Suspense`, keeps showing the previous results while new ones load
        <Transition fallback=|| view! { <p>"Recepten aan het laden..."</p> }>
            {move || {
                let searching = !search().trim().is_empty();

                Suspend::new(async move {
                    match recipes.await {
                        Ok(recipes) if recipes.is_empty() && searching => {
                            view! { <p>"Geen recepten gevonden."</p> }.into_any()
                        }
                        Ok(recipes) if recipes.is_empty() => view! {
                            <p>"Er zijn nog geen recepten. " <A href="/new">"Voeg er eentje toe!"</A></p>
                        }
                            .into_any(),
                        Ok(recipes) => view! {
                            <ul class="recipe-list">
                                {recipes
                                    .into_iter()
                                    .map(|recipe| view! {
                                        <li class="recipe-link">
                                            <A href=format!("/recipe/{}", recipe.id)>{recipe.title}</A>
                                        </li>
                                    })
                                    .collect_view()}
                            </ul>
                        }
                            .into_any(),
                        Err(error) => view! { <ErrorMessage error/> }.into_any(),
                    }
                })
            }}
        </Transition>
    }
}
