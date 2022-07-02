/// this is a hard copy from TestClient at axum
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
        println!("Listening on {}", addr);

        tokio::spawn(async move {
            let server = Server::from_tcp(listener).unwrap().serve(Shared::new(svc));
            server.await.expect("server error");
        });

        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();

        TestClient { client, addr }
    }

    #[allow(dead_code)]
    pub fn get(&self, url: &str) -> RequestBuilder {
        RequestBuilder {
            builder: self.client.get(format!("http://{}{}", self.addr, url)),
        }
    }

    #[allow(dead_code)]
    pub fn head(&self, url: &str) -> RequestBuilder {
        RequestBuilder {
            builder: self.client.head(format!("http://{}{}", self.addr, url)),
        }
    }

    #[allow(dead_code)]
    pub fn post(&self, url: &str) -> RequestBuilder {
        RequestBuilder {
            builder: self.client.post(format!("http://{}{}", self.addr, url)),
        }
    }

    #[allow(dead_code)]
    pub fn put(&self, url: &str) -> RequestBuilder {
        RequestBuilder {
            builder: self.client.put(format!("http://{}{}", self.addr, url)),
        }
    }

    #[allow(dead_code)]
    pub fn patch(&self, url: &str) -> RequestBuilder {
        RequestBuilder {
            builder: self.client.patch(format!("http://{}{}", self.addr, url)),
        }
    }

    #[allow(dead_code)]
    pub fn delete(&self, url: &str) -> RequestBuilder {
        RequestBuilder {
            builder: self.client.delete(format!("http://{}{}", self.addr, url)),
        }
    }
}

#[allow(dead_code)]
pub struct RequestBuilder {
    builder: reqwest::RequestBuilder,
}

impl RequestBuilder {
    #[allow(dead_code)]
    pub async fn send(self) -> TestResponse {
        TestResponse {
            response: self.builder.send().await.unwrap(),
        }
    }

    #[allow(dead_code)]
    pub fn body(mut self, body: impl Into<reqwest::Body>) -> Self {
        self.builder = self.builder.body(body);
        self
    }

    #[allow(dead_code)]
    pub fn json<T>(mut self, json: &T) -> Self
    where
        T: serde::Serialize,
    {
        self.builder = self.builder.json(json);
        self
    }

    #[allow(dead_code)]
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

    #[allow(dead_code)]
    pub fn multipart(mut self, form: reqwest::multipart::Form) -> Self {
        self.builder = self.builder.multipart(form);
        self
    }
}

#[allow(dead_code)]
pub struct TestResponse {
    response: reqwest::Response,
}

impl TestResponse {
    #[allow(dead_code)]
    pub async fn text(self) -> String {
        self.response.text().await.unwrap()
    }

    #[allow(dead_code)]
    pub async fn json<T>(self) -> T
    where
        T: serde::de::DeserializeOwned,
    {
        self.response.json().await.unwrap()
    }

    #[allow(dead_code)]
    pub fn status(&self) -> StatusCode {
        self.response.status()
    }

    #[allow(dead_code)]
    pub fn headers(&self) -> &http::HeaderMap {
        self.response.headers()
    }

    #[allow(dead_code)]
    pub async fn chunk(&mut self) -> Option<Bytes> {
        self.response.chunk().await.unwrap()
    }

    #[allow(dead_code)]
    pub async fn chunk_text(&mut self) -> Option<String> {
        let chunk = self.chunk().await?;
        Some(String::from_utf8(chunk.to_vec()).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use axum::routing::get;
    use axum::Router;
    use http::StatusCode;

    #[tokio::test]
    async fn test_get_request() {
        let app = Router::new().route("/", get(|| async {}));
        let client = super::TestClient::new(app);
        let res = client.get("/").send().await;
        assert_eq!(res.status(), StatusCode::OK);
    }
}
