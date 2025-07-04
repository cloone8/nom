use leptos::prelude::*;

#[derive(Debug, Clone)]
pub struct Recipe {
    pub title: String,
    pub ingredients: Vec<String>,
    pub instructions: String,
}

impl Recipe {
    pub fn test() -> Self {
        Recipe {
            title: "Het Testrecept".to_string(),
            ingredients: vec![
                "1 ui".to_string(),
                "twee planten".to_string(),
                "Een heel paard".to_string(),
            ],
            instructions: "Kook het paard met de twee planten en de ui. Klaar!".to_string(),
        }
    }
}

#[component]
pub fn RecipeComponent(recipe: Recipe) -> impl IntoView {
    let ingredients = recipe
        .ingredients
        .iter()
        .map(|ingr| {
            view! { <li>{ingr.clone()}</li>}
        })
        .collect_view();

    view! {
        <h2>{recipe.title}</h2>
        <ul>{ingredients}</ul>
        <p>{recipe.instructions}</p>
    }
}
