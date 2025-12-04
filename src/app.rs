use leptos::prelude::*;
use leptos::reactive::spawn_local;
use leptos_meta::{MetaTags, Stylesheet, Title, provide_meta_context};
use leptos_router::components::{A, Route, Router, Routes};
use leptos_router::hooks::use_url;
use leptos_router::{NavigateOptions, path};
use web_sys::MouseEvent;

use crate::pages::editrecipe::EditRecipePage;
use crate::pages::home::HomePage;
use crate::pages::newrecipe::NewRecipePage;
use crate::pages::recipe::RecipePage;
use crate::pages::trmnl::TrmnlPage;
use crate::recipe::random_recipe;

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
            <NavBar/>
            <main>
                <Routes fallback=|| "Page not found.".into_view()>
                    <Route path=path!("/") view=HomePage/>
                    <Route path=path!("/trmnl") view=TrmnlPage/>
                    <Route path=path!("/recipe/:id") view=RecipePage/>
                    <Route path=path!("/edit/:id") view=EditRecipePage/>
                    <Route path=path!("/new") view=NewRecipePage/>
                </Routes>
            </main>
        </Router>
    }
}

#[component]
fn NavBar() -> impl IntoView {
    let url = use_url();
    let url_is_trmnl = move || url.get().path() == "/trmnl";

    let random_recipe = |ev: MouseEvent| {
        ev.prevent_default();

        spawn_local(async {
            let random_recipe_id = random_recipe().await.unwrap();

            match random_recipe_id {
                Some(id) => {
                    let navigate = leptos_router::hooks::use_navigate();

                    navigate(format!("/recipe/{id}").as_str(), NavigateOptions::default());
                }
                None => {
                    web_sys::window()
                        .unwrap()
                        .alert_with_message("Er zijn nog geen recepted. Voeg er eentje toe!")
                        .unwrap();
                }
            }
        });
    };

    view! {
        {
            move || if url_is_trmnl() {
                ().into_any()
            } else {
                view! {
                    <nav class="nom-navbar">
                        <A class:link-button href="/">"Home"</A>
                        <A class:link-button href="/new">"Nieuw recept"</A>
                        <button class:link-button on:click=random_recipe>"Random"</button>
                    </nav>
                }.into_any()
            }
        }
    }
}
