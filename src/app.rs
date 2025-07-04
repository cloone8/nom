use leptos::logging::log;
use leptos::prelude::*;
use leptos::reactive::spawn_local;
use leptos_meta::{MetaTags, Stylesheet, Title, provide_meta_context};
use leptos_router::hooks::use_params;
use leptos_router::path;
use leptos_router::{
    StaticSegment,
    components::{Route, Router, Routes},
    params::Params,
};

use crate::recipe::{Recipe, RecipeComponent};

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone() />
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[server]
pub async fn log_on_server(msg: String) -> Result<(), ServerFnError> {
    log!("Message: {msg}");
    Ok(())
}

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/nom_front.css"/>

        // sets the document title
        <Title text="NomNomNom"/>

        // content for this welcome page
        <Router>
            <main>
                <Routes fallback=|| "Page not found.".into_view()>
                    <Route path=path!("/") view=HomePage/>
                    <Route path=path!("/recipe/:id") view=RecipePage/>
                </Routes>
            </main>
        </Router>
    }
}

/// Renders the home page of your application.
#[component]
fn HomePage() -> impl IntoView {
    view! {
        <h1>"Het NomNomNom Receptenboek"</h1>
        <a href="/recipe/test">"Test recept"</a>
    }
}

#[derive(Debug, Params, PartialEq)]
struct RecipeArgs {
    id: Option<String>,
}

#[component]
fn RecipePage() -> impl IntoView {
    let id = move || {
        use_params::<RecipeArgs>()
            .read()
            .as_ref()
            .ok()
            .and_then(|params| params.id.clone())
            .unwrap()
    };

    let test_recipe = Recipe::test();

    view! {
        <h1>"Het NomNomNom Receptenboek " {id}</h1>
        <RecipeComponent recipe={test_recipe}/>
        <a href="/">"Terug"</a>
    }
}
