pub mod filters {
    use super::handlers;
    use crate::{
        database::{with_db, DbPool},
        sessions::{with_auth, Key},
    };
    use warp::Filter;

    /// GET /
    pub fn get_feed(
        pool: DbPool,
        key: Key,
    ) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
        warp::get()
            .and(with_db(pool))
            .and(with_auth(key))
            .and_then(handlers::get_feed)
    }
}

mod handlers {
    use super::models::*;
    use crate::{database::Db, sessions::User, InternalServerError};
    use warp::{Rejection, Reply};

    pub async fn get_feed(mut db: Db, _user: User) -> Result<impl Reply, Rejection> {
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
        .fetch_all(&mut *db)
        .await
        .map_err(|_| warp::reject::custom(InternalServerError))?;

        Ok(PostsPage { posts })
    }
}

mod models {
    use askama::Template;

    #[derive(Template)]
    #[template(path = "feed.html")]
    pub struct PostsPage {
        pub posts: Vec<Post>,
    }

    pub struct Post {
        pub username: String,
        pub content: String,
    }
}
