use leptos::prelude::*;

use crate::recipe::{ListedRecipe, list_recipes};

/// Renders the home page of your application.
#[component]
pub fn HomePage() -> impl IntoView {
    async fn fetch() -> Vec<ListedRecipe> {
        list_recipes().await.unwrap()
    }

    view! {
        <h1>"Het NomNomNom Receptenboek"</h1>
        <Await
            future=fetch()
            let:recipes
        >
            <ul>
                {
                    let mut recipes = recipes.clone();

                    recipes.sort_by_cached_key(|rp| rp.title.clone());

                    recipes.iter().map(|rp|{
                        let url = format!("/recipe/{}", rp.id);

                        view! {
                            <li class="recipe-link"><a href={url}>{rp.title.clone()}</a></li>
                        }
                    }).collect_view()
                }
            </ul>
        </Await>
    }
}
