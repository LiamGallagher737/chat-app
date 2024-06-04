pub mod filters {
    use super::handlers;
    use super::templates;
    use crate::database::{with_db, DbPool};
    use crate::form_body;
    use warp::Filter;

    pub fn routes(
        pool: DbPool,
    ) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
        warp::path("signup").and(signup_page().or(users_create(pool)))
    }

    /// GET /signup
    pub fn signup_page(
    ) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
        warp::get().map(|| templates::SignupPage::default())
    }

    /// POST /users with form body
    pub fn users_create(
        pool: DbPool,
    ) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
        warp::post()
            .and(form_body())
            .and(with_db(pool))
            .and_then(handlers::create_user)
            .recover(handlers::rejection_handler)
    }
}

mod handlers {
    use super::models::*;
    use super::templates::SignupPage;
    use crate::{database::Db, InternalServerError};
    use argon2::Argon2;
    use password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
    use warp::{http::Uri, reject::Rejection, reply::Reply};

    pub async fn create_user(user: NewUser, mut db: Db) -> Result<impl Reply, Rejection> {
        let already_exists = sqlx::query!("SELECT * FROM users WHERE username = ?", user.username)
            .fetch_optional(&mut *db)
            .await
            .map_err(|_| warp::reject::custom(InternalServerError))?
            .is_some();

        if already_exists {
            return Err(warp::reject::custom(Error::UserAlreadyExists));
        }

        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let hash = argon2
            .hash_password(user.password.as_bytes(), &salt)
            .map_err(|_| warp::reject::custom(InternalServerError))?
            .to_string();

        sqlx::query!(
            "INSERT INTO users (username, password_hash) VALUES ( ?, ? )",
            user.username,
            hash,
        )
        .execute(&mut *db)
        .await
        .map_err(|_| warp::reject::custom(InternalServerError))?;

        Ok(warp::redirect::see_other(Uri::from_static("/login")))
    }

    pub async fn rejection_handler(err: Rejection) -> Result<impl Reply, Rejection> {
        if let Some(Error::UserAlreadyExists) = err.find() {
            return Ok(SignupPage {
                error: Some("A user already exists with that username"),
            });
        }
        Err(err)
    }
}

mod templates {
    use askama::Template;

    #[derive(Template, Default)]
    #[template(path = "signup.html")]
    pub struct SignupPage {
        pub error: Option<&'static str>,
    }
}

mod models {
    use serde::Deserialize;
    use warp::reject::Reject;

    #[derive(Debug, Deserialize)]
    pub struct NewUser {
        pub username: String,
        pub password: String,
    }

    #[derive(Debug)]
    pub enum Error {
        UserAlreadyExists,
    }
    impl Reject for Error {}
}
