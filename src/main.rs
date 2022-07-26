use axum::{Router, routing::get, Server};
use tower_http::trace::TraceLayer;
use std::net::SocketAddr;


async fn handler() -> String {
    "hello world".into()
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let router = Router::new()
    .route("/", get(handler))
    .layer(TraceLayer::new_for_http());

    let addr: SocketAddr = ([127,0,0,1], 3000).into();

    tracing::debug!("Listening on port {:?}", addr);
    Server::bind(&addr).serve(router.into_make_service()).await.unwrap()
}
