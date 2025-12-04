use leptos::prelude::*;

use crate::recipe::{Recipe, RecipeComponent, get_recipe, random_recipe};

#[component]
pub fn TrmnlPage() -> impl IntoView {
    async fn fetch() -> (i64, Recipe) {
        let id = random_recipe().await.unwrap().unwrap();
        let recipe = get_recipe(id).await.unwrap().unwrap();

        (id, recipe)
    }

    view! {
        <Await future=fetch() let:id_recipe>
            <RecipeComponent id={id_recipe.0} recipe={id_recipe.1.clone()} with_mod=false/>
        </Await>
    }
}
