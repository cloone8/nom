use leptos::prelude::*;
use leptos_meta::{MetaTags, Stylesheet, Title, provide_meta_context};
use leptos_router::components::{Route, Router, Routes};
use leptos_router::path;

use crate::pages::home::HomePage;
use crate::pages::newrecipe::NewRecipePage;
use crate::pages::recipe::RecipePage;

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
            <nav>
                <NavBar/>
            </nav>
            <main>
                <Routes fallback=|| "Page not found.".into_view()>
                    <Route path=path!("/") view=HomePage/>
                    <Route path=path!("/recipe/:id") view=RecipePage/>
                    <Route path=path!("/new") view=NewRecipePage/>
                </Routes>
            </main>
        </Router>
    }
}

#[component]
fn NavBar() -> impl IntoView {
    view! {
        <a href="/">"Home"</a><br/>
        <a href="/new">"Nieuw recept"</a><br/>
        <a href="/recipe/test">"Test recept"</a><br/>
    }
}
