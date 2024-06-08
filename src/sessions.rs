use jwt_simple::{algorithms::MACLike, claims::Claims, reexports::coarsetime::Duration};

pub use filters::with_auth;
pub use handlers::not_authenticated_handler;

pub const SESSION_LENGTH_SECS: usize = 60 * 60 * 24; // 1 day

pub type Key = jwt_simple::algorithms::HS256Key;
pub type User = AdditionalClaimData;

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct AdditionalClaimData {
    pub user_id: i64,
    pub username: String,
}

pub fn generate_token(
    key: Key,
    user_id: i64,
    username: String,
) -> Result<String, jwt_simple::Error> {
    let custom = AdditionalClaimData { user_id, username };
    let claims = Claims::with_custom_claims(custom, Duration::from_hours(2));
    key.authenticate(claims)
}

pub mod filters {
    use crate::database::{with_db, DbPool};
    use crate::form_body;

    use super::{handlers, templates};
    use super::{models::Error, AdditionalClaimData, Key};
    use jwt_simple::algorithms::MACLike;
    use warp::Filter;

    pub fn routes(
        pool: DbPool,
        key: Key,
    ) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
        warp::any()
            .and(login_page())
            .or(session_create(pool.clone(), key.clone()))
    }

    /// GET /login
    pub fn login_page(
    ) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
        warp::path("login")
            .and(warp::get())
            .map(|| templates::LoginPage::default())
    }

    /// POST /login
    pub fn session_create(
        pool: DbPool,
        key: Key,
    ) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
        warp::path("login")
            .and(warp::post())
            .and(form_body())
            .and(with_db(pool))
            .and(with_key(key))
            .and_then(handlers::create_session)
            .recover(handlers::rejection_handler)
    }

    pub fn with_auth(
        key: Key,
    ) -> impl Filter<Extract = (AdditionalClaimData,), Error = warp::Rejection> + Clone {
        warp::any()
            .and(warp::cookie::optional::<String>("jwt"))
            .and_then(move |jwt: Option<String>| {
                let key = key.clone();
                async move {
                    match jwt {
                        Some(jwt) => Ok(key
                            .verify_token::<AdditionalClaimData>(&jwt, None)
                            .map_err(|_| Error::NotAuthenticated)?
                            .custom),
                        None => Err(warp::reject::custom(Error::NotAuthenticated)),
                    }
                }
            })
    }

    pub fn with_key(
        key: Key,
    ) -> impl Filter<Extract = (Key,), Error = std::convert::Infallible> + Clone {
        warp::any().map(move || key.clone())
    }
}

mod handlers {
    use super::{models::*, templates::*, Key, SESSION_LENGTH_SECS};
    use crate::{database::Db, InternalServerError};
    use argon2::Argon2;
    use password_hash::{PasswordHash, PasswordVerifier};
    use warp::{
        http::Uri,
        reject::Rejection,
        reply::{with_header, Reply},
    };

    pub async fn create_session(
        credentials: UserCredentials,
        mut db: Db,
        key: Key,
    ) -> Result<impl Reply, Rejection> {
        let user = sqlx::query_as!(
            UserRow,
            "SELECT id, username, password_hash FROM users WHERE username = ?",
            credentials.username,
        )
        .fetch_optional(&mut *db)
        .await
        .map_err(|_| warp::reject::custom(InternalServerError))?;

        let Some(user) = user else {
            return Err(Error::InvalidCredentials.into());
        };

        let argon2 = Argon2::default();
        let correct_password = argon2
            .verify_password(
                credentials.password.as_bytes(),
                &PasswordHash::new(&user.password_hash).unwrap(),
            )
            .is_ok();
        if !correct_password {
            return Err(Error::InvalidCredentials.into());
        };

        let token = crate::sessions::generate_token(key, user.id, user.username)
            .map_err(|_| warp::reject::custom(InternalServerError))?;

        Ok(with_header(
            warp::redirect::see_other(Uri::from_static("/")),
            "set-cookie",
            format!("jwt={token}; max-age={SESSION_LENGTH_SECS}; secure; httponly;"),
        ))
    }

    pub async fn rejection_handler(err: Rejection) -> Result<impl Reply, Rejection> {
        if let Some(Error::InvalidCredentials) = err.find() {
            return Ok(LoginPage {
                error: Some("Invalid credentials"),
            });
        }
        Err(err)
    }

    pub async fn not_authenticated_handler(err: Rejection) -> Result<impl Reply, Rejection> {
        if let Some(Error::NotAuthenticated) = err.find() {
            return Ok(warp::redirect::see_other(Uri::from_static("/login")));
        }
        Err(err)
    }
}

mod templates {
    use askama::Template;

    #[derive(Template, Default)]
    #[template(path = "login.html")]
    pub struct LoginPage {
        pub error: Option<&'static str>,
    }
}

mod models {
    #[derive(Debug, serde::Deserialize)]
    pub struct UserCredentials {
        pub username: String,
        pub password: String,
    }

    pub struct UserRow {
        pub id: i64,
        pub username: String,
        pub password_hash: String,
    }

    #[derive(Debug)]
    pub enum Error {
        InvalidCredentials,
        NotAuthenticated,
    }
    impl warp::reject::Reject for Error {}
}
