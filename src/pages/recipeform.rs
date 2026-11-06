use leptos::form::ActionForm;
use leptos::html;
use leptos::prelude::*;
use leptos_router::components::A;

use super::ErrorMessage;
use crate::recipe::{RecipeSource, SaveRecipe};

#[derive(Clone, Copy)]
enum Field {
    Ingredients,
    Instructions,
}

/// The form for a new recipe (`recipe` is `None`) or for editing one. Works without WASM too,
/// except for adding photos.
#[component]
pub fn RecipeForm(recipe: Option<RecipeSource>, submit_label: &'static str) -> impl IntoView {
    let ingredients_ref = NodeRef::<html::Textarea>::new();
    let instructions_ref = NodeRef::<html::Textarea>::new();
    // A photo goes into the field last typed in
    let photo_field = RwSignal::new(Field::Instructions);
    let photo_status = RwSignal::new(None::<String>);

    // Uploading needs the WASM, so the button only shows once it runs (effects never run on
    // the server)
    let hydrated = RwSignal::new(false);
    Effect::new(move |_| hydrated.set(true));

    let on_photo = move |ev: leptos::ev::Event| {
        #[cfg(feature = "hydrate")]
        {
            let input = event_target::<web_sys::HtmlInputElement>(&ev);
            let file = input.files().and_then(|files| files.get(0));
            // So picking the same file again still counts as a change
            input.set_value("");

            let textarea = match photo_field.get_untracked() {
                Field::Ingredients => ingredients_ref,
                Field::Instructions => instructions_ref,
            }
            .get_untracked();

            if let (Some(file), Some(textarea)) = (file, textarea) {
                photo_status.set(Some("Foto uploaden...".to_string()));

                leptos::task::spawn_local(async move {
                    let result = crate::images::upload_and_insert(file, textarea).await;
                    photo_status.set(result.err());
                });
            }
        }

        #[cfg(not(feature = "hydrate"))]
        let _ = ev;
    };

    let save = ServerAction::<SaveRecipe>::new();
    let error = move || match save.value().get() {
        Some(Err(error)) => Some(view! { <ErrorMessage error/> }),
        _ => None,
    };

    let id = recipe.as_ref().map(|r| r.id);
    let cancel_href = id.map_or("/".to_string(), |id| format!("/recipe/{id}"));
    let RecipeSource {
        title,
        ingredients,
        instructions,
        ..
    } = recipe.unwrap_or(RecipeSource {
        id: 0,
        title: String::new(),
        ingredients: String::new(),
        instructions: String::new(),
    });
    let ingredient_rows = ingredients.lines().count() + 2;

    view! {
        <ActionForm action=save>
            {id.map(|id| view! { <input type="hidden" name="id" value=id/> })}
            <label>
                "Titel"
                <input type="text" name="title" required value=title/>
            </label>
            <label>
                "Ingrediënten"
                <textarea
                    name="ingredients"
                    placeholder="- 500 g bloem"
                    rows=ingredient_rows
                    node_ref=ingredients_ref
                    on:focus=move |_| photo_field.set(Field::Ingredients)
                >
                    {ingredients}
                </textarea>
            </label>
            <label>
                "Bereiding"
                <textarea
                    name="instructions"
                    node_ref=instructions_ref
                    on:focus=move |_| photo_field.set(Field::Instructions)
                >
                    {instructions}
                </textarea>
            </label>
            <Show when=move || hydrated.get()>
                <div class="photo">
                    // No `name`, so the form doesn't submit the file itself
                    <label class="link-button">
                        "📷 Foto toevoegen"
                        <input type="file" accept="image/*" hidden on:change=on_photo/>
                    </label>
                    {move || photo_status.get().map(|status| view! { <span class="hint">{status}</span> })}
                </div>
            </Show>
            <p class="hint">
                "Opmaak met Markdown: " <code>"- lijstje"</code> ", " <code>"**vet**"</code> ", "
                <code>"## kopje"</code>
            </p>
            {error}
            <div class="form-actions">
                <A attr:class="link-button" href=cancel_href>"Annuleer"</A>
                <input
                    class="link-button button-positive"
                    type="submit"
                    value=submit_label
                    disabled=save.pending()
                />
            </div>
        </ActionForm>
    }
}
