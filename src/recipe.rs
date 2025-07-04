use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawRecipe {
    pub title: String,
    pub ingredients: String,
    pub instructions: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[server]
pub async fn new_recipe(raw_recipe: RawRecipe) -> Result<(), ServerFnError> {
    use crate::DB;

    let ingredients_split: Vec<&str> = raw_recipe.ingredients.lines().collect();

    let mut db = DB.lock().await;

    let transaction = db.transaction()?;

    {
        let mut new_recipe_stmt = transaction
            .prepare_cached("INSERT INTO recipes (title, instructions) VALUES (?1, ?2);")?;

        let inserted = new_recipe_stmt.execute((raw_recipe.title, raw_recipe.instructions))?;

        assert_eq!(1, inserted);
    }

    let new_recipe_id = transaction.last_insert_rowid();

    {
        let mut new_ingredient_stmt = transaction
            .prepare_cached("INSERT INTO ingredients (recipe, ingredient) VALUES (?1, ?2);")?;

        for ingredient in ingredients_split {
            let inserted = new_ingredient_stmt.execute((new_recipe_id, ingredient))?;
            assert_eq!(1, inserted);
        }
    }

    transaction.commit()?;

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListedRecipe {
    pub id: i64,
    pub title: String,
}

#[server]
pub async fn list_recipes() -> Result<Vec<ListedRecipe>, ServerFnError> {
    use crate::DB;

    let db = DB.lock().await;

    let mut get_recipes_stmt = db.prepare_cached("SELECT id, title FROM recipes;").unwrap();

    let recipes = get_recipes_stmt.query(()).unwrap();

    Ok(recipes
        .mapped(|recipe| {
            Ok(ListedRecipe {
                id: recipe.get(0).unwrap(),
                title: recipe.get(1).unwrap(),
            })
        })
        .collect::<Result<Vec<_>, _>>()?)
}

#[server]
pub async fn get_recipe(id: i64) -> Result<Recipe, ServerFnError> {
    use crate::DB;

    let db = DB.lock().await;

    let mut get_recipe_stmt = db
        .prepare_cached("SELECT title, instructions FROM recipes WHERE id = (?1);")
        .unwrap();

    let (title, instructions) = get_recipe_stmt
        .query_one((id,), |row| Ok((row.get(0).unwrap(), row.get(1).unwrap())))
        .unwrap();

    let mut get_ingredients_stmt = db
        .prepare_cached("SELECT ingredient FROM ingredients WHERE recipe = (?1);")
        .unwrap();

    let ingredients = get_ingredients_stmt
        .query_map((id,), |row| Ok(row.get(0).unwrap()))
        .unwrap()
        .map(Result::unwrap)
        .collect();

    Ok(Recipe {
        title,
        ingredients,
        instructions,
    })
}
