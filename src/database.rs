use crate::InternalServerError;
use sqlx::{pool::PoolConnection, Sqlite, SqlitePool};
use warp::Filter;

pub type Db = PoolConnection<Sqlite>;
pub type DbPool = SqlitePool;

pub fn with_db(db_pool: DbPool) -> impl Filter<Extract = (Db,), Error = warp::Rejection> + Clone {
    warp::any().and_then(move || {
        let pool = db_pool.clone();
        async move {
            pool.acquire().await.map_err(|e| {
                eprintln!("Failed to acquire a connection: {}", e);
                warp::reject::custom(InternalServerError)
            })
        }
    })
}
