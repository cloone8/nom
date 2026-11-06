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
    // 3: uploaded images, referenced from recipe Markdown as `/images/<id>`. AUTOINCREMENT, so an
    // id is never reused and browsers may cache an image forever.
    "CREATE TABLE images (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        mime TEXT NOT NULL,
        data BLOB NOT NULL,
        created INTEGER NOT NULL DEFAULT (unixepoch())
    );",
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

/// Returns the new image's id.
pub fn insert_image(conn: &Connection, mime: &str, data: &[u8]) -> Result<i64> {
    conn.prepare_cached("INSERT INTO images (mime, data) VALUES (?1, ?2) RETURNING id")?
        .query_row((mime, data), |row| row.get(0))
}

/// The image's MIME type and bytes.
pub fn get_image(conn: &Connection, id: i64) -> Result<Option<(String, Vec<u8>)>> {
    conn.prepare_cached("SELECT mime, data FROM images WHERE id = ?1")?
        .query_row([id], |row| Ok((row.get(0)?, row.get(1)?)))
        .optional()
}

/// Deletes images that no recipe mentions as `images/<id>` anymore (with or without the
/// leading `/`: a relative `images/5` works on every page too). Only those older than a day, so
/// an image uploaded for a recipe that's still being written survives. Returns how many were
/// deleted.
pub fn delete_unused_images(conn: &Connection) -> Result<usize> {
    // The trailing space guarantees a non-digit after the id, so image 1 doesn't count as
    // mentioned by `images/12`
    conn.prepare_cached(
        "DELETE FROM images
        WHERE created < unixepoch() - 24 * 60 * 60
        AND NOT EXISTS (
            SELECT 1 FROM recipes
            WHERE ingredients || ' ' || instructions || ' ' GLOB '*images/' || images.id || '[^0-9]*'
        )",
    )?
    .execute([])
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

    #[test]
    fn deletes_old_unused_images() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn).unwrap();

        let ids: Vec<i64> = (0..13)
            .map(|_| insert_image(&conn, "image/jpeg", b"jpeg").unwrap())
            .collect();
        assert_eq!(ids, (1..=13).collect::<Vec<_>>());

        // Mentions 12 (not 1) and a relative 4 in the ingredients, and 3 at the very end of the
        // instructions
        insert_recipe(
            &conn,
            "Soep",
            "- ![](/images/12)\n- ![](images/4)",
            "Klaar. ![](/images/3)",
        )
        .unwrap();

        // Everything is recent, so nothing goes yet
        assert_eq!(delete_unused_images(&conn).unwrap(), 0);

        conn.execute(
            "UPDATE images SET created = created - 2 * 24 * 60 * 60 WHERE id <> 13",
            [],
        )
        .unwrap();
        // 1, 2 and 5–11 are old and unused; 3, 4 and 12 are mentioned; 13 is recent
        assert_eq!(delete_unused_images(&conn).unwrap(), 9);

        let left: Vec<i64> = conn
            .prepare("SELECT id FROM images ORDER BY id")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_>>()
            .unwrap();
        assert_eq!(left, [3, 4, 12, 13]);
        assert_eq!(
            get_image(&conn, 3).unwrap().unwrap(),
            ("image/jpeg".to_string(), b"jpeg".to_vec())
        );

        // An id is never reused, so a cached /images/<id> can't turn into another image
        assert_eq!(insert_image(&conn, "image/png", b"png").unwrap(), 14);
    }
}
