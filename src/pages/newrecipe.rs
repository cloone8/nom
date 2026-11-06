use leptos::prelude::*;

use super::recipeform::RecipeForm;

#[component]
pub fn NewRecipePage() -> impl IntoView {
    view! {
        <h1>"Nieuw recept"</h1>
        <RecipeForm recipe=None submit_label="Maak"/>
    }
}
