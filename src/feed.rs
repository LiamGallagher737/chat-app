use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

// The type for storing the currently conencted users and their sender channels
type Users = Arc<Mutex<Vec<mpsc::UnboundedSender<models::SseMessage>>>>;

pub mod filters {
    use super::handlers;
    use super::Users;
    use crate::{
        database::{with_db, DbPool},
        form_body,
        sessions::{with_auth, Key},
    };
    use std::sync::Arc;
    use std::sync::Mutex;
    use warp::Filter;

    pub fn routes(
        pool: DbPool,
        key: Key,
    ) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
        let users: Users = Arc::new(Mutex::new(Vec::new()));

        warp::any()
            .and(get_feed(pool.clone(), key.clone()))
            .or(post_post(pool.clone(), key.clone(), users.clone()))
            .or(get_live_feed(key, users))
    }

    /// GET /
    pub fn get_feed(
        pool: DbPool,
        key: Key,
    ) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
        warp::path::end()
            .and(warp::get())
            .and(with_db(pool))
            .and(with_auth(key))
            .and_then(handlers::get_feed)
    }

    /// POST /feed
    pub fn post_post(
        pool: DbPool,
        key: Key,
        users: Users,
    ) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
        warp::path("feed")
            .and(warp::post())
            .and(with_db(pool))
            .and(with_auth(key))
            .and(with_users(users))
            .and(form_body())
            .and_then(handlers::create_post)
    }

    pub fn get_live_feed(
        key: Key,
        users: Users,
    ) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
        warp::path("feed")
            .and(warp::get())
            .and(with_auth(key))
            .and(with_users(users))
            .map(|user, users| {
                let stream = handlers::get_live_feed(user, users);
                warp::sse::reply(warp::sse::keep_alive().stream(stream))
            })
    }

    fn with_users(
        users: Users,
    ) -> impl Filter<Extract = (Users,), Error = std::convert::Infallible> + Clone {
        warp::any().map(move || users.clone())
    }
}

mod handlers {
    use super::{models::*, templates::*, Users};
    use crate::{database::Db, sessions::User, InternalServerError};
    use futures_util::Stream;
    use log::info;
    use tokio::sync::mpsc;
    use tokio_stream::{wrappers::UnboundedReceiverStream, StreamExt};
    use warp::{filters::sse::Event, Rejection, Reply};

    pub async fn get_feed(mut db: Db, _user: User) -> Result<impl Reply, Rejection> {
        let posts = sqlx::query_as!(
            Post,
            r#"
            SELECT p.content, u.username, u.id AS user_id
            FROM posts p
            JOIN users u ON p.user_id = u.id
            ORDER BY p.id DESC
            LIMIT 50
            "#
        )
        .fetch_all(&mut *db)
        .await
        .map_err(|_| warp::reject::custom(InternalServerError))?;

        Ok(PostsPage { posts })
    }

    pub async fn create_post(
        mut db: Db,
        user: User,
        users: Users,
        post: NewPost,
    ) -> Result<impl Reply, Rejection> {
        info!(
            "User {:?} send a new message \"{:.32}\"",
            user.username, post.content
        );

        sqlx::query!(
            "INSERT INTO posts (user_id, content) VALUES ( ?, ? )",
            user.user_id,
            post.content
        )
        .execute(&mut *db)
        .await
        .map_err(|_| warp::reject::custom(InternalServerError))?;

        users.lock().unwrap().retain(|tx| {
            tx.send(SseMessage::Post(format!(
                "{}: {}",
                user.username, post.content
            )))
            .is_ok()
        });

        Ok(warp::reply())
    }

    pub fn get_live_feed(
        user: User,
        users: Users,
    ) -> impl Stream<Item = Result<Event, warp::Error>> + Send + 'static {
        info!("User {:?} connected to live feed", user.username);

        let (tx, rx) = mpsc::unbounded_channel();
        let rx = UnboundedReceiverStream::new(rx);

        users.lock().unwrap().push(tx);

        rx.map(|msg| match msg {
            SseMessage::Post(text) => Ok(Event::default().data(text)),
        })
    }
}

mod templates {
    use super::models;
    use askama::Template;

    #[derive(Template)]
    #[template(path = "feed.html")]
    pub struct PostsPage {
        pub posts: Vec<models::Post>,
    }
}

mod models {
    #[derive(Debug)]
    pub struct Post {
        pub user_id: u64,
        pub username: String,
        pub content: String,
    }

    #[derive(Debug, serde::Deserialize)]
    pub struct NewPost {
        pub content: String,
    }

    pub enum SseMessage {
        Post(String),
    }
}
