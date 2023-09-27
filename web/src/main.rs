#![allow(non_snake_case)]
use std::collections::{BTreeMap, HashMap};

use dioxus::prelude::*;
use itertools::Itertools;
use material_dioxus::{theming::Colors, MatFab, MatTheme};
use once_cell::sync::Lazy;
use reqwest::Client;
use schema::{Movie, Tag};
use sir::{css, AppStyle};

use crate::components::app_specific::{
    AddMovieDialog, CreateTagDialog, FilterCallback, FilterCard, MovieCard,
};

mod api;
mod components;

pub static CLIENT: Lazy<Client> = Lazy::new(Client::new);
pub static BASE_URL: Lazy<String> = Lazy::new(|| {
    #[cfg(target_family = "wasm")]
    let url = web_sys::window().unwrap().location().origin().unwrap();
    #[cfg(not(target_family = "wasm"))]
    let url = String::from("http://localhost:19283");
    url
});

fn main() {
    #[cfg(target_family = "wasm")]
    dioxus_web::launch(App);
    #[cfg(not(target_family = "wasm"))]
    dioxus_desktop::launch(App);
}

fn use_dark_theme(_cx: &ScopeState) -> bool {
    #[cfg(target_family = "wasm")]
    let dark_theme = dioxus_std::color_scheme::use_preferred_color_scheme(_cx)
        .map_or(false, |theme| {
            matches!(theme, dioxus_std::color_scheme::PreferredColorScheme::Dark)
        });
    #[cfg(not(target_family = "wasm"))]
    let dark_theme = true;
    dark_theme
}

#[derive(Default)]
pub struct AppState {
    fetched: bool,
    pub movies: BTreeMap<u64, Movie>,
    pub tags: HashMap<u32, Tag>,
}

pub fn use_app_state(cx: &ScopeState) -> &UseSharedState<AppState> {
    use_shared_state(cx).expect("AppState not provided")
}

fn App(cx: Scope) -> Element {
    let dark_theme = use_dark_theme(cx);
    let error = use_state(cx, || None);

    use_shared_state_provider(cx, AppState::default);
    let app_state = use_app_state(cx);
    if !app_state.read().fetched {
        app_state.write().fetched = true;
        cx.spawn({
            to_owned![error, app_state];
            async move {
                let res = api::fetch::<BTreeMap<u64, Movie>>("/movie").await;
                match res {
                    Ok(movies) => app_state.write().movies = movies,
                    Err(err) => error.set(Some(err.to_string())),
                }
            }
        });
        cx.spawn({
            to_owned![error, app_state];
            async move {
                let res = api::fetch::<HashMap<u32, Tag>>("/tag").await;
                match res {
                    Ok(tags) => app_state.write().tags = tags,
                    Err(err) => error.set(Some(err.to_string())),
                }
            }
        });
    }

    let add_movie_dialog_open = use_state(cx, || false);
    let create_tag_dialog_open = use_state(cx, || false);
    let filter = use_state(cx, || Box::new(|_: &&Movie| true) as FilterCallback);

    let fab = css!(
        "
        position: fixed;
        z-index: 5;
        bottom: 3rem;
        right: 3rem;
    "
    );

    let movie_list = rsx! {
        for movie in app_state
    .read()
    .movies
    .values()
    .filter(|m| filter.get()(m))
    .sorted_by_key(|m| &m.title)
    .cloned() {
            MovieCard {
                key: "{movie.tmdb_id}",
                movie: movie,
                create_tag_dialog_open: create_tag_dialog_open.clone()
            }
        }
    };

    // TODO: better errors
    render! {
        AppStyle {}
        MatTheme { dark_theme: None }
        if dark_theme {
            rsx! {
                MatTheme {
                    theme: Colors::DEFAULT_DARK,
                    dark_theme: None,
                }
            }
        }
        main {
            class: if dark_theme { "dark-theme" } else { "" },
            display: "flex",
            gap: "1rem",
            flex_direction: "column",
            margin: "auto",
            padding: "1rem 1rem 5rem",
            max_width: "60rem",
            FilterCard { callback: filter.clone() }
            match error.get() {
                Some(error) => rsx! { div { color: "red", "{error}" } },
                None => movie_list,
            }
        }
        span { onclick: move |_| add_movie_dialog_open.set(true), MatFab { class: "{fab}", icon: "add" } }
        AddMovieDialog { open: add_movie_dialog_open.clone(), create_tag_dialog_open: create_tag_dialog_open.clone() }
        CreateTagDialog { open: create_tag_dialog_open.clone() }
    }
}