use askama::Template;
use sqlx::SqlitePool;
use warp::http::StatusCode;

#[derive(Template)]
#[template(path = "feed.html")]
pub struct PostsPage {
    posts: Vec<Post>,
}

struct Post {
    username: String,
    content: String,
}

pub async fn get_feed(db: SqlitePool) -> Result<PostsPage, StatusCode> {
    let mut conn = db
        .acquire()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let posts = sqlx::query_as!(
        Post,
        r#"
        SELECT p.content, u.username
        FROM posts p
        JOIN users u ON p.user_id = u.id
        ORDER BY p.id DESC
        LIMIT 50
        "#
    )
    .fetch_all(&mut *conn)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(PostsPage { posts })
}
