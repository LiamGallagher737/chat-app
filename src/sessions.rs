use jwt_simple::{algorithms::MACLike, claims::Claims, reexports::coarsetime::Duration};

pub use filters::with_auth;
pub use handlers::not_authenticated_handler;

pub type Key = jwt_simple::algorithms::HS256Key;
pub type User = AdditionalClaimData;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct AdditionalClaimData {
    user_id: i64,
    username: String,
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
    use crate::database::DbPool;

    use super::templates;
    use super::{models::Error, AdditionalClaimData, Key};
    use jwt_simple::algorithms::MACLike;
    use warp::Filter;

    pub fn routes(
        pool: DbPool,
        key: Key,
    ) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
        warp::path("login").and(login_page())
    }

    /// GET /signup
    pub fn login_page(
    ) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
        warp::get().map(|| templates::LoginPage::default())
    }

    pub fn with_auth(
        key: Key,
    ) -> impl Filter<Extract = (AdditionalClaimData,), Error = warp::Rejection> + Clone {
        warp::header::optional("authorization").and_then(move |auth: Option<String>| {
            let key = key.clone();
            async move {
                match auth {
                    Some(token) => match key.verify_token::<AdditionalClaimData>(&token, None) {
                        Ok(claims) => Ok(claims.custom),
                        Err(_) => Err(warp::reject::custom(Error::NotAuthenticated)),
                    },
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
    use super::models::Error;
    use warp::{http::Uri, reject::Rejection, reply::Reply};

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
    #[derive(Debug)]
    pub enum Error {
        NotAuthenticated,
    }
    impl warp::reject::Reject for Error {}
}
