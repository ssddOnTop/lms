use hyper::body::Bytes;
use lms_core::http::response::Response;
use lms_core::HttpIO;
use reqwest::{Client, Request};

#[derive(Default, Clone)]
pub struct NativeHttp {
    client: Client,
}

#[async_trait::async_trait]
impl HttpIO for NativeHttp {
    async fn execute(&self, request: Request) -> anyhow::Result<Response<Bytes>> {
        log::info!(
            "{} {} {:?}",
            request.method(),
            request.url(),
            request.version()
        );
        log::debug!("request: {:?}", request);
        let response = self.client.execute(request).await?;
        log::debug!("response: {:?}", response);

        Ok(Response::from_reqwest(response).await?)
    }
}

#[cfg(test)]
mod tests {
    use reqwest::Method;
    use tokio;

    use super::*;

    fn start_mock_server() -> httpmock::MockServer {
        httpmock::MockServer::start()
    }

    #[tokio::test]
    async fn test_native_http_get_request() {
        let server = start_mock_server();

        let header_serv = server.mock(|when, then| {
            when.method(httpmock::Method::GET).path("/test");
            then.status(200).body("Alo");
        });

        let native_http = NativeHttp::default();
        let port = server.port();
        // Build a GET request to the mock server
        let request_url = format!("http://localhost:{}/test", port);
        let request = Request::new(Method::GET, request_url.parse().unwrap());

        // Execute the request
        let result = native_http.execute(request).await;

        // Assert the response is as expected
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.status, reqwest::StatusCode::OK);
        assert_eq!(response.body, Bytes::from("Alo"));

        header_serv.assert();
    }
}
