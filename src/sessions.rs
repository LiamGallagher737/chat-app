use jwt_simple::{algorithms::MACLike, claims::Claims, reexports::coarsetime::Duration};

pub type Key = jwt_simple::algorithms::HS256Key;

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
    use super::{models::Error, AdditionalClaimData, Key};
    use jwt_simple::algorithms::MACLike;
    use warp::Filter;

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

    pub fn with_key(key: Key) -> impl Filter<Extract = (Key,), Error = std::convert::Infallible> + Clone {
        warp::any().map(move || key.clone())
    }
}

mod handlers {
    use super::models::Error;
    use warp::{http::Uri, reject::Rejection, reply::Reply};

    pub async fn rejection_handler(err: Rejection) -> Result<impl Reply, Rejection> {
        if let Some(Error::NotAuthenticated) = err.find() {
            return Ok(warp::redirect::see_other(Uri::from_static("/login")));
        }
        Err(err)
    }
}

mod models {
    #[derive(Debug)]
    pub enum Error {
        NotAuthenticated,
    }
    impl warp::reject::Reject for Error {}
}
