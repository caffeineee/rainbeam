use crate::database::Database;
use crate::model::{DatabaseError, IpBanCreate};
use databeam::prelude::DefaultReturn;

use axum::response::IntoResponse;
use axum::{
    extract::{Path, State},
    Json,
};
use axum_extra::extract::cookie::CookieJar;

/// Create a ipban
pub async fn create_request(
    jar: CookieJar,
    State(database): State<Database>,
    Json(props): Json<IpBanCreate>,
) -> impl IntoResponse {
    // get user from token
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database.get_profile_by_unhashed(c.value_trimmed()).await {
            Ok(ua) => ua,
            Err(e) => return Json(e.to_json()),
        },
        None => return Json(DatabaseError::NotAllowed.to_json()),
    };

    // return
    match database.create_ipban(props, auth_user).await {
        Ok(_) => Json(DefaultReturn {
            success: true,
            message: "Acceptable".to_string(),
            payload: (),
        }),
        Err(e) => Json(e.to_json()),
    }
}

/// Delete an ipban
pub async fn delete_request(
    jar: CookieJar,
    Path(id): Path<String>,
    State(database): State<Database>,
) -> impl IntoResponse {
    // get user from token
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database.get_profile_by_unhashed(c.value_trimmed()).await {
            Ok(ua) => ua,
            Err(e) => return Json(e.to_json()),
        },
        None => return Json(DatabaseError::NotAllowed.to_json()),
    };

    // return
    match database.delete_ipban(&id, auth_user).await {
        Ok(_) => Json(DefaultReturn {
            success: true,
            message: "Acceptable".to_string(),
            payload: (),
        }),
        Err(e) => Json(e.to_json()),
    }
}
