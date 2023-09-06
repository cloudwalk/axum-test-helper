//! # Axum Test Helper
//! This is a hard copy from TestClient at axum
//!
//! ## Features
//! - `cookies` - Enables support for cookies in the test client.
//! - `withouttrace` - Disables tracing for the test client.
//!
//! ## Example
//! ```rust
//! use axum::Router;
//! use axum::http::StatusCode;
//! use axum::routing::get;
//! use axum_test_helper::TestClient;
//!
//! fn main() {
//!     let async_block = async {
//!         // you can replace this Router with your own app
//!         let app = Router::new().route("/", get(|| async {}));
//!
//!         // initiate the TestClient with the previous declared Router
//!         let client = TestClient::new(app);
//!
//!         let res = client.get("/").send().await;
//!         assert_eq!(res.status(), StatusCode::OK);
//!     };
//!
//!     // Create a runtime for executing the async block. This runtime is local
//!     // to the main function and does not require any global setup.
//!     let runtime = tokio::runtime::Builder::new_current_thread()
//!         .enable_all()
//!         .build()
//!         .unwrap();
//!
//!     // Use the local runtime to block on the async block.
//!     runtime.block_on(async_block);
//! }

use axum::{body::HttpBody, BoxError};
use bytes::Bytes;
use http::{
    header::{HeaderName, HeaderValue},
    Request, StatusCode,
};
use hyper::{Body, Server};
use std::convert::TryFrom;
use std::net::{SocketAddr, TcpListener};
use tower::make::Shared;
use tower_service::Service;

pub struct TestClient {
    client: reqwest::Client,
    addr: SocketAddr,
}

impl TestClient {
    pub fn new<S, ResBody>(svc: S) -> Self
    where
        S: Service<Request<Body>, Response = http::Response<ResBody>> + Clone + Send + 'static,
        ResBody: HttpBody + Send + 'static,
        ResBody::Data: Send,
        ResBody::Error: Into<BoxError>,
        S::Future: Send,
        S::Error: Into<BoxError>,
    {
        let listener = TcpListener::bind("127.0.0.1:0").expect("Could not bind ephemeral socket");
        let addr = listener.local_addr().unwrap();
        #[cfg(feature = "withouttrace")]
        print!("");
        #[cfg(feature = "withtrace")]
        println!("Listening on {}", addr);

        tokio::spawn(async move {
            let server = Server::from_tcp(listener).unwrap().serve(Shared::new(svc));
            server.await.expect("server error");
        });

        #[cfg(feature = "cookies")]
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .cookie_store(true)
            .build()
            .unwrap();

        #[cfg(not(feature = "cookies"))]
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();

        TestClient { client, addr }
    }

    /// returns the base URL (http://ip:port) for this TestClient
    ///
    /// this is useful when trying to check if Location headers in responses
    /// are generated correctly as Location contains an absolute URL
    pub fn base_url(&self) -> String {
        format!("http://{}", self.addr)
    }

    pub fn get(&self, url: &str) -> RequestBuilder {
        RequestBuilder {
            builder: self.client.get(format!("http://{}{}", self.addr, url)),
        }
    }

    pub fn head(&self, url: &str) -> RequestBuilder {
        RequestBuilder {
            builder: self.client.head(format!("http://{}{}", self.addr, url)),
        }
    }

    pub fn post(&self, url: &str) -> RequestBuilder {
        RequestBuilder {
            builder: self.client.post(format!("http://{}{}", self.addr, url)),
        }
    }

    pub fn put(&self, url: &str) -> RequestBuilder {
        RequestBuilder {
            builder: self.client.put(format!("http://{}{}", self.addr, url)),
        }
    }

    pub fn patch(&self, url: &str) -> RequestBuilder {
        RequestBuilder {
            builder: self.client.patch(format!("http://{}{}", self.addr, url)),
        }
    }

    pub fn delete(&self, url: &str) -> RequestBuilder {
        RequestBuilder {
            builder: self.client.delete(format!("http://{}{}", self.addr, url)),
        }
    }
}

pub struct RequestBuilder {
    builder: reqwest::RequestBuilder,
}

impl RequestBuilder {
    pub async fn send(self) -> TestResponse {
        TestResponse {
            response: self.builder.send().await.unwrap(),
        }
    }

    pub fn body(mut self, body: impl Into<reqwest::Body>) -> Self {
        self.builder = self.builder.body(body);
        self
    }

    pub fn form<T: serde::Serialize + ?Sized>(mut self, form: &T) -> Self {
        self.builder = self.builder.form(&form);
        self
    }

    pub fn json<T>(mut self, json: &T) -> Self
    where
        T: serde::Serialize,
    {
        self.builder = self.builder.json(json);
        self
    }

    pub fn header<K, V>(mut self, key: K, value: V) -> Self
    where
        HeaderName: TryFrom<K>,
        <HeaderName as TryFrom<K>>::Error: Into<http::Error>,
        HeaderValue: TryFrom<V>,
        <HeaderValue as TryFrom<V>>::Error: Into<http::Error>,
    {
        self.builder = self.builder.header(key, value);
        self
    }

    pub fn multipart(mut self, form: reqwest::multipart::Form) -> Self {
        self.builder = self.builder.multipart(form);
        self
    }
}

/// A wrapper around [`reqwest::Response`] that provides common methods with internal `unwrap()`s.
///
/// This is conventient for tests where panics are what you want. For access to
/// non-panicking versions or the complete `Response` API use `into_inner()` or
/// `as_ref()`.
pub struct TestResponse {
    response: reqwest::Response,
}

impl TestResponse {
    pub async fn text(self) -> String {
        self.response.text().await.unwrap()
    }

    #[allow(dead_code)]
    pub async fn bytes(self) -> Bytes {
        self.response.bytes().await.unwrap()
    }

    pub async fn json<T>(self) -> T
    where
        T: serde::de::DeserializeOwned,
    {
        self.response.json().await.unwrap()
    }

    pub fn status(&self) -> StatusCode {
        self.response.status()
    }

    pub fn headers(&self) -> &http::HeaderMap {
        self.response.headers()
    }

    pub async fn chunk(&mut self) -> Option<Bytes> {
        self.response.chunk().await.unwrap()
    }

    pub async fn chunk_text(&mut self) -> Option<String> {
        let chunk = self.chunk().await?;
        Some(String::from_utf8(chunk.to_vec()).unwrap())
    }

    /// Get the inner [`reqwest::Response`] for less convenient but more complete access.
    pub fn into_inner(self) -> reqwest::Response {
        self.response
    }
}

impl AsRef<reqwest::Response> for TestResponse {
    fn as_ref(&self) -> &reqwest::Response {
        &self.response
    }
}

#[cfg(test)]
mod tests {
    use axum::response::Html;
    use axum::routing::{get, post};
    use axum::Router;
    use http::StatusCode;
    use serde::{Deserialize, Serialize};
    use axum::{routing::get, routing::post, Router, Json};
    use http::{StatusCode, header::{HeaderName, HeaderValue}};

    #[derive(Deserialize)]
    struct FooForm {
        val: String,
    }

    async fn handle_form(axum::Form(form): axum::Form<FooForm>) -> (StatusCode, Html<String>) {
        (StatusCode::OK, Html(form.val))
    }

    #[tokio::test]
    async fn test_get_request() {
        let app = Router::new().route("/", get(|| async {}));
        let client = super::TestClient::new(app);
        let res = client.get("/").send().await;
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_post_form_request() {
        let app = Router::new().route("/", post(handle_form));
        let client = super::TestClient::new(app);
        let form = [("val", "bar"), ("baz", "quux")];
        let res = client.post("/").form(&form).send().await;
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.text().await, "bar");
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestPayload {
        name: String,
        age: i32,
    }

    #[tokio::test]
    async fn test_post_request_with_json() {
        let app = Router::new().route("/", post(|json_value: Json<serde_json::Value>| async {json_value}));
        let client = super::TestClient::new(app);
        let payload = TestPayload {
            name: "Alice".to_owned(),
            age: 30,
        };
        let res = client.post("/")
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::OK);
        let response_body: TestPayload = serde_json::from_str(&res.text().await).unwrap();
        assert_eq!(response_body, payload);
    }
}
