mod feed;
mod sessions;
mod users;

use axum::{routing::get, Router};
use sqlx::SqlitePool;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tower_sessions::{cookie::time::Duration, Expiry, MemoryStore, SessionManagerLayer};
use tracing::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let db_conenction_pool = SqlitePool::connect("sqlite://db.sqlite").await.unwrap();
    sqlx::migrate!()
        .run(&db_conenction_pool)
        .await
        .unwrap();

    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false)
        .with_expiry(Expiry::OnInactivity(Duration::minutes(5)));

    let app = Router::new()
        .route("/", get(feed::get_posts).post(feed::post_post))
        .route("/signup", get(users::signup_page).post(users::post_user))
        .route("/login", get(sessions::login_page).post(sessions::login))
        .route("/htmx.min.js", get(htmx))
        .layer(TraceLayer::new_for_http())
        .layer(session_layer)
        .with_state(db_conenction_pool);

    let listener = TcpListener::bind("0.0.0.0:43561").await.unwrap();

    info!("Listening on http://{}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.unwrap();
}

async fn htmx() -> &'static str {
    include_str!("htmx.min.js")
}
