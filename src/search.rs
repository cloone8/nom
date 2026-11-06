//! Recipe search over titles and ingredients. Server only.
//!
//! Case and accents are ignored everywhere ("creme" finds "Crème"). A recipe is found if:
//! 1. its title matches the whole search fuzzily, so typos like "spagetti" still find
//!    "Spaghetti", or
//! 2. every word of the search appears in its title or ingredients. Words are matched anywhere
//!    inside a word, because Dutch writes compounds as one word ("soep" finds "Erwtensoep").
//!
//! Fuzzy title matches come first, best match first. Then recipes with every word in the title,
//! then the rest. Within those two groups the given order is kept.

use nucleo_matcher::chars::{normalize, to_lower_case};
use nucleo_matcher::pattern::{AtomKind, CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};

use crate::recipe::RecipeSummary;

/// Shorter searches aren't matched fuzzily: "ei" fuzzily matches "Spaghetti" (e…i).
const MIN_FUZZY_CHARS: usize = 3;

/// Shorter words must be at the start of a word, so "ui" finds "uien" but not "suiker".
const MIN_INFIX_CHARS: usize = 3;

/// Filters `recipes` (each with its ingredients) by `search`, ordered as described in the module
/// docs. An empty search returns all recipes.
pub fn search(recipes: Vec<(RecipeSummary, String)>, search: &str) -> Vec<RecipeSummary> {
    let folded_search = fold(search);
    let words: Vec<&str> = folded_search.split_whitespace().collect();

    if words.is_empty() {
        return recipes.into_iter().map(|(recipe, _)| recipe).collect();
    }

    let fuzzy = (search.trim().chars().count() >= MIN_FUZZY_CHARS).then(|| {
        Pattern::new(
            search,
            CaseMatching::Ignore,
            Normalization::Smart,
            AtomKind::Fuzzy,
        )
    });
    let mut matcher = Matcher::new(Config::DEFAULT);
    let mut buf = Vec::new();

    let mut fuzzy_title_matches = Vec::new();
    let mut title_matches = Vec::new();
    let mut other_matches = Vec::new();

    for (recipe, ingredients) in recipes {
        let fuzzy_score = fuzzy
            .as_ref()
            .and_then(|p| p.score(Utf32Str::new(&recipe.title, &mut buf), &mut matcher));

        if let Some(score) = fuzzy_score {
            fuzzy_title_matches.push((score, recipe));
            continue;
        }

        let title = fold(&recipe.title);
        let ingredients = fold(&ingredients);

        if words.iter().all(|word| contains_word(&title, word)) {
            title_matches.push(recipe);
        } else if words
            .iter()
            .all(|word| contains_word(&title, word) || contains_word(&ingredients, word))
        {
            other_matches.push(recipe);
        }
    }

    // Stable, so equal scores keep the given order
    fuzzy_title_matches.sort_by(|a, b| b.0.cmp(&a.0));

    fuzzy_title_matches
        .into_iter()
        .map(|(_, recipe)| recipe)
        .chain(title_matches)
        .chain(other_matches)
        .collect()
}

/// Lowercase, without accents.
fn fold(text: &str) -> String {
    text.chars().map(|c| normalize(to_lower_case(c))).collect()
}

fn contains_word(text: &str, word: &str) -> bool {
    if word.chars().count() >= MIN_INFIX_CHARS {
        return text.contains(word);
    }

    text.match_indices(word).any(|(i, _)| {
        !text[..i]
            .chars()
            .next_back()
            .is_some_and(char::is_alphanumeric)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const RECIPES: &[(&str, &str)] = &[
        // Sorted by title, as `db::list_recipes` returns them
        (
            "Crème brûlée",
            "- 500 ml room\n- 5 eidooiers\n- 100 g suiker",
        ),
        (
            "Erwtensoep",
            "- 500 g spliterwten\n- 1 rookworst\n- 2 wortels\n- 1 ui",
        ),
        ("Kippensoep", "- 1 soepkip\n- 2 wortels\n- 1 prei"),
        ("Lasagne", "- lasagnebladen\n- 500 g gehakt"),
        ("Pannenkoeken", "- 250 g bloem\n- 2 eieren\n- 500 ml melk"),
        (
            "Spaghetti bolognese",
            "- 400 g spaghetti\n- 1 ui\n- 2 tenen knoflook",
        ),
        (
            "Stamppot boerenkool",
            "- 1 kg aardappelen\n- 1 rookworst\n- melk",
        ),
        ("Uiensoep", "- 1 kg uien\n- stokbrood\n- gruyère"),
    ];

    fn titles(query: &str) -> Vec<String> {
        let recipes = RECIPES
            .iter()
            .enumerate()
            .map(|(id, (title, ingredients))| {
                let summary = RecipeSummary {
                    id: id as i64,
                    title: title.to_string(),
                };

                (summary, ingredients.to_string())
            })
            .collect();

        search(recipes, query)
            .into_iter()
            .map(|r| r.title)
            .collect()
    }

    #[test]
    fn empty_search_returns_everything_in_order() {
        assert_eq!(titles("  ").len(), RECIPES.len());
        assert_eq!(titles("")[0], "Crème brûlée");
    }

    #[test]
    fn compound_words() {
        assert_eq!(titles("soep"), ["Erwtensoep", "Kippensoep", "Uiensoep"]);
        assert_eq!(titles("erwt soep"), ["Erwtensoep"]);
    }

    #[test]
    fn case_and_accents_are_ignored() {
        assert_eq!(titles("CREME"), ["Crème brûlée"]);
        // At the end of the text, where nucleo-matcher 0.3.1's own substring mode misses it
        assert_eq!(titles("gruyere"), ["Uiensoep"]);
        assert_eq!(titles("Gruyère"), ["Uiensoep"]);
    }

    #[test]
    fn typos_in_titles() {
        assert_eq!(titles("spagetti"), ["Spaghetti bolognese"]);
        assert_eq!(titles("pannekoek"), ["Pannenkoeken"]);
    }

    #[test]
    fn ingredients_and_title_priority() {
        assert_eq!(titles("rookworst"), ["Erwtensoep", "Stamppot boerenkool"]);
        // Words may be split over title and ingredients
        assert_eq!(titles("soep wortel"), ["Erwtensoep", "Kippensoep"]);
        // The title match comes first, then the ingredient matches by title
        assert_eq!(
            titles("ui"),
            ["Uiensoep", "Erwtensoep", "Spaghetti bolognese"]
        );
    }

    #[test]
    fn short_words_must_start_a_word() {
        // Not "suiker" (Crème brûlée), but "uien"
        assert!(!titles("ui").contains(&"Crème brûlée".to_string()));
        // "eidooiers" and "eieren", not "prei" (Kippensoep) or "Spaghetti" fuzzily
        assert_eq!(titles("ei"), ["Crème brûlée", "Pannenkoeken"]);
    }

    #[test]
    fn nothing_found() {
        assert!(titles("ananas").is_empty());
        assert!(titles("soep room").is_empty());
    }
}
