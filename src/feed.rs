use crate::sessions::SessionUser;
use askama::Template;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    Form,
};
use serde::Deserialize;
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
