use serde::de::DeserializeOwned;
use warp::{reject::Reject, Filter};

mod database;
mod feed;
mod sessions;
mod users;

#[tokio::main]
async fn main() {
    pretty_env_logger::formatted_builder()
        .parse_filters("warp=info,chat_app=trace")
        .init();

    let db_conenction_pool = database::DbPool::connect("sqlite://db.sqlite")
        .await
        .unwrap();
    sqlx::migrate!().run(&db_conenction_pool).await.unwrap();

    let jwt_key = sessions::Key::generate();

    let routes = users::filters::routes(db_conenction_pool.clone(), jwt_key.clone())
        .or(sessions::filters::routes(
            db_conenction_pool.clone(),
            jwt_key.clone(),
        ))
        .or(feed::filters::routes(
            db_conenction_pool.clone(),
            jwt_key.clone(),
        ))
        .recover(sessions::not_authenticated_handler)
        //.or(warp::any().map(|| warp::reply::with_status("Not found", warp::http::StatusCode::NOT_FOUND)))
        .with(warp::log("warp"));

    warp::serve(routes).run(([127, 0, 0, 1], 43561)).await;
}

fn form_body<T: DeserializeOwned + Send + Sync>(
) -> impl Filter<Extract = (T,), Error = warp::Rejection> + Clone {
    warp::body::content_length_limit(1024).and(warp::body::form())
}

#[derive(Debug)]
pub struct InternalServerError;
impl Reject for InternalServerError {}
