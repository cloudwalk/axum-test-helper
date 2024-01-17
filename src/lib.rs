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

use bytes::Bytes;
use http::StatusCode;
use std::net::SocketAddr;
use tokio::net::TcpListener;

pub struct TestClient {
    client: reqwest::Client,
    addr: SocketAddr,
}

impl TestClient {
    pub async fn new(svc: axum::Router) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Could not bind ephemeral socket");
        let addr = listener.local_addr().unwrap();
        #[cfg(feature = "withtrace")]
        println!("Listening on {}", addr);

        tokio::spawn(async move {
            let server = axum::serve(listener, svc);
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

    pub fn header(mut self, key: &str, value: &str) -> Self {
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
        StatusCode::from_u16(self.response.status().as_u16()).unwrap()
    }

    // pub fn headers(&self) -> &http::HeaderMap {
    //     self.response.headers()
    // }

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
    use axum::{
        response::{IntoResponse, Response},
        routing::get,
        routing::post,
        Json, Router,
    };
    use http::StatusCode;
    use serde::{Deserialize, Serialize};

    #[derive(Deserialize)]
    struct FooForm {
        val: String,
    }

    async fn handle_form(axum::Form(form): axum::Form<FooForm>) -> Response {
        (StatusCode::OK, Html(form.val)).into_response()
    }

    #[tokio::test]
    async fn test_get_request() {
        let app = Router::new().route("/", get(|| async {}));
        let client = super::TestClient::new(app).await;
        let res = client.get("/").send().await;
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_post_form_request() {
        let app = Router::new().route("/", post(handle_form));
        let client = super::TestClient::new(app).await;
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
        let app = Router::new().route(
            "/",
            post(|json_value: Json<serde_json::Value>| async { json_value }),
        );
        let client = super::TestClient::new(app).await;
        let payload = TestPayload {
            name: "Alice".to_owned(),
            age: 30,
        };
        let res = client
            .post("/")
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await;
        assert_eq!(res.status(), StatusCode::OK);
        let response_body: TestPayload = serde_json::from_str(&res.text().await).unwrap();
        assert_eq!(response_body, payload);
    }
}
