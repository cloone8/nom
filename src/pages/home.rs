use leptos::prelude::*;
use leptos_router::components::A;

use super::ErrorMessage;
use crate::recipe::list_recipes;

#[component]
pub fn HomePage() -> impl IntoView {
    let recipes = OnceResource::new(list_recipes());

    view! {
        <h1>"Het NomNomNom Receptenboek"</h1>
        <Suspense fallback=|| view! { <p>"Recepten aan het laden..."</p> }>
            {move || Suspend::new(async move {
                match recipes.await {
                    Ok(recipes) if recipes.is_empty() => view! {
                        <p>"Er zijn nog geen recepten. " <A href="/new">"Voeg er eentje toe!"</A></p>
                    }
                        .into_any(),
                    Ok(recipes) => view! {
                        <ul>
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
            })}
        </Suspense>
    }
}
