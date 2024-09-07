use askama_axum::Template;
use axum::extract::Query;
use axum::response::IntoResponse;
use axum::{extract::State, response::Html};
use axum_extra::extract::CookieJar;

use xsu_authman::model::{Permission, Profile};

use super::{SearchHomeQuery, SearchQuery};
use crate::config::Config;
use crate::database::Database;
use crate::model::{DatabaseError, Question, QuestionResponse};

#[derive(Template)]
#[template(path = "search/homepage.html")]
struct HomepageTemplate {
    config: Config,
    profile: Option<Profile>,
    unread: usize,
    notifs: usize,
    query: String,
    driver: i8,
}

/// GET /search
pub async fn search_homepage_request(
    jar: CookieJar,
    State(database): State<Database>,
    Query(query): Query<SearchHomeQuery>,
) -> impl IntoResponse {
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database
            .auth
            .get_profile_by_unhashed(c.value_trimmed().to_string())
            .await
        {
            Ok(ua) => Some(ua),
            Err(_) => None,
        },
        None => None,
    };

    let unread = if let Some(ref ua) = auth_user {
        match database.get_questions_by_recipient(ua.id.to_owned()).await {
            Ok(unread) => unread.len(),
            Err(_) => 0,
        }
    } else {
        0
    };

    let notifs = if let Some(ref ua) = auth_user {
        database
            .auth
            .get_notification_count_by_recipient(ua.id.to_owned())
            .await
    } else {
        0
    };

    Html(
        HomepageTemplate {
            config: database.server_options.clone(),
            profile: auth_user.clone(),
            unread,
            notifs,
            query: String::new(),
            driver: query.driver,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "search/responses.html")]
struct ResponsesTemplate {
    config: Config,
    profile: Option<Profile>,
    unread: usize,
    notifs: usize,
    query: String,
    page: i32,
    driver: i8,
    // search-specific
    results: Vec<(Question, QuestionResponse, usize, usize)>,
    is_powerful: bool, // at least "manager"
    is_helper: bool,   // at least "helper"
}

/// GET /search/responses
pub async fn search_responses_request(
    jar: CookieJar,
    State(database): State<Database>,
    Query(query): Query<SearchQuery>,
) -> impl IntoResponse {
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database
            .auth
            .get_profile_by_unhashed(c.value_trimmed().to_string())
            .await
        {
            Ok(ua) => Some(ua),
            Err(_) => None,
        },
        None => None,
    };

    let unread = if let Some(ref ua) = auth_user {
        match database.get_questions_by_recipient(ua.id.to_owned()).await {
            Ok(unread) => unread.len(),
            Err(_) => 0,
        }
    } else {
        0
    };

    let notifs = if let Some(ref ua) = auth_user {
        database
            .auth
            .get_notification_count_by_recipient(ua.id.to_owned())
            .await
    } else {
        0
    };

    // search results
    let results = match database
        .get_responses_searched_paginated(query.page, query.q.clone())
        .await
    {
        Ok(responses) => responses,
        Err(e) => return Html(e.to_html(database)),
    };

    // permissions
    let mut is_helper: bool = false;
    let is_powerful = if let Some(ref ua) = auth_user {
        let group = match database.auth.get_group_by_id(ua.group).await {
            Ok(g) => g,
            Err(_) => return Html(DatabaseError::Other.to_html(database)),
        };

        is_helper = group.permissions.contains(&Permission::Helper);
        group.permissions.contains(&Permission::Manager)
    } else {
        false
    };

    // render
    Html(
        ResponsesTemplate {
            config: database.server_options.clone(),
            profile: auth_user.clone(),
            unread,
            notifs,
            query: query.q,
            page: query.page,
            driver: 0,
            // search-specific
            results,
            is_powerful,
            is_helper,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "search/responses.html")]
struct PostsTemplate {
    config: Config,
    profile: Option<Profile>,
    unread: usize,
    notifs: usize,
    query: String,
    page: i32,
    driver: i8,
    // search-specific
    results: Vec<(Question, QuestionResponse, usize, usize)>,
    is_powerful: bool, // at least "manager"
    is_helper: bool,   // at least "helper"
}

/// GET /search/posts
pub async fn search_posts_request(
    jar: CookieJar,
    State(database): State<Database>,
    Query(query): Query<SearchQuery>,
) -> impl IntoResponse {
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database
            .auth
            .get_profile_by_unhashed(c.value_trimmed().to_string())
            .await
        {
            Ok(ua) => Some(ua),
            Err(_) => None,
        },
        None => None,
    };

    let unread = if let Some(ref ua) = auth_user {
        match database.get_questions_by_recipient(ua.id.to_owned()).await {
            Ok(unread) => unread.len(),
            Err(_) => 0,
        }
    } else {
        0
    };

    let notifs = if let Some(ref ua) = auth_user {
        database
            .auth
            .get_notification_count_by_recipient(ua.id.to_owned())
            .await
    } else {
        0
    };

    // search results
    let results = match database
        .get_posts_searched_paginated(query.page, query.q.clone())
        .await
    {
        Ok(responses) => responses,
        Err(e) => return Html(e.to_html(database)),
    };

    // permissions
    let mut is_helper: bool = false;
    let is_powerful = if let Some(ref ua) = auth_user {
        let group = match database.auth.get_group_by_id(ua.group).await {
            Ok(g) => g,
            Err(_) => return Html(DatabaseError::Other.to_html(database)),
        };

        is_helper = group.permissions.contains(&Permission::Helper);
        group.permissions.contains(&Permission::Manager)
    } else {
        false
    };

    // render
    Html(
        PostsTemplate {
            config: database.server_options.clone(),
            profile: auth_user.clone(),
            unread,
            notifs,
            query: query.q,
            page: query.page,
            driver: 2,
            // search-specific
            results,
            is_powerful,
            is_helper,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "search/questions.html")]
struct QuestionsTemplate {
    config: Config,
    profile: Option<Profile>,
    unread: usize,
    notifs: usize,
    query: String,
    page: i32,
    driver: i8,
    // search-specific
    results: Vec<(Question, usize, usize)>,
    is_helper: bool, // at least "helper"
}

/// GET /search/questions
pub async fn search_questions_request(
    jar: CookieJar,
    State(database): State<Database>,
    Query(query): Query<SearchQuery>,
) -> impl IntoResponse {
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database
            .auth
            .get_profile_by_unhashed(c.value_trimmed().to_string())
            .await
        {
            Ok(ua) => Some(ua),
            Err(_) => None,
        },
        None => None,
    };

    let unread = if let Some(ref ua) = auth_user {
        match database.get_questions_by_recipient(ua.id.to_owned()).await {
            Ok(unread) => unread.len(),
            Err(_) => 0,
        }
    } else {
        0
    };

    let notifs = if let Some(ref ua) = auth_user {
        database
            .auth
            .get_notification_count_by_recipient(ua.id.to_owned())
            .await
    } else {
        0
    };

    // search results
    let results = match database
        .get_global_questions_searched_paginated(query.page, query.q.clone())
        .await
    {
        Ok(responses) => responses,
        Err(e) => return Html(e.to_html(database)),
    };

    // permissions
    let is_helper = if let Some(ref ua) = auth_user {
        let group = match database.auth.get_group_by_id(ua.group).await {
            Ok(g) => g,
            Err(_) => return Html(DatabaseError::Other.to_html(database)),
        };

        group.permissions.contains(&Permission::Helper)
    } else {
        false
    };

    // render
    Html(
        QuestionsTemplate {
            config: database.server_options.clone(),
            profile: auth_user.clone(),
            unread,
            notifs,
            query: query.q,
            page: query.page,
            driver: 1,
            // search-specific
            results,
            is_helper,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "search/users.html")]
struct UsersTemplate {
    config: Config,
    profile: Option<Profile>,
    unread: usize,
    notifs: usize,
    query: String,
    page: i32,
    driver: i8,
    // search-specific
    results: Vec<Profile>,
}

/// GET /search/users
pub async fn search_users_request(
    jar: CookieJar,
    State(database): State<Database>,
    Query(query): Query<SearchQuery>,
) -> impl IntoResponse {
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database
            .auth
            .get_profile_by_unhashed(c.value_trimmed().to_string())
            .await
        {
            Ok(ua) => Some(ua),
            Err(_) => None,
        },
        None => None,
    };

    let unread = if let Some(ref ua) = auth_user {
        match database.get_questions_by_recipient(ua.id.to_owned()).await {
            Ok(unread) => unread.len(),
            Err(_) => 0,
        }
    } else {
        0
    };

    let notifs = if let Some(ref ua) = auth_user {
        database
            .auth
            .get_notification_count_by_recipient(ua.id.to_owned())
            .await
    } else {
        0
    };

    // search results
    let results = match database
        .get_profiles_searched_paginated(query.page, query.q.clone())
        .await
    {
        Ok(responses) => responses,
        Err(e) => return Html(e.to_html(database)),
    };

    // render
    Html(
        UsersTemplate {
            config: database.server_options.clone(),
            profile: auth_user.clone(),
            unread,
            notifs,
            query: query.q,
            page: query.page,
            driver: 3,
            // search-specific
            results,
        }
        .render()
        .unwrap(),
    )
}
