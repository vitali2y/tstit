use serde::{Deserialize, Serialize};
use serde_json::{Value::Object, json};
use std::{
    collections::HashMap,
    convert::Infallible,
    env,
    sync::{
        atomic::{AtomicUsize, Ordering},
        {Arc, Mutex},
    },
};
use warp::{Filter, Rejection, Reply, http::StatusCode, reject, reply};

type Db = Arc<Mutex<HashMap<usize, serde_json::Value>>>;

#[derive(Deserialize, Serialize, Debug)]
struct Response {
    code: usize,
    data: serde_json::Value,
}

impl Response {
    fn new(data: serde_json::Value) -> Self {
        Self { code: 0, data }
    }

    fn with_status(self, status: StatusCode) -> impl Reply {
        reply::with_status(reply::json(&self), status)
    }
}

fn with_db(db: Db) -> impl Filter<Extract = (Db,), Error = Infallible> + Clone {
    warp::any().map(move || db.clone())
}

fn with_auth() -> impl Filter<Extract = (), Error = Rejection> + Clone {
    warp::header::<String>("Authorization")
        .and_then(|token: String| async move {
            let expected_token = env::var("TSTIT_TKN")
                .map_err(|_| "TSTIT_TKN env var is not set!")
                .unwrap();
            if token.is_empty() || token != expected_token {
                Err(reject::custom(AuthError))
            } else {
                Ok(())
            }
        })
        .untuple_one()
}

#[tokio::main]
async fn main() {
    let db: Db = Arc::new(Mutex::new(HashMap::new()));

    let customer_routes = warp::path("v1").and(warp::path("customer")).and(
        warp::post()
            .and(with_auth())
            .and(warp::body::json())
            .and(with_db(db.clone()))
            .and_then(create_customer)
            .or(warp::patch()
                .and(with_auth())
                .and(warp::path::param())
                .and(warp::body::json())
                .and(with_db(db.clone()))
                .and_then(patch_customer))
            .or(warp::put()
                .and(with_auth())
                .and(warp::path::param())
                .and(warp::body::json())
                .and(with_db(db.clone()))
                .and_then(update_customer))
            .or(warp::delete()
                .and(with_auth())
                .and(warp::path::param())
                .and(with_db(db.clone()))
                .and_then(delete_customer))
            .or(with_auth()
                .and(warp::path::param())
                .and(with_db(db.clone()))
                .and_then(get_customer_by_id))
            .or(with_auth()
                .and(with_db(db.clone()))
                .and_then(get_all_customers)),
    );

    let routes = customer_routes.recover(handle_rejection);

    let url = env::var("TSTIT_URL")
        .map_err(|_| "TSTIT_URL env var is not set!")
        .unwrap();
    println!("fake_server is running @ {}", url);
    let addr = url.split("//").last().unwrap_or(&url);
    let socket_addr: std::net::SocketAddr = addr.parse().expect("failed to parse address");

    warp::serve(routes).run(socket_addr).await;
}

impl Reply for Response {
    fn into_response(self) -> warp::reply::Response {
        warp::reply::json(&self).into_response()
    }
}

async fn create_customer(customer: serde_json::Value, db: Db) -> Result<impl Reply, Rejection> {
    println!("create_customer: {customer:?}");
    let mut db_lock = db.lock().unwrap();
    let id = generate_id();
    db_lock.insert(id, customer);
    Ok(Response::new(json!(id)).with_status(StatusCode::CREATED))
}

async fn get_customer_by_id(id: usize, db: Db) -> Result<impl Reply, Rejection> {
    println!("get_customer_by_id: {id}");
    let db_lock = db.lock().unwrap();
    match db_lock.get(&id) {
        Some(customer) => Ok(Response::new(customer.clone())),
        None => Err(reject::not_found()),
    }
}

async fn get_all_customers(db: Db) -> Result<impl Reply, Rejection> {
    println!("get_all_customers");
    let db_lock = db.lock().unwrap();
    let customers: Vec<_> = db_lock.values().cloned().collect();
    Ok(Response::new(json!(customers)))
}

async fn update_customer(
    id: usize,
    customer: serde_json::Value,
    db: Db,
) -> Result<impl Reply, Rejection> {
    println!("update_customer: {id}");
    let mut db_lock = db.lock().unwrap();
    if db_lock.contains_key(&id) {
        db_lock.insert(id, customer);
        Ok(Response::new(json!(id)))
    } else {
        Err(reject::not_found())
    }
}

async fn patch_customer(
    id: usize,
    patch: serde_json::Value,
    db: Db,
) -> Result<impl Reply, Rejection> {
    println!("patch_customer: {id}, patch: {patch:?}");
    let mut db_lock = db.lock().unwrap();
    match db_lock.get_mut(&id) {
        Some(customer) => {
            if let (Object(patch_obj), Object(customer_obj)) = (patch, customer) {
                for (key, value) in patch_obj {
                    customer_obj.insert(key, value);
                }
            }
            Ok(Response::new(json!(id)))
        }
        None => Err(reject::not_found()),
    }
}

async fn delete_customer(id: usize, db: Db) -> Result<impl Reply, Rejection> {
    println!("delete_customer: {id}");
    let mut db_lock = db.lock().unwrap();
    if db_lock.contains_key(&id) {
        db_lock.remove(&id);
        Ok(Response::new(json!(id)))
    } else {
        Err(reject::not_found())
    }
}

fn generate_id() -> usize {
    static COUNTER_ID: AtomicUsize = AtomicUsize::new(1);
    COUNTER_ID.fetch_add(1, Ordering::Relaxed)
}

#[derive(Debug)]
struct AuthError;
impl reject::Reject for AuthError {}

async fn handle_rejection(err: Rejection) -> Result<impl Reply, Infallible> {
    eprintln!("handle_rejection: {:?}", err);
    let (code, message) = if err.is_not_found() {
        (StatusCode::NOT_FOUND, "NOT_FOUND")
    } else if err.find::<AuthError>().is_some() {
        (StatusCode::UNAUTHORIZED, "UNAUTHORIZED")
    } else if err
        .find::<warp::filters::body::BodyDeserializeError>()
        .is_some()
    {
        (StatusCode::BAD_REQUEST, "BAD_REQUEST")
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_SERVER_ERROR")
    };

    Ok(Response {
        code: code.as_u16() as usize,
        data: json!(message),
    }
    .with_status(code))
}
