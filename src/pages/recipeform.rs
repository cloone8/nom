use leptos::form::ActionForm;
use leptos::html;
use leptos::prelude::*;
use leptos_router::components::A;

use super::ErrorMessage;
use crate::recipe::{RecipeSource, SaveRecipe};

/// The form for a new recipe (`recipe` is `None`) or for editing one. Works without WASM too,
/// except for the photo buttons.
#[component]
pub fn RecipeForm(recipe: Option<RecipeSource>, submit_label: &'static str) -> impl IntoView {
    let save = ServerAction::<SaveRecipe>::new();
    let error = move || {
        save.value()
            .get()
            .and_then(Result::err)
            .map(|error| view! { <ErrorMessage error/> })
    };

    let id = recipe.as_ref().map(|r| r.id);
    let cancel_href = id.map_or("/".to_string(), |id| format!("/recipe/{id}"));
    let RecipeSource {
        title,
        ingredients,
        instructions,
        photos,
        ..
    } = recipe.unwrap_or_default();
    let ingredient_rows = ingredients.lines().count() + 2;

    // The recipe's photos as they'll be saved; see `save_recipe`
    let photos = RwSignal::new(photos);
    let photo_ids = move || {
        photos
            .get()
            .iter()
            .map(i64::to_string)
            .collect::<Vec<_>>()
            .join(",")
    };

    // Photos still uploading; saving now would leave them out
    let uploading = RwSignal::new(0_u32);

    let ingredients_ref = NodeRef::<html::Textarea>::new();
    let instructions_ref = NodeRef::<html::Textarea>::new();

    view! {
        <ActionForm action=save>
            {id.map(|id| view! { <input type="hidden" name="id" value=id/> })}
            <input type="hidden" name="photos" value=photo_ids/>
            <label>
                "Titel"
                // `pattern`: the server refuses a title of only spaces, and without JS that
                // reloads the form, losing what was typed
                <input
                    type="text"
                    name="title"
                    required
                    pattern=r".*\S.*"
                    title="Een recept heeft een titel nodig"
                    value=title
                />
            </label>
            <label>
                "Ingrediënten"
                <textarea
                    name="ingredients"
                    placeholder="- 500 g bloem"
                    rows=ingredient_rows
                    node_ref=ingredients_ref
                >
                    {ingredients}
                </textarea>
            </label>
            <label>
                "Bereiding"
                <textarea name="instructions" node_ref=instructions_ref>
                    {instructions}
                </textarea>
            </label>
            <MarkdownHelp/>
            <PhotoEditor photos uploading ingredients_ref instructions_ref/>
            {error}
            <div class="form-actions">
                <A attr:class="link-button" href=cancel_href>"Annuleer"</A>
                <input
                    class="link-button button-positive"
                    type="submit"
                    value=submit_label
                    disabled=move || { save.pending().get() || uploading.get() != 0 }
                />
            </div>
        </ActionForm>
    }
}

/// The recipe's photos: upload, pick the header photo (the first), remove. Changes are saved
/// with the form. Needs the WASM, so it only shows once that runs.
#[component]
fn PhotoEditor(
    photos: RwSignal<Vec<i64>>,
    uploading: RwSignal<u32>,
    ingredients_ref: NodeRef<html::Textarea>,
    instructions_ref: NodeRef<html::Textarea>,
) -> impl IntoView {
    let error = RwSignal::new(None::<String>);

    // Effects never run on the server
    let hydrated = RwSignal::new(false);
    Effect::new(move |_| hydrated.set(true));

    let on_upload = move |ev: leptos::ev::Event| {
        let input = event_target::<web_sys::HtmlInputElement>(&ev);
        let file = input.files().and_then(|files| files.get(0));
        // So picking the same file again still counts as a change
        input.set_value("");

        if let Some(file) = file {
            error.set(None);
            uploading.update(|n| *n += 1);

            leptos::task::spawn_local(async move {
                let result = crate::images::upload(file).await;
                uploading.update(|n| *n -= 1);

                match result {
                    Ok(id) => photos.update(|photos| photos.push(id)),
                    Err(message) => error.set(Some(message)),
                }
            });
        }
    };

    let make_header = move |id: i64| {
        photos.update(|photos| {
            photos.retain(|&photo| photo != id);
            photos.insert(0, id);
        })
    };

    let remove = move |id: i64| {
        photos.update(|photos| photos.retain(|&photo| photo != id));

        // Older recipes may show a photo in the text; don't leave a broken image there
        for field in [ingredients_ref, instructions_ref] {
            if let Some(textarea) = field.get_untracked() {
                crate::images::remove_from_text(&textarea, id);
            }
        }
    };

    view! {
        <Show when=move || hydrated.get()>
            <div class="photo-editor">
                <strong>"Foto's"</strong>
                <div class="thumbnails">
                    <For each=move || photos.get() key=|id| *id let:id>
                        <figure>
                            <img src=format!("/images/{id}") alt=""/>
                            {move || {
                                if photos.with(|photos| photos.first() == Some(&id)) {
                                    view! { <span class="header-badge">"Hoofdfoto"</span> }.into_any()
                                } else {
                                    // `type="button"`, or these would submit the form
                                    view! {
                                        <button
                                            type="button"
                                            class="link-button"
                                            on:click=move |_| make_header(id)
                                        >
                                            "Maak hoofdfoto"
                                        </button>
                                    }
                                        .into_any()
                                }
                            }}
                            <button
                                type="button"
                                class="link-button button-negative"
                                on:click=move |_| remove(id)
                            >
                                "Verwijder"
                            </button>
                        </figure>
                    </For>
                </div>
                <div class="photo-upload">
                    // No `name`, so the form doesn't submit the file itself
                    <label class="link-button">
                        "📷 Foto toevoegen"
                        <input type="file" accept="image/*" hidden on:change=on_upload/>
                    </label>
                    {move || {
                        let status = if uploading.get() > 0 {
                            Some("Foto uploaden...".to_string())
                        } else {
                            error.get()
                        };

                        status.map(|status| view! { <span class="hint">{status}</span> })
                    }}
                </div>
                <p class="hint">"De hoofdfoto komt bovenaan het recept, de andere eronder."</p>
            </div>
        </Show>
    }
}

/// The most common Markdown, folded away until needed.
#[component]
fn MarkdownHelp() -> impl IntoView {
    view! {
        <details class="markdown-help">
            <summary>"Opmaak-hulp"</summary>
            <table>
                <tr><td><code>"- melk"</code></td><td>"lijstje"</td></tr>
                <tr><td><code>"1. Snijd de ui"</code></td><td>"genummerde stappen"</td></tr>
                <tr><td><code>"**heet**"</code></td><td><strong>"vet"</strong></td></tr>
                <tr><td><code>"*optioneel*"</code></td><td><em>"schuin"</em></td></tr>
                <tr><td><code>"## Saus"</code></td><td>"kopje"</td></tr>
                <tr><td><code>"[tekst](https://...)"</code></td><td>"link"</td></tr>
                <tr><td>"lege regel"</td><td>"nieuwe alinea"</td></tr>
            </table>
        </details>
    }
}
