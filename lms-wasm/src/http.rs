use crate::to_anyhow;
use anyhow::{anyhow, Context};
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::http::{HeaderName, HeaderValue};
use lms_core::http::request::Request;
use lms_core::http::response::Response;
use lms_core::HttpIO;
use reqwest::Client;
use std::str::FromStr;

#[derive(Clone)]
pub struct WasmHttp {
    client: Client,
}

impl WasmHttp {
    pub fn init() -> Self {
        let client = Client::new();
        Self { client }
    }
}

#[async_trait::async_trait]
impl HttpIO for WasmHttp {
    // HttpClientOptions are ignored in Cloudflare
    // This is because there is little control over the underlying HTTP client
    async fn execute(&self, request: reqwest::Request) -> anyhow::Result<Response<Bytes>> {
        let client = self.client.clone();
        let method = request.method().clone();
        let url = request.url().clone();
        // TODO: remove spawn local
        let res = async_std::task::spawn_local(async move {
            let response = client
                .execute(request)
                .await?
                .error_for_status()
                .map_err(|err| err.without_url())?;
            Response::from_reqwest(response).await
        })
        .await?;
        log::info!("{} {} {}", method, url, res.status.as_u16());
        Ok(res)
    }
}

pub async fn to_response(
    response: hyper::Response<Full<Bytes>>,
) -> anyhow::Result<worker::Response> {
    let status = response.status().as_u16();
    let headers = response.headers().clone();

    let bytes = response
        .into_body()
        .frame()
        .await
        .context("unable to extract frame")??
        .into_data()
        .map_err(to_anyhow)?;

    let body = worker::ResponseBody::Body(bytes.to_vec());
    let mut w_response = worker::Response::from_body(body).map_err(to_anyhow)?;
    w_response = w_response.with_status(status);
    let mut_headers = w_response.headers_mut();
    for (name, value) in headers.iter() {
        let value = String::from_utf8(value.as_bytes().to_vec())?;
        mut_headers
            .append(name.as_str(), &value)
            .map_err(to_anyhow)?;
    }

    Ok(w_response)
}

pub async fn to_request(mut req: worker::Request) -> anyhow::Result<Request> {
    let body = req.text().await.map_err(to_anyhow)?;
    let method = req.method();
    let uri = req.url().map_err(to_anyhow)?.as_str().to_string();
    let req_headers = req.headers();

    let mut headers = hyper::HeaderMap::new();
    for (k, v) in req_headers {
        headers.insert(
            HeaderName::from_str(&k)?,
            HeaderValue::from_str(v.as_str())?,
        );
    }

    let req = Request {
        method: to_method(method)?,
        url: hyper::Uri::from_str(&uri)?,
        headers,
        body: bytes::Bytes::from(body),
    };

    Ok(req)
}

pub fn to_method(method: worker::Method) -> anyhow::Result<hyper::Method> {
    let method = &*method.to_string().to_uppercase();
    match method {
        "GET" => Ok(hyper::Method::GET),
        "POST" => Ok(hyper::Method::POST),
        "PUT" => Ok(hyper::Method::PUT),
        "DELETE" => Ok(hyper::Method::DELETE),
        "HEAD" => Ok(hyper::Method::HEAD),
        "OPTIONS" => Ok(hyper::Method::OPTIONS),
        "PATCH" => Ok(hyper::Method::PATCH),
        "CONNECT" => Ok(hyper::Method::CONNECT),
        "TRACE" => Ok(hyper::Method::TRACE),
        method => Err(anyhow!("Unsupported HTTP method: {}", method)),
    }
}
