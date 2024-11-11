//! Responds to API requests
use crate::database::Database;
use crate::model::DatabaseError;
use databeam::DefaultReturn;

use axum::response::IntoResponse;
use axum::{
    routing::{delete, get, post},
    Json, Router,
};

pub mod general;
pub mod ipbans;
pub mod ipblocks;
pub mod me;
pub mod notifications;
pub mod profile;
pub mod relationships;
pub mod warnings;

pub async fn not_found() -> impl IntoResponse {
    Json(DefaultReturn::<u16> {
        success: false,
        message: DatabaseError::NotFound.to_string(),
        payload: 404,
    })
}

pub fn routes(database: Database) -> Router {
    Router::new()
        // relationships
        .route(
            "/relationships/follow/:id",
            post(relationships::follow_request),
        )
        .route(
            "/relationships/friend/:id",
            post(relationships::friend_request),
        )
        .route(
            "/relationships/block/:id",
            post(relationships::block_request),
        )
        .route(
            "/relationships/current/:id",
            delete(relationships::delete_request),
        )
        // profiles
        .route(
            "/profile/:id/tokens/generate",
            post(profile::generate_token_request),
        )
        .route("/profile/:id/tokens", post(profile::update_tokens_request))
        .route("/profile/:id/tier", post(profile::update_tier_request))
        .route("/profile/:id/group", post(profile::update_group_request))
        .route(
            "/profile/:id/password",
            post(profile::update_password_request),
        )
        .route(
            "/profile/:id/username",
            post(profile::update_username_request),
        )
        .route(
            "/profile/:id/metadata",
            post(profile::update_metdata_request),
        )
        .route("/profile/:id/badges", post(profile::update_badges_request))
        .route("/profile/:id/banner", get(profile::banner_request))
        .route("/profile/:id/avatar", get(profile::avatar_request))
        .route("/profile/:id", delete(profile::delete_request))
        .route("/profile/:id", get(profile::get_request))
        // notifications
        .route("/notifications/:id", delete(notifications::delete_request))
        .route(
            "/notifications/clear",
            delete(notifications::delete_all_request),
        )
        // warnings
        .route("/warnings", post(warnings::create_request))
        .route("/warnings/:id", delete(warnings::delete_request))
        // ipbans
        .route("/ipbans", post(ipbans::create_request))
        .route("/ipbans/:id", delete(ipbans::delete_request))
        // ipblocks
        .route("/ipblocks", post(ipblocks::create_request))
        .route("/ipblocks/:id", delete(ipblocks::delete_request))
        // me
        .route("/me/tokens/generate", post(me::generate_token_request))
        .route("/me/tokens", post(me::update_tokens_request))
        .route("/me/delete", post(me::delete_request))
        .route("/me", get(me::get_request))
        // account
        .route("/register", post(general::create_request))
        .route("/login", post(general::login_request))
        .route("/callback", get(general::callback_request))
        .route("/logout", post(general::logout_request))
        .route("/untag", post(general::remove_tag))
        // ...
        .with_state(database)
}