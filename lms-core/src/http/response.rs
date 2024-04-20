use anyhow::{Context, Result};
use bytes::Bytes;
use derive_setters::Setters;
use http_body_util::Full;
use serde::de;
use std::str::FromStr;

#[derive(Clone, Debug, Default, Setters)]
pub struct Response<Body: Default + Clone> {
    pub status: reqwest::StatusCode,
    pub headers: reqwest::header::HeaderMap,
    pub body: Body,
}

impl Response<Bytes> {
    pub async fn from_reqwest(resp: reqwest::Response) -> Result<Self> {
        let status = resp.status();
        let headers = resp.headers().to_owned();
        let body = resp.bytes().await?;
        Ok(Response {
            status,
            headers,
            body,
        })
    }
    pub fn empty() -> Self {
        Response {
            status: reqwest::StatusCode::OK,
            headers: reqwest::header::HeaderMap::default(),
            body: Bytes::new(),
        }
    }

    pub fn to_json<T: de::DeserializeOwned + Clone + Default>(self) -> Result<Response<T>> {
        let mut resp = Response::default();
        let body = serde_json::from_slice::<T>(&self.body)?;
        resp.body = body;
        resp.status = self.status;
        resp.headers = self.headers;
        Ok(resp)
    }

    pub fn to_resp_string(self) -> Result<Response<String>> {
        Ok(Response::<String> {
            body: String::from_utf8(self.body.to_vec())?,
            status: self.status,
            headers: self.headers,
        })
    }
    pub fn into_hyper(self) -> Result<hyper::Response<Full<Bytes>>> {
        let mut builder =
            hyper::Response::builder().status(hyper::StatusCode::from_u16(self.status.as_u16())?);
        for (key, value) in self.headers {
            builder = builder.header(
                hyper::header::HeaderName::from_str(
                    key.context("Invalid header key")?
                        .as_str()
                        .to_string()
                        .as_str(),
                )?,
                hyper::header::HeaderValue::from_str(value.to_str()?)?,
            );
        }
        Ok(builder.body(Full::new(self.body))?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;
    use reqwest::StatusCode;

    #[tokio::test]
    async fn test_response_empty() {
        let response: Response<Bytes> = Response::empty();
        assert_eq!(response.status, StatusCode::OK);
        assert!(response.headers.is_empty());
        assert!(response.body.is_empty());
    }
    #[tokio::test]
    async fn test_to_json() {
        let response = Response {
            status: StatusCode::OK,
            headers: reqwest::header::HeaderMap::new(),
            body: Bytes::from(r#"{"name":"test"}"#),
        };
        let json_response: Result<Response<serde_json::Value>> = response.to_json();
        assert!(json_response.is_ok());
        assert_eq!(json_response.unwrap().body["name"], "test");
    }

    #[tokio::test]
    async fn test_to_resp_string() {
        let response = Response {
            status: StatusCode::OK,
            headers: reqwest::header::HeaderMap::new(),
            body: Bytes::from("hello world"),
        };
        let string_response: Result<Response<String>> = response.to_resp_string();
        assert!(string_response.is_ok());
        assert_eq!(string_response.unwrap().body, "hello world");
    }
    #[tokio::test]
    async fn test_into_hyper() -> Result<()> {
        let response = Response {
            status: StatusCode::OK,
            headers: reqwest::header::HeaderMap::new(),
            body: Bytes::from("data"),
        };
        let hyper_response: Result<hyper::Response<Full<Bytes>>> = response.into_hyper();
        assert!(hyper_response.is_ok());
        let bytes = hyper_response
            .unwrap()
            .into_body()
            .frame()
            .await
            .context("unable to extract frame")??
            .into_data()
            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
        assert_eq!(bytes, &Bytes::from("data"));
        Ok(())
    }
}
