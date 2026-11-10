use super::*;

#[test]
fn requests_stay_under_their_upstream() {
    assert_eq!(split("/t/abc/v1/models"), Some(("abc", "/v1/models")));
    assert_eq!(split("/t/abc"), Some(("abc", "")));
    assert_eq!(split("/t//x"), None);
    assert_eq!(split("/x/abc"), None);
    let mut record = Record::header("https://api.example.com/v1", "x-api-key", "k".into());
    let url = |r: &Record, rest, q| target(r, rest, q).unwrap().to_string();
    assert_eq!(url(&record, "/models", None), "https://api.example.com/v1/models");
    assert_eq!(url(&record, "", Some("a=1")), "https://api.example.com/v1?a=1");
    assert_eq!(url(&record, "//evil.example/x", None), "https://api.example.com/v1//evil.example/x");
    assert!(url(&record, "/../../x", None).starts_with("https://api.example.com/"));
    assert!(url(&record, "@evil.example/x", None).starts_with("https://api.example.com/"));
    record.strip_query = vec!["key".into()];
    assert_eq!(url(&record, "/m", Some("key=codync-keeper&alt=sse")), "https://api.example.com/v1/m?alt=sse");
    assert_eq!(url(&record, "/m", Some("key=x")), "https://api.example.com/v1/m");
}

async fn echo_upstream() -> (String, Arc<std::sync::Mutex<Vec<String>>>) {
    let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
    let recorded = seen.clone();
    let app = axum::Router::new().fallback(move |request: Request| {
        let recorded = recorded.clone();
        async move {
            recorded.lock().unwrap().push(request.headers()["authorization"].to_str().unwrap().to_owned());
            StatusCode::OK
        }
    });
    let socket = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", socket.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(socket, app).await.unwrap() });
    (url, seen)
}

async fn suspended_request(
    keeper: Arc<Keeper>,
    url: &str,
) -> (tokio::task::JoinHandle<Result<Response, StatusCode>>, tokio::sync::oneshot::Sender<()>) {
    let (release, wait) = tokio::sync::oneshot::channel();
    let started = Arc::new(tokio::sync::Notify::new());
    let polled = started.clone();
    let body = Body::from_stream(futures::stream::once(async move {
        polled.notify_one();
        wait.await.unwrap();
        Ok::<_, std::io::Error>(axum::body::Bytes::new())
    }));
    let request = Request::builder().method("POST").uri(format!("{url}/x")).body(body).unwrap();
    let request = tokio::spawn(async move { forward(&keeper, request).await });
    started.notified().await;
    (request, release)
}

#[tokio::test]
async fn a_refresh_replacement_never_sends_new_credentials_to_the_old_upstream() {
    let k = super::super::tests::test_keeper();
    let (old_url, old_seen) = echo_upstream().await;
    let (new_url, new_seen) = echo_upstream().await;
    let mut old = Record::bearer(&old_url, "old");
    old.oauth = Some(record::OAuth {
        token_url: format!("{old_url}/token"),
        client_id: "test".into(),
        client_secret: None,
        refresh_token: Some("old-refresh".into()),
        expires_at: 1,
        header: "authorization".into(),
        resource: None,
    });
    k.keeper.put_record("test", &old).unwrap();
    let grant = k.keeper.grant("test", None).unwrap();
    let (request, release) = suspended_request(k.keeper.clone(), &grant.url).await;
    k.keeper.put_record("test", &Record::bearer(&new_url, "new")).unwrap();
    release.send(()).unwrap();
    assert_eq!(request.await.unwrap().unwrap().status(), StatusCode::OK);
    assert!(old_seen.lock().unwrap().is_empty(), "the old upstream must never receive the new credential");
    assert_eq!(*new_seen.lock().unwrap(), vec!["Bearer new"]);
}

#[tokio::test]
async fn a_request_revoked_while_its_body_waits_never_reaches_the_upstream() {
    let k = super::super::tests::test_keeper();
    let (url, seen) = echo_upstream().await;
    k.keeper.put_record("test", &Record::bearer(&url, "key")).unwrap();
    let grant = k.keeper.grant("test", None).unwrap();
    let (request, release) = suspended_request(k.keeper.clone(), &grant.url).await;
    k.keeper.revoke(&grant.token);
    release.send(()).unwrap();
    assert_eq!(request.await.unwrap().unwrap_err(), StatusCode::UNAUTHORIZED);
    assert!(seen.lock().unwrap().is_empty());
}

#[tokio::test]
async fn a_late_refresh_preserves_a_replacement_and_cannot_resurrect_a_deleted_record() {
    let k = super::super::tests::test_keeper();
    let old = Record::bearer("https://old.example", "old");
    let fresh = Record::bearer("https://old.example", "refreshed-old");
    let replacement = Record::bearer("https://new.example", "new");
    k.keeper.put_record("test", &replacement).unwrap();
    let actual = save(&k.keeper, "test", old.clone(), fresh.clone()).await.unwrap();
    assert!(actual == replacement);
    assert!(k.keeper.record("test").unwrap().unwrap() == replacement);
    k.keeper.secrets.delete("test").unwrap();
    assert!(save(&k.keeper, "test", old, fresh).await.is_err());
    assert!(k.keeper.record("test").unwrap().is_none());
}
