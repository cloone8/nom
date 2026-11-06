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
    // 4: each image belongs to a recipe; NULL marks a draft, uploaded for a recipe that isn't
    // saved yet. Existing images go to the first recipe that mentions them; the rest become
    // drafts.
    "ALTER TABLE images ADD COLUMN recipe INTEGER;
    UPDATE images SET recipe = (
        SELECT id FROM recipes
        WHERE ingredients || ' ' || instructions || ' ' GLOB '*images/' || images.id || '[^0-9]*'
        ORDER BY id
        LIMIT 1
    );
    CREATE INDEX images_recipe ON images (recipe);",
    // 5: the order of a recipe's photos; the first is the header photo. Upload order until
    // the user changes it.
    "ALTER TABLE images ADD COLUMN position INTEGER NOT NULL DEFAULT 0;
    UPDATE images SET position = id;",
    // 6: AUTOINCREMENT, so a deleted recipe's id isn't reused: an edit form still open on it
    // would save over the new recipe
    "CREATE TABLE recipes_new (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        title TEXT NOT NULL,
        ingredients TEXT NOT NULL,
        instructions TEXT NOT NULL
    );
    INSERT INTO recipes_new (id, title, ingredients, instructions)
    SELECT id, title, ingredients, instructions FROM recipes;
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
    enable_auto_vacuum(&conn)?;

    assert!(DB.set(Mutex::new(conn)).is_ok(), "db::init called twice");
    Ok(())
}

/// Makes SQLite give the space of deleted data (photos, mostly) back to the file system on
/// every commit. Switching an existing database over takes a `VACUUM`, so the first start
/// after this was added takes a moment longer.
fn enable_auto_vacuum(conn: &Connection) -> Result<()> {
    // 0 = none, 1 = full, 2 = incremental
    let mode: u32 = conn.query_row("PRAGMA auto_vacuum", [], |row| row.get(0))?;

    if mode != 1 {
        conn.execute_batch("PRAGMA auto_vacuum = FULL; VACUUM;")?;
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
    conn.prepare("SELECT id, title, ingredients FROM recipes ORDER BY title COLLATE NOCASE")?
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
    let sql = "SELECT title, ingredients, instructions FROM recipes WHERE id = ?1";
    conn.query_row(sql, [id], |row| {
        Ok(RecipeSource {
            id,
            title: row.get(0)?,
            ingredients: row.get(1)?,
            instructions: row.get(2)?,
            photos: recipe_photos(conn, id)?,
        })
    })
    .optional()
}

/// The recipe's photos in order; the first is the header photo.
fn recipe_photos(conn: &Connection, recipe: i64) -> Result<Vec<i64>> {
    conn.prepare("SELECT id FROM images WHERE recipe = ?1 ORDER BY position, id")?
        .query_map([recipe], |row| row.get(0))?
        .collect()
}

/// Makes `photos`, in this order, the recipe's photos: drafts among them are linked to it, and
/// its photos that aren't among them are deleted. Ids of another recipe's photos are ignored.
pub fn set_recipe_photos(conn: &Connection, recipe: i64, photos: &[i64]) -> Result<()> {
    for old in recipe_photos(conn, recipe)? {
        if !photos.contains(&old) {
            conn.execute("DELETE FROM images WHERE id = ?1", [old])?;
        }
    }

    for (position, &photo) in photos.iter().enumerate() {
        // A repeated id keeps its first position
        if photos[..position].contains(&photo) {
            continue;
        }

        conn.execute(
            "UPDATE images SET recipe = ?1, position = ?2
            WHERE id = ?3 AND (recipe IS NULL OR recipe = ?1)",
            (recipe, position as i64, photo),
        )?;
    }

    Ok(())
}

/// Returns the new recipe's id.
pub fn insert_recipe(
    conn: &Connection,
    title: &str,
    ingredients: &str,
    instructions: &str,
) -> Result<i64> {
    let sql =
        "INSERT INTO recipes (title, ingredients, instructions) VALUES (?1, ?2, ?3) RETURNING id";
    conn.query_row(sql, (title, ingredients, instructions), |row| row.get(0))
}

/// Returns `false` if there's no recipe with this id.
pub fn update_recipe(
    conn: &Connection,
    id: i64,
    title: &str,
    ingredients: &str,
    instructions: &str,
) -> Result<bool> {
    let sql = "UPDATE recipes SET title = ?2, ingredients = ?3, instructions = ?4 WHERE id = ?1";
    Ok(conn.execute(sql, (id, title, ingredients, instructions))? == 1)
}

/// Deletes the recipe and its photos.
pub fn delete_recipe(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM images WHERE recipe = ?1", [id])?;
    conn.execute("DELETE FROM recipes WHERE id = ?1", [id])?;

    Ok(())
}

/// Stores a draft image (no recipe yet; see [`set_recipe_photos`]) and returns its id.
pub fn insert_image(conn: &Connection, mime: &str, data: &[u8]) -> Result<i64> {
    let sql = "INSERT INTO images (mime, data) VALUES (?1, ?2) RETURNING id";
    conn.query_row(sql, (mime, data), |row| row.get(0))
}

/// The image's MIME type and bytes.
pub fn get_image(conn: &Connection, id: i64) -> Result<Option<(String, Vec<u8>)>> {
    let sql = "SELECT mime, data FROM images WHERE id = ?1";
    conn.query_row(sql, [id], |row| Ok((row.get(0)?, row.get(1)?)))
        .optional()
}

/// Deletes drafts older than a week: uploaded while editing, but never saved with a recipe.
/// Returns how many were deleted.
pub fn delete_old_drafts(conn: &Connection) -> Result<usize> {
    conn.execute(
        "DELETE FROM images WHERE recipe IS NULL AND created < unixepoch() - 7 * 24 * 60 * 60",
        [],
    )
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
        let conn = fresh();

        let id = insert_recipe(&conn, "Pannenkoeken", "- bloem", "Bakken.").unwrap();
        let (summary, ingredients) = &list_recipes(&conn).unwrap()[0];
        assert_eq!(
            (summary.title.as_str(), ingredients.as_str()),
            ("Pannenkoeken", "- bloem")
        );
        assert_eq!(random_recipe_id(&conn).unwrap(), Some(id));

        delete_recipe(&conn, id).unwrap();
        assert_eq!(random_recipe_id(&conn).unwrap(), None);

        // The deleted id isn't given out again
        assert_eq!(insert_recipe(&conn, "Wafels", "", "").unwrap(), id + 1);
    }

    #[test]
    fn deleting_shrinks_the_file() {
        let path = std::env::temp_dir().join(format!("nom_vacuum_{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let size = || std::fs::metadata(&path).unwrap().len();

        // A database from before auto-vacuum, with a 1 MB photo
        let mut conn = Connection::open(&path).unwrap();
        migrate(&mut conn).unwrap();
        let photo = insert_image(&conn, "image/jpeg", &vec![0; 1 << 20]).unwrap();

        enable_auto_vacuum(&conn).unwrap();
        let with_photo = size();
        conn.execute("DELETE FROM images WHERE id = ?1", [photo])
            .unwrap();

        assert!(size() < with_photo / 10, "{} -> {}", with_photo, size());
        drop(conn);
        std::fs::remove_file(&path).unwrap();
    }

    fn fresh() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn).unwrap();
        conn
    }

    fn all_images(conn: &Connection) -> Vec<(i64, Option<i64>)> {
        conn.prepare("SELECT id, recipe FROM images ORDER BY id")
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<Result<_>>()
            .unwrap()
    }

    #[test]
    fn migration_4_gives_images_to_the_recipe_that_mentions_them() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(&MIGRATIONS[..3].join("\n")).unwrap();
        conn.pragma_update(None, "user_version", 3).unwrap();

        for _ in 0..5 {
            conn.execute(
                "INSERT INTO images (mime, data) VALUES ('image/jpeg', x'00')",
                [],
            )
            .unwrap();
        }
        // `images/12` must not count as 1, a relative `images/3` counts, and 4 at the very end
        // of the text counts
        conn.execute_batch(
            "INSERT INTO recipes (id, title, ingredients, instructions) VALUES
                (1, 'Soep', '- ![](/images/2) ![](/images/12)', 'Klaar. ![](images/3)'),
                (2, 'Taart', '', '![](/images/2) ![](/images/4)');",
        )
        .unwrap();

        migrate(&mut conn).unwrap();

        // 1 and 5: mentioned nowhere, so drafts. 2: mentioned by both, so the first recipe.
        assert_eq!(
            all_images(&conn),
            [
                (1, None),
                (2, Some(1)),
                (3, Some(1)),
                (4, Some(2)),
                (5, None)
            ]
        );
        assert_eq!(get_recipe(&conn, 1).unwrap().unwrap().photos, [2, 3]);
    }

    #[test]
    fn photos_belong_to_a_recipe() {
        let conn = fresh();

        let soup = insert_recipe(&conn, "Soep", "", "").unwrap();
        let cake = insert_recipe(&conn, "Taart", "", "").unwrap();
        let photo = || insert_image(&conn, "image/jpeg", b"jpeg").unwrap();
        let (a, b, c, d) = (photo(), photo(), photo(), photo());

        set_recipe_photos(&conn, soup, &[a, b]).unwrap();
        set_recipe_photos(&conn, cake, &[c]).unwrap();
        assert_eq!(get_recipe(&conn, soup).unwrap().unwrap().photos, [a, b]);

        // Dropping b deletes it, adding draft d links it, and the cake's c can't be taken
        set_recipe_photos(&conn, soup, &[a, c, d]).unwrap();
        assert_eq!(get_recipe(&conn, soup).unwrap().unwrap().photos, [a, d]);

        // The order is kept: d becomes the header photo. A repeat changes nothing.
        set_recipe_photos(&conn, soup, &[d, a, d]).unwrap();
        assert_eq!(get_recipe(&conn, soup).unwrap().unwrap().photos, [d, a]);
        assert_eq!(get_recipe(&conn, cake).unwrap().unwrap().photos, [c]);
        assert!(get_image(&conn, b).unwrap().is_none());

        // Deleting a recipe deletes its photos, and only those
        delete_recipe(&conn, soup).unwrap();
        assert_eq!(all_images(&conn), [(c, Some(cake))]);

        // An id is never reused, so a cached /images/<id> can't turn into another image
        assert_eq!(photo(), d + 1);
    }

    #[test]
    fn deletes_old_drafts_only() {
        let conn = fresh();

        let soup = insert_recipe(&conn, "Soep", "", "").unwrap();
        let photo = || insert_image(&conn, "image/jpeg", b"jpeg").unwrap();
        let (saved, old_draft, new_draft) = (photo(), photo(), photo());
        set_recipe_photos(&conn, soup, &[saved]).unwrap();

        // A recent draft may belong to a recipe that's still being written
        assert_eq!(delete_old_drafts(&conn).unwrap(), 0);

        conn.execute(
            "UPDATE images SET created = created - 8 * 24 * 60 * 60 WHERE id <> ?1",
            [new_draft],
        )
        .unwrap();
        assert_eq!(delete_old_drafts(&conn).unwrap(), 1);
        assert!(get_image(&conn, old_draft).unwrap().is_none());
        assert_eq!(all_images(&conn), [(saved, Some(soup)), (new_draft, None)]);
    }
}
