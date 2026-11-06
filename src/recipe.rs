use leptos::prelude::*;
use leptos_router::components::A;
use serde::{Deserialize, Serialize};

#[cfg(feature = "ssr")]
use crate::db;

/// A recipe in the list on the home page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeSummary {
    pub id: i64,
    pub title: String,
}

/// A recipe as stored, with its text as Markdown. Used for editing.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecipeSource {
    pub id: i64,
    pub title: String,
    pub ingredients: String,
    pub instructions: String,
    /// Its photos (image ids) in order; the first is the header photo.
    pub photos: Vec<i64>,
}

/// A recipe with its Markdown rendered to HTML. Used for showing it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeView {
    pub id: i64,
    pub title: String,
    pub ingredients_html: String,
    pub instructions_html: String,
    /// Shown large above the recipe.
    pub header_photo: Option<i64>,
    /// The other photos, shown below it.
    pub gallery: Vec<i64>,
}

#[cfg(feature = "ssr")]
impl From<RecipeSource> for RecipeView {
    fn from(recipe: RecipeSource) -> Self {
        use crate::markdown::render;

        let mut photos = recipe.photos.into_iter();

        Self {
            id: recipe.id,
            title: recipe.title,
            ingredients_html: render(&recipe.ingredients),
            instructions_html: render(&recipe.instructions),
            header_photo: photos.next(),
            gallery: photos.collect(),
        }
    }
}

/// A recipe, with an "Aanpassen" link if `editable`.
#[component]
pub fn Recipe(recipe: RecipeView, #[prop(optional)] editable: bool) -> impl IntoView {
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

    let gallery = (!recipe.gallery.is_empty()).then(|| {
        view! {
            <section class="gallery">
                <h2>"Foto's"</h2>
                <div class="photos">
                    {recipe
                        .gallery
                        .into_iter()
                        .map(|id| {
                            // `rel="external"`: a server route, not a page, so the router stays out
                            view! {
                                <a href=format!("/images/{id}") target="_blank" rel="external">
                                    <img src=format!("/images/{id}") alt="" loading="lazy"/>
                                </a>
                            }
                        })
                        .collect_view()}
                </div>
            </section>
        }
    });

    view! {
        <article class="recipe">
            <h1>{recipe.title}</h1>
            {recipe.header_photo.map(|id| view! {
                <img class="header-photo" src=format!("/images/{id}") alt=""/>
            })}
            {section("ingredients", "Ingrediënten", recipe.ingredients_html)}
            {section("instructions", "Bereiding", recipe.instructions_html)}
            {gallery}
            {editable.then(|| view! {
                <A attr:class="link-button edit" href=format!("/edit/{}", recipe.id)>"Aanpassen"</A>
            })}
        </article>
    }
}

/// All recipes, or the ones matching `search`; see [`crate::search`].
#[server]
pub async fn list_recipes(search: String) -> Result<Vec<RecipeSummary>, ServerFnError> {
    let recipes = db::list_recipes(&db::connection())?;
    Ok(crate::search::search(recipes, &search))
}

#[server]
pub async fn get_recipe_view(id: i64) -> Result<Option<RecipeView>, ServerFnError> {
    let recipe = db::get_recipe(&db::connection(), id)?;
    Ok(recipe.map(RecipeView::from))
}

#[server]
pub async fn get_random_recipe_view() -> Result<Option<RecipeView>, ServerFnError> {
    let conn = db::connection();
    let recipe = db::random_recipe_id(&conn)?.map(|id| db::get_recipe(&conn, id));
    Ok(recipe.transpose()?.flatten().map(RecipeView::from))
}

#[server]
pub async fn get_recipe_source(id: i64) -> Result<Option<RecipeSource>, ServerFnError> {
    Ok(db::get_recipe(&db::connection(), id)?)
}

/// Creates a recipe if `id` is `None`, otherwise updates that recipe. Then goes to the recipe's
/// page.
///
/// `photos` lists the recipe's photos after editing, as comma-separated image ids: drafts in it
/// become the recipe's, and its photos missing from it are deleted.
#[server]
pub async fn save_recipe(
    id: Option<i64>,
    title: String,
    ingredients: String,
    instructions: String,
    photos: String,
) -> Result<(), ServerFnError> {
    let title = title.trim();
    if title.is_empty() {
        return Err(ServerFnError::new("Een recept heeft een titel nodig"));
    }

    // Browsers submit textarea line breaks as \r\n
    let ingredients = ingredients.replace("\r\n", "\n");
    let instructions = instructions.replace("\r\n", "\n");
    let photos: Vec<i64> = photos
        .split(',')
        .filter_map(|id| id.trim().parse().ok())
        .collect();

    let mut conn = db::connection();
    // The recipe and its photos change together or not at all
    let tx = conn.transaction()?;

    let id = match id {
        None => db::insert_recipe(&tx, title, &ingredients, &instructions)?,
        Some(id) if db::update_recipe(&tx, id, title, &ingredients, &instructions)? => id,
        Some(_) => return Err(ServerFnError::new("Onbekend recept")),
    };

    db::set_recipe_photos(&tx, id, &photos)?;
    tx.commit()?;
    delete_old_drafts(&conn);

    leptos_axum::redirect(&format!("/recipe/{id}"));
    Ok(())
}

/// Deletes a recipe and its photos, then goes to the home page.
#[server]
pub async fn delete_recipe(id: i64) -> Result<(), ServerFnError> {
    let mut conn = db::connection();
    // The recipe and its photos go together or not at all
    let tx = conn.transaction()?;
    db::delete_recipe(&tx, id)?;
    tx.commit()?;
    delete_old_drafts(&conn);

    leptos_axum::redirect("/");
    Ok(())
}

/// See [`crate::db::delete_old_drafts`]. A failure here shouldn't fail the save, so it's only
/// logged.
#[cfg(feature = "ssr")]
fn delete_old_drafts(conn: &rusqlite::Connection) {
    match crate::db::delete_old_drafts(conn) {
        Ok(0) => {}
        Ok(deleted) => leptos::logging::log!("Deleted {deleted} old draft photo(s)"),
        Err(e) => leptos::logging::log!("Deleting old draft photos failed: {e}"),
    }
}

/// Handler for `GET /random`: redirects to a random recipe, or to the home page if there are none.
#[cfg(feature = "ssr")]
pub async fn random_recipe_redirect() -> Result<axum::response::Redirect, axum::http::StatusCode> {
    use axum::http::StatusCode;
    use axum::response::Redirect;

    match db::random_recipe_id(&db::connection()) {
        Ok(Some(id)) => Ok(Redirect::to(&format!("/recipe/{id}"))),
        Ok(None) => Ok(Redirect::to("/")),
        Err(e) => {
            leptos::logging::log!("Picking a random recipe failed: {e}");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[cfg(all(test, feature = "ssr"))]
#[test]
fn first_photo_is_the_header() {
    let view = RecipeView::from(RecipeSource {
        photos: vec![5, 1, 20],
        ..Default::default()
    });
    assert_eq!((view.header_photo, view.gallery), (Some(5), vec![1, 20]));

    let view = RecipeView::from(RecipeSource::default());
    assert_eq!((view.header_photo, view.gallery), (None, vec![]));
}
