//! HTTP API. Everything hangs off `/api`; the SPA serves all other paths.

pub mod activity;
pub mod auth;
pub mod calendar;
pub mod clients;
pub mod indexers;
pub mod library;
pub mod lookup;
pub mod proxy;
pub mod releases;
pub mod requests;
pub mod settings;
pub mod setup;
pub mod system;
pub mod torrents;
pub mod users;

use axum::routing::{delete, get, patch, post, put};
use axum::Router;

use crate::AppState;
use std::sync::Arc;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        // public
        .route("/status", get(system::status))
        .route("/setup", post(setup::setup))
        .route("/auth/login", post(auth::login))
        // session
        .route("/auth/logout", post(auth::logout))
        .route("/auth/me", get(auth::me))
        .route("/auth/password", post(auth::change_password))
        // users (admin)
        .route("/users", get(users::list).post(users::create))
        .route("/users/{id}", put(users::update).delete(users::remove))
        .route("/users/{id}/apikey", post(users::create_apikey))
        // settings
        .route("/settings", get(settings::all))
        .route(
            "/settings/{section}",
            get(settings::get_one).put(settings::put),
        )
        .route("/settings/engine/restart", post(settings::restart_engine))
        // indexers
        .route("/indexers", get(indexers::list).post(indexers::create))
        .route("/indexers/test", post(indexers::test_new))
        .route(
            "/indexers/{id}",
            put(indexers::update).delete(indexers::remove),
        )
        .route("/indexers/{id}/test", post(indexers::test))
        // download clients
        .route("/clients", get(clients::list).post(clients::create))
        .route("/clients/test", post(clients::test_new))
        .route(
            "/clients/{id}",
            put(clients::update).delete(clients::remove),
        )
        .route("/clients/{id}/test", post(clients::test))
        // torrents
        .route("/torrents", get(torrents::list).post(torrents::add))
        .route("/torrents/file", post(torrents::add_file))
        .route("/torrents/limits", put(torrents::set_limits))
        .route(
            "/torrents/{id}",
            get(torrents::one).delete(torrents::remove),
        )
        .route("/torrents/{id}/files", get(torrents::files))
        .route("/torrents/{id}/only-files", put(torrents::set_only_files))
        .route("/torrents/{id}/pause", post(torrents::pause))
        .route("/torrents/{id}/resume", post(torrents::resume))
        // library: movies
        .route(
            "/movies",
            get(library::list_movies).post(library::add_movie),
        )
        .route(
            "/movies/{id}",
            get(library::get_movie)
                .put(library::update_movie)
                .delete(library::delete_movie),
        )
        .route("/movies/{id}/search", post(releases::search_movie_releases))
        // library: series
        .route(
            "/series",
            get(library::list_series).post(library::add_series),
        )
        .route(
            "/series/{id}",
            get(library::get_series)
                .put(library::update_series)
                .delete(library::delete_series),
        )
        .route("/series/{id}/episodes", get(library::series_episodes))
        .route("/series/{id}/refresh", post(library::refresh_series))
        .route(
            "/series/{id}/search",
            post(releases::search_series_releases),
        )
        .route("/seasons/{id}/monitor", put(library::set_season_monitored))
        .route("/episodes/{id}", patch(library::update_episode))
        .route(
            "/episodes/{id}/search",
            post(releases::search_episode_releases),
        )
        // library: music
        .route(
            "/artists",
            get(library::list_artists).post(library::add_artist),
        )
        .route(
            "/artists/{id}",
            get(library::get_artist)
                .put(library::update_artist)
                .delete(library::delete_artist),
        )
        .route("/artists/{id}/albums", get(library::artist_albums))
        .route("/albums/{id}", patch(library::update_album))
        .route("/albums/{id}/search", post(releases::search_album_releases))
        // metadata lookup
        .route("/lookup/movies", get(lookup::movies))
        .route("/lookup/series", get(lookup::series))
        .route("/lookup/artists", get(lookup::artists))
        // releases + grab
        .route("/releases/grab", post(releases::grab))
        // requests
        .route("/requests", get(requests::list).post(requests::create))
        .route("/requests/{id}", delete(requests::remove))
        .route("/requests/{id}/approve", post(requests::approve))
        .route("/requests/{id}/decline", post(requests::decline))
        // activity / wanted / calendar
        .route("/activity/queue", get(activity::queue))
        .route("/activity/history", get(activity::history))
        .route("/wanted", get(library::wanted))
        .route("/calendar", get(calendar::calendar))
        .route("/scan", post(library::scan_library))
        // system
        .route("/system/info", get(system::info))
        .route("/system/logs", get(system::logs))
        .route("/system/disk", get(system::disk))
        // misc
        .route("/proxy/image", get(proxy::image))
}
