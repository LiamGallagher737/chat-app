use serde::de::DeserializeOwned;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc, RwLock};
use warp::{filters::ws::Message, reject::Reject, Filter};

mod database;
mod feed;
mod live;
mod sessions;
mod users;

// The type for storing the currently conencted users and their sender channels
type Users = Arc<RwLock<HashMap<usize, mpsc::UnboundedSender<Message>>>>;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let db_conenction_pool = database::DbPool::connect("sqlite://db.sqlite")
        .await
        .unwrap();
    sqlx::migrate!().run(&db_conenction_pool).await.unwrap();

    let jwt_key = sessions::Key::generate();

    let users = Users::default();
    let users = warp::any().map(move || users.clone());

    // The path of the websocket
    let chat = warp::path("chat")
        .and(warp::ws())
        .and(users)
        .map(|ws: warp::ws::Ws, users| {
            ws.on_upgrade(move |socket| live::user_connected(socket, users))
        });

    let routes = users::filters::routes(db_conenction_pool.clone(), jwt_key.clone());

    warp::serve(routes).run(([127, 0, 0, 1], 43561)).await;
}

fn form_body<T: DeserializeOwned + Send + Sync>(
) -> impl Filter<Extract = (T,), Error = warp::Rejection> + Clone {
    warp::body::content_length_limit(1024).and(warp::body::form())
}

#[derive(Debug)]
pub struct InternalServerError;
impl Reject for InternalServerError {}
