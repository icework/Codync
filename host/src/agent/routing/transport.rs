//! One pooled connection client, a fixed production endpoint, and bounded/sanitized responses.
use super::Outcome;
use serde_json::Value;
use std::{sync::LazyLock, time::Duration};

static CLIENT: LazyLock<Result<reqwest::Client, Outcome>> = LazyLock::new(|| client(super::TIMEOUT));

fn client(timeout: Duration) -> Result<reqwest::Client, Outcome> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(timeout)
        .connect_timeout(Duration::from_secs(3))
        .build()
        .map_err(|_| Outcome::ClientError)
}

pub(super) async fn send(key: &str, body: &Value) -> Result<Value, Outcome> {
    exchange(CLIENT.as_ref().map_err(|e| *e)?, "https://api.openai.com/v1/decisions", key, body).await
}

async fn exchange(client: &reqwest::Client, endpoint: &str, key: &str, body: &Value) -> Result<Value, Outcome> {
    let mut response = client.post(endpoint).bearer_auth(key).json(body).send().await.map_err(|e| network(&e))?;
    if !response.status().is_success() {
        return Err(Outcome::ApiRejected);
    }
    if response.content_length().is_some_and(|n| n > 65536) {
        return Err(Outcome::InvalidResponse);
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e| network(&e))? {
        if bytes.len() + chunk.len() > 65536 {
            return Err(Outcome::InvalidResponse);
        }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes).map_err(|_| Outcome::InvalidResponse)
}

fn network(error: &reqwest::Error) -> Outcome {
    if error.is_timeout() { Outcome::Timeout } else { Outcome::NetworkError }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    async fn server(response: String, delay: Duration) -> (String, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/v1/decisions", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = [0; 4096];
            let _ = socket.read(&mut buf).await;
            tokio::time::sleep(delay).await;
            let _ = socket.write_all(response.as_bytes()).await;
        });
        (url, task)
    }
    #[tokio::test]
    async fn http_failures_are_bounded_and_redacted() {
        let c = client(Duration::from_secs(1)).unwrap();
        for (raw, expected) in [
            ("HTTP/1.1 401 Unauthorized\r\nContent-Length: 6\r\n\r\nsecret".to_owned(), Outcome::ApiRejected),
            (
                "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:1/secret\r\nContent-Length: 0\r\n\r\n".to_owned(),
                Outcome::ApiRejected,
            ),
            ("HTTP/1.1 200 OK\r\nContent-Length: 1\r\n\r\n{".to_owned(), Outcome::InvalidResponse),
            ("HTTP/1.1 200 OK\r\nContent-Length: 65537\r\n\r\n".to_owned(), Outcome::InvalidResponse),
            (
                format!(
                    "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n10001\r\n{}\r\n0\r\n\r\n",
                    "x".repeat(65537)
                ),
                Outcome::InvalidResponse,
            ),
        ] {
            let (url, task) = server(raw, Duration::ZERO).await;
            assert_eq!(exchange(&c, &url, "test-only", &serde_json::json!({})).await.unwrap_err(), expected);
            task.await.unwrap();
        }
        let (url, task) = server(String::new(), Duration::from_secs(1)).await;
        let c = client(Duration::from_millis(30)).unwrap();
        assert_eq!(exchange(&c, &url, "test-only", &Value::Null).await.unwrap_err(), Outcome::Timeout);
        task.abort();
    }
    #[tokio::test]
    async fn a_live_http_response_reaches_the_strict_answer_parser() {
        let body = r#"{"answers":[{"name":"route","type":"choice","choice":"terra-high-standard"}]}"#;
        let (url, task) =
            server(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{body}", body.len()), Duration::ZERO).await;
        let value = exchange(&client(Duration::from_secs(1)).unwrap(), &url, "test-only", &Value::Null).await.unwrap();
        let offered = vec![("terra-high-standard".into(), super::super::Profile::TerraHigh, false)];
        assert!(super::super::answer(&value, &offered).is_ok());
        task.await.unwrap();
    }
}
