use crate::sessions::SessionUser;
use askama::Template;
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use serde::{Deserialize, Serialize};
use sqlx::{pool::PoolConnection, Sqlite, SqlitePool};
use tower_sessions::Session;
use tracing::info;

#[derive(Template)]
#[template(path = "feed.html")]
pub struct PostsPage {
    posts: Vec<Post>,
}

struct Post {
    user: String,
    content: String,
}

#[derive(Debug)]
struct DbPost {
    id: i64,
    user_id: i64,
    content: String,
    name: String,
}

pub async fn get_posts(State(db): State<SqlitePool>) -> axum::response::Result<PostsPage> {
    let mut conn = db
        .acquire()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut posts = retrieve_posts_with_user_name(&mut conn)
        .await
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(PostsPage {
        posts: posts
            .drain(..)
            .map(|p| Post {
                user: p.name,
                content: p.content,
            })
            .collect(),
    })
}

#[derive(Deserialize, Debug)]
pub struct Input {
    content: String,
}

pub async fn post_post(
    session: Session,
    State(db): State<SqlitePool>,
    Form(input): Form<Input>,
) -> axum::response::Result<Response> {
    info!("New post from user: {input:?}");
    let mut conn = db
        .acquire()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let Some(user) = session
        .get::<SessionUser>("user")
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    else {
        return Ok(Redirect::to("/login").into_response());
    };

    if input.content.is_empty() {
        return Ok(StatusCode::BAD_REQUEST.into_response());
    }
    
    if !is_kind_message(&input.content)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        return Ok(
            Html("<p>Watch your lanuage pal</p><a href=\"/\">I'm sorry</a>").into_response(),
        );
    }

    sqlx::query!(
        "INSERT INTO posts (user_id, content) VALUES ( ?, ? )",
        user.id,
        input.content,
    )
    .execute(&mut *conn)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut posts = retrieve_posts_with_user_name(&mut conn)
        .await
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(PostsPage {
        posts: posts
            .drain(..)
            .map(|p| Post {
                user: p.name,
                content: p.content,
            })
            .collect(),
    }
    .into_response())
}

async fn retrieve_posts_with_user_name(conn: &mut PoolConnection<Sqlite>) -> Option<Vec<DbPost>> {
    sqlx::query_as!(
        DbPost,
        r#"
        SELECT p.*, u.name
        FROM posts p
        JOIN users u ON p.user_id = u.id
        ORDER BY p.id DESC
        LIMIT 50
        "#
    )
    .fetch_all(&mut **conn)
    .await
    .ok()
}

async fn is_kind_message(input: &str) -> Result<bool, reqwest::Error> {
    let client = reqwest::Client::new();
    let report: ModerationReport = client
        .post("https://despam.io/api/v1/moderate")
        .json(&ModerationRequest { input })
        .header("x-api-key", include_str!("despam.token"))
        .send()
        .await?
        .json()
        .await?;

    Ok(report.toxic < 0.6
        && report.indecent < 0.6
        && report.threat < 0.6
        && report.offensive < 0.8
        && report.erotic < 0.6
        && report.spam < 0.8)
}

#[derive(Serialize)]
struct ModerationRequest<'a> {
    input: &'a str,
}

#[derive(Deserialize, Debug)]
struct ModerationReport {
    pub toxic: f64,
    pub indecent: f64,
    pub threat: f64,
    pub offensive: f64,
    pub erotic: f64,
    pub spam: f64,
}
