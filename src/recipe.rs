use leptos::prelude::*;
use leptos_router::components::A;
use serde::{Deserialize, Serialize};

/// A recipe in the list on the home page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeSummary {
    pub id: i64,
    pub title: String,
}

/// A recipe as stored, with its text as Markdown. Used for editing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeSource {
    pub id: i64,
    pub title: String,
    pub ingredients: String,
    pub instructions: String,
}

/// A recipe with its Markdown rendered to HTML. Used for showing it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeView {
    pub id: i64,
    pub title: String,
    pub ingredients_html: String,
    pub instructions_html: String,
}

#[cfg(feature = "ssr")]
impl From<RecipeSource> for RecipeView {
    fn from(recipe: RecipeSource) -> Self {
        use crate::markdown::render;

        Self {
            id: recipe.id,
            title: recipe.title,
            ingredients_html: render(&recipe.ingredients),
            instructions_html: render(&recipe.instructions),
        }
    }
}

#[component]
pub fn RecipeComponent(recipe: RecipeView, with_edit: bool) -> impl IntoView {
    // An empty field renders to "", and then its heading would stand alone
    let section = |class: &'static str, heading: &'static str, html: String| {
        (!html.is_empty()).then(|| {
            view! {
                <section class=class>
                    <h2>{heading}</h2>
                    <div class="markdown" inner_html=html></div>
                </section>
            }
        })
    };

    view! {
        <article class="recipe">
            <h1>{recipe.title}</h1>
            {section("ingredients", "Ingrediënten", recipe.ingredients_html)}
            {section("instructions", "Bereiding", recipe.instructions_html)}
            {with_edit.then(|| view! {
                <A attr:class="link-button edit" href=format!("/edit/{}", recipe.id)>"Aanpassen"</A>
            })}
        </article>
    }
}

/// All recipes, or the ones matching `search`; see [`crate::search`].
#[server]
pub async fn list_recipes(search: String) -> Result<Vec<RecipeSummary>, ServerFnError> {
    use crate::db;

    let recipes = db::list_recipes(&db::connection())?;
    Ok(crate::search::search(recipes, &search))
}

#[server]
pub async fn get_recipe_view(id: i64) -> Result<Option<RecipeView>, ServerFnError> {
    use crate::db;

    let recipe = db::get_recipe(&db::connection(), id)?;
    Ok(recipe.map(RecipeView::from))
}

#[server]
pub async fn get_random_recipe_view() -> Result<Option<RecipeView>, ServerFnError> {
    use crate::db;

    let conn = db::connection();
    let recipe = match db::random_recipe_id(&conn)? {
        Some(id) => db::get_recipe(&conn, id)?,
        None => None,
    };
    drop(conn);

    Ok(recipe.map(RecipeView::from))
}

#[server]
pub async fn get_recipe_source(id: i64) -> Result<Option<RecipeSource>, ServerFnError> {
    use crate::db;

    Ok(db::get_recipe(&db::connection(), id)?)
}

/// Creates a recipe if `id` is `None`, otherwise updates that recipe. Then goes to the recipe's
/// page.
#[server]
pub async fn save_recipe(
    id: Option<i64>,
    title: String,
    ingredients: String,
    instructions: String,
) -> Result<(), ServerFnError> {
    use crate::db;

    let title = title.trim();
    if title.is_empty() {
        return Err(ServerFnError::new("Een recept heeft een titel nodig"));
    }

    // Browsers submit textarea line breaks as \r\n
    let ingredients = ingredients.replace("\r\n", "\n");
    let instructions = instructions.replace("\r\n", "\n");

    let conn = db::connection();
    let id = match id {
        None => db::insert_recipe(&conn, title, &ingredients, &instructions)?,
        Some(id) => {
            let recipe = RecipeSource {
                id,
                title: title.to_string(),
                ingredients,
                instructions,
            };

            if !db::update_recipe(&conn, &recipe)? {
                return Err(ServerFnError::new("Onbekend recept"));
            }

            id
        }
    };
    delete_unused_images(&conn);
    drop(conn);

    leptos_axum::redirect(&format!("/recipe/{id}"));
    Ok(())
}

/// Deletes a recipe, then goes to the home page.
#[server]
pub async fn delete_recipe(id: i64) -> Result<(), ServerFnError> {
    use crate::db;

    let conn = db::connection();
    db::delete_recipe(&conn, id)?;
    delete_unused_images(&conn);
    drop(conn);

    leptos_axum::redirect("/");
    Ok(())
}

/// Cleans up after a recipe changed; see [`crate::db::delete_unused_images`]. A failure here
/// shouldn't fail the save, so it's only logged.
#[cfg(feature = "ssr")]
fn delete_unused_images(conn: &rusqlite::Connection) {
    match crate::db::delete_unused_images(conn) {
        Ok(0) => {}
        Ok(deleted) => leptos::logging::log!("Deleted {deleted} unused image(s)"),
        Err(e) => leptos::logging::log!("Deleting unused images failed: {e}"),
    }
}

/// Handler for `GET /random`: redirects to a random recipe, or to the home page if there are none.
#[cfg(feature = "ssr")]
pub async fn random_recipe_redirect()
-> Result<axum::response::Redirect, (axum::http::StatusCode, String)> {
    use crate::db;
    use axum::http::StatusCode;
    use axum::response::Redirect;

    match db::random_recipe_id(&db::connection()) {
        Ok(Some(id)) => Ok(Redirect::to(&format!("/recipe/{id}"))),
        Ok(None) => Ok(Redirect::to("/")),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}
