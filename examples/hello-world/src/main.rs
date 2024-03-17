//! Run with
//!
//! ```not_rust
//! cd examples && cargo run -p example-hello-world
//! ```

use axum::{response::Html, routing::get, Router};
use std::net::SocketAddr;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    // build our application with a route
    let router = router();

    // run it
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("listening on {}", addr);

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, router.into_make_service()).await.unwrap();
}

fn router() -> Router {
    Router::new().route("/", get(handler))
}

async fn handler() -> Html<&'static str> {
    Html("<h1>Hello, World!</h1>")
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum_test_helper::TestClient;

    #[tokio::test]
    async fn test_main_router() {
        let router = router();
        let client = TestClient::new(router);
        let res = client.get("/").send().await;
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.text().await, "<h1>Hello, World!</h1>");
    }
}
