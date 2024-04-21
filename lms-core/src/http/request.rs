use http_body_util::BodyExt;
use hyper::body::Incoming;

pub struct Request {
    pub method: hyper::Method,
    pub url: hyper::Uri,
    pub headers: hyper::HeaderMap,
    pub body: bytes::Bytes,
}

impl Request {
    pub async fn from_hyper(req: hyper::Request<Incoming>) -> anyhow::Result<Self> {
        let (part, body) = req.into_parts();
        let body = body.collect().await?.to_bytes();

        Ok(Self {
            method: part.method,
            url: part.uri,
            headers: part.headers,
            body,
        })
    }
}
