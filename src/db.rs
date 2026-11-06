//! SQLite storage. Server only.
//!
//! The functions take a `&Connection`, so they're plain synchronous code that's easy to test.
//! Server functions get the shared connection from [`connection`].

use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};

use rusqlite::{Connection, OptionalExtension, Result};

use crate::recipe::{RecipeSource, RecipeSummary};

static DB: OnceLock<Mutex<Connection>> = OnceLock::new();

/// Each entry upgrades the schema by one version. `PRAGMA user_version` holds how many have run.
const MIGRATIONS: &[&str] = &[
    // 1: the schema from before versioning, hence `IF NOT EXISTS`
    "CREATE TABLE IF NOT EXISTS recipes (
        id INTEGER PRIMARY KEY,
        title TEXT,
        instructions TEXT
    );
    CREATE TABLE IF NOT EXISTS ingredients (
        id INTEGER PRIMARY KEY,
        recipe INTEGER,
        ingredient TEXT,
        FOREIGN KEY(recipe) REFERENCES recipes(id)
    );",
    // 2: ingredients and instructions become Markdown; each ingredient row becomes a list item
    "CREATE TABLE recipes_new (
        id INTEGER PRIMARY KEY,
        title TEXT NOT NULL,
        ingredients TEXT NOT NULL,
        instructions TEXT NOT NULL
    );
    INSERT INTO recipes_new (id, title, ingredients, instructions)
    SELECT
        id,
        COALESCE(title, ''),
        COALESCE((
            SELECT group_concat('- ' || trim(ingredient), char(10) ORDER BY ingredients.id)
            FROM ingredients
            WHERE recipe = recipes.id AND trim(ingredient) <> ''
        ), ''),
        COALESCE(instructions, '')
    FROM recipes;
    DROP TABLE ingredients;
    DROP TABLE recipes;
    ALTER TABLE recipes_new RENAME TO recipes;",
];

/// Opens the database at `NOM_DB` (default `nom.db`) and brings its schema up to date.
/// Call once at startup, before anything uses [`connection`].
pub fn init() -> Result<()> {
    let path = std::env::var("NOM_DB")
        .ok()
        .filter(|p| !p.is_empty())
        .unwrap_or_else(|| "nom.db".to_string());

    let mut conn = Connection::open(path)?;
    migrate(&mut conn)?;

    if DB.set(Mutex::new(conn)).is_err() {
        panic!("db::init called twice");
    }

    Ok(())
}

/// The shared connection. Don't hold it across an `.await`.
pub fn connection() -> MutexGuard<'static, Connection> {
    DB.get()
        .expect("db::init wasn't called")
        .lock()
        // A panic elsewhere while holding the lock doesn't leave SQLite in a bad state
        .unwrap_or_else(PoisonError::into_inner)
}

fn migrate(conn: &mut Connection) -> Result<()> {
    let tx = conn.transaction()?;
    let version: u32 = tx.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    let latest = MIGRATIONS.len() as u32;

    if version < latest {
        for sql in &MIGRATIONS[version as usize..] {
            tx.execute_batch(sql)?;
        }

        tx.pragma_update(None, "user_version", latest)?;
    }

    tx.commit()
}

/// Every recipe with its ingredients (what search looks at), by title.
pub fn list_recipes(conn: &Connection) -> Result<Vec<(RecipeSummary, String)>> {
    conn.prepare_cached("SELECT id, title, ingredients FROM recipes ORDER BY title COLLATE NOCASE")?
        .query_map([], |row| {
            let summary = RecipeSummary {
                id: row.get(0)?,
                title: row.get(1)?,
            };

            Ok((summary, row.get(2)?))
        })?
        .collect()
}

pub fn get_recipe(conn: &Connection, id: i64) -> Result<Option<RecipeSource>> {
    conn.prepare_cached("SELECT title, ingredients, instructions FROM recipes WHERE id = ?1")?
        .query_row([id], |row| {
            Ok(RecipeSource {
                id,
                title: row.get(0)?,
                ingredients: row.get(1)?,
                instructions: row.get(2)?,
            })
        })
        .optional()
}

/// Returns the new recipe's id.
pub fn insert_recipe(
    conn: &Connection,
    title: &str,
    ingredients: &str,
    instructions: &str,
) -> Result<i64> {
    conn.prepare_cached(
        "INSERT INTO recipes (title, ingredients, instructions) VALUES (?1, ?2, ?3) RETURNING id",
    )?
    .query_row((title, ingredients, instructions), |row| row.get(0))
}

/// Returns `false` if there's no recipe with this id.
pub fn update_recipe(conn: &Connection, recipe: &RecipeSource) -> Result<bool> {
    let updated = conn
        .prepare_cached(
            "UPDATE recipes SET title = ?1, ingredients = ?2, instructions = ?3 WHERE id = ?4",
        )?
        .execute((
            &recipe.title,
            &recipe.ingredients,
            &recipe.instructions,
            recipe.id,
        ))?;

    Ok(updated == 1)
}

pub fn delete_recipe(conn: &Connection, id: i64) -> Result<()> {
    conn.prepare_cached("DELETE FROM recipes WHERE id = ?1")?
        .execute([id])?;

    Ok(())
}

pub fn random_recipe_id(conn: &Connection) -> Result<Option<i64>> {
    conn.query_row(
        "SELECT id FROM recipes ORDER BY random() LIMIT 1",
        [],
        |row| row.get(0),
    )
    .optional()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_the_unversioned_schema() {
        let mut conn = Connection::open_in_memory().unwrap();

        // The schema and data as the app wrote them before versioning
        conn.execute_batch(MIGRATIONS[0]).unwrap();
        conn.execute_batch(
            "INSERT INTO recipes (id, title, instructions) VALUES
                (1, 'Erwtensoep', 'Week de erwten.\nKook 2 uur.'),
                (2, 'Leeg', NULL);
            INSERT INTO ingredients (recipe, ingredient) VALUES
                (1, '500 g spliterwten'),
                (1, '  '),
                (1, ' 1 rookworst ');",
        )
        .unwrap();

        migrate(&mut conn).unwrap();

        let version: u32 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version as usize, MIGRATIONS.len());

        let soup = get_recipe(&conn, 1).unwrap().unwrap();
        assert_eq!(soup.title, "Erwtensoep");
        assert_eq!(soup.ingredients, "- 500 g spliterwten\n- 1 rookworst");
        assert_eq!(soup.instructions, "Week de erwten.\nKook 2 uur.");

        let empty = get_recipe(&conn, 2).unwrap().unwrap();
        assert_eq!(
            (empty.ingredients.as_str(), empty.instructions.as_str()),
            ("", "")
        );

        // Running again on an up-to-date database changes nothing
        migrate(&mut conn).unwrap();
        assert_eq!(
            get_recipe(&conn, 1).unwrap().unwrap().ingredients,
            soup.ingredients
        );
    }

    #[test]
    fn creates_a_fresh_database() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn).unwrap();

        let id = insert_recipe(&conn, "Pannenkoeken", "- bloem", "Bakken.").unwrap();
        let (summary, ingredients) = &list_recipes(&conn).unwrap()[0];
        assert_eq!(
            (summary.title.as_str(), ingredients.as_str()),
            ("Pannenkoeken", "- bloem")
        );
        assert_eq!(random_recipe_id(&conn).unwrap(), Some(id));

        delete_recipe(&conn, id).unwrap();
        assert_eq!(random_recipe_id(&conn).unwrap(), None);
    }
}
