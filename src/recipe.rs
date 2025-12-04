use leptos::prelude::*;
use leptos_router::components::A;
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

#[component]
pub fn RecipeComponent(id: i64, recipe: Recipe, with_mod: bool) -> impl IntoView {
    let ingredients = recipe
        .ingredients
        .iter()
        .map(|ingr| {
            view! { <li>{ingr.clone()}</li>}
        })
        .collect_view();

    view! {
        <div class="recipe">
            <h1>{recipe.title}</h1>
            <ul>{ingredients}</ul>
            <p>{recipe.instructions}</p>
            <br/>
            {with_mod.then(|| view!{ <A class:link-button href={format!("/edit/{id}")}>"Aanpassen"</A>})}
        </div>
    }
}

#[server]
pub async fn new_recipe(raw_recipe: RawRecipe) -> Result<i64, ServerFnError> {
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

    Ok(new_recipe_id)
}

#[server]
pub async fn update_recipe(recipe_id: i64, raw_recipe: RawRecipe) -> Result<Recipe, ServerFnError> {
    use crate::DB;

    // Smaller scope because the future is !Send otherwise
    {
        let mut db = DB.lock().await;

        let transaction = db.transaction()?;

        // First delete the old recipes
        {
            let mut delete_ingredients_stmt = transaction
                .prepare_cached("DELETE FROM ingredients WHERE recipe = (?1);")
                .expect("Malformed query");

            _ = delete_ingredients_stmt
                .execute((recipe_id,))
                .expect("Failed to delete previous ingredients");
        }

        // Insert the new ones
        let ingredients_split: Vec<&str> = raw_recipe
            .ingredients
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();

        {
            let mut new_ingredient_stmt = transaction
                .prepare_cached("INSERT INTO ingredients (recipe, ingredient) VALUES (?1, ?2);")?;

            for ingredient in ingredients_split {
                let inserted = new_ingredient_stmt.execute((recipe_id, ingredient))?;
                assert_eq!(1, inserted);
            }
        }

        // Update the recipe itself
        {
            let mut update_recipe_stmt = transaction.prepare_cached(
                "UPDATE recipes SET title = ?1, instructions = ?2 WHERE id = ?3;",
            )?;

            let updated = update_recipe_stmt.execute((
                raw_recipe.title,
                raw_recipe.instructions,
                recipe_id,
            ))?;

            assert_eq!(1, updated);
        }

        transaction.commit()?;

        std::mem::drop(db);
    }

    let updated_recipe = get_recipe(recipe_id).await.unwrap();
    Ok(updated_recipe.expect("Could not find updated recipe"))
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
pub async fn get_recipe(id: i64) -> Result<Option<Recipe>, ServerFnError> {
    use crate::DB;
    use rusqlite::OptionalExtension;

    let db = DB.lock().await;

    let mut get_recipe_stmt = db
        .prepare_cached("SELECT title, instructions FROM recipes WHERE id = (?1);")
        .expect("Invalid statement");

    let (title, instructions) = if let Some((title, instructions)) = get_recipe_stmt
        .query_one((id,), |row| Ok((row.get(0).unwrap(), row.get(1).unwrap())))
        .optional()
        .unwrap()
    {
        (title, instructions)
    } else {
        return Ok(None);
    };

    let mut get_ingredients_stmt = db
        .prepare_cached("SELECT ingredient FROM ingredients WHERE recipe = (?1);")
        .expect("Invalid statement");

    let ingredients = get_ingredients_stmt
        .query_map((id,), |row| Ok(row.get(0).unwrap()))
        .unwrap()
        .map(Result::unwrap)
        .collect();

    Ok(Some(Recipe {
        title,
        ingredients,
        instructions,
    }))
}

#[server]
pub async fn delete_recipe(recipe_id: i64) -> Result<(), ServerFnError> {
    use crate::DB;

    let mut db = DB.lock().await;

    let transaction = db.transaction()?;

    {
        let mut delete_ingredients_stmt = transaction
            .prepare_cached("DELETE FROM ingredients WHERE recipe = (?1);")
            .expect("Malformed query");

        _ = delete_ingredients_stmt
            .execute((recipe_id,))
            .expect("Failed to  ingredients");
    }

    {
        let mut delete_recipe_stmt = transaction
            .prepare_cached("DELETE FROM recipes WHERE id = (?1);")
            .expect("Malformed query");

        let num_deleted = delete_recipe_stmt
            .execute((recipe_id,))
            .expect("Failed to recipe");

        assert_eq!(1, num_deleted, "Deleted an unexpected number of recipes");
    }

    transaction.commit()?;

    Ok(())
}

#[server]
pub async fn random_recipe() -> Result<Option<i64>, ServerFnError> {
    use crate::DB;
    use rusqlite::OptionalExtension;

    let db = DB.lock().await;

    Ok(db
        .query_one(
            "SELECT id FROM recipes ORDER BY RANDOM() LIMIT 1;",
            (),
            |row| Ok(row.get::<_, i64>(0).unwrap()),
        )
        .optional()
        .expect("Failed to read random ID"))
}
