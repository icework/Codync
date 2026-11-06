//! Real host/API/ACP boundary with a scripted agent; no live Decisions or model inference.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod common;
use common::host::Host;
use serde_json::{Value, json};
use std::{path::Path, time::Duration};

fn calls(cwd: &Path) -> Vec<Value> {
    std::fs::read_to_string(cwd.join("calls.jsonl"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[tokio::test]
async fn routing_preserves_defaults_reuses_sessions_and_waits_for_acknowledgement() {
    let empty = std::ffi::OsStr::new("");
    let mut host = Host::start_with(&[("OPENAI_API_KEY", empty), ("CODYNC_DECISIONS_API_KEY", empty)]).await;
    let cwd = host.home.join("work");
    std::fs::create_dir(&cwd).unwrap();
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/routing_agent.py");
    let bot = host
        .call(
            "createBot",
            json!({"name":"Routing", "backend":"codex", "smartRouting":true,
        "command":common::python_agent(&script), "cwd":cwd, "model":"gpt-6.1-sol",
        "modelOptions":{"reasoning_effort":"medium"}, "fastMode":true,
        "connectors":[], "skills":[], "notify":false}),
        )
        .await["bot"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    host.reply(&bot, "one").await;
    let log = calls(&cwd);
    let prompt = log.iter().find(|v| v["method"] == "session/prompt").unwrap();
    assert_eq!(prompt["model"], "gpt-6.1-sol");
    assert_eq!(prompt["fast"], "off", "missing credentials cannot spend Fast quota");
    let sid = prompt["sid"].as_str().unwrap().to_owned();
    let history = host.call("history", json!({"botId":bot})).await.to_string();
    assert!(history.contains("missing_api_key"), "{history}");

    // Fixture a previously chosen route, then restart the actual host. It must load that
    // route before the next prompt, despite the saved manual Sol/medium/Fast defaults.
    host.restart(|home| {
        let db = rusqlite::Connection::open(home.join("codync.db")).unwrap();
        db.execute(
            "UPDATE kv SET v=?2 WHERE k=?1",
            rusqlite::params![
                format!("routing.{bot}.{sid}"),
                json!({"profile":"terra-high","fast":true,"outcome":"selected"}).to_string()
            ],
        )
        .unwrap();
    })
    .await;
    host.reply(&bot, "two").await;
    host.reply(&bot, "three").await;
    let log = calls(&cwd);
    let prompts: Vec<_> = log.iter().filter(|v| v["method"] == "session/prompt").collect();
    assert_eq!(prompts.len(), 3);
    for prompt in &prompts[1..] {
        assert_eq!(prompt["model"], "gpt-6.1-terra");
        assert_eq!(prompt["effort"], "high");
        assert_eq!(prompt["fast"], "off", "custom account's quota is unknown");
        assert_eq!(prompt["sid"], sid);
    }
    assert_eq!(log.iter().filter(|v| v["method"] == "session/new").count(), 1);
    let snapshot = host.snapshot().await;
    let saved = common::bot(&snapshot, &bot);
    assert_eq!(saved["model"], "gpt-6.1-sol");
    assert_eq!(saved["modelOptions"]["reasoning_effort"], "medium");
    assert_eq!(saved["fastMode"], true);
    assert_eq!(saved["smartRouting"], true);

    // Recover a config-save crash: the switch is on but the prior session has no route.
    host.restart(|home| {
        let db = rusqlite::Connection::open(home.join("codync.db")).unwrap();
        db.execute("DELETE FROM kv WHERE k=?1", [format!("routing.{bot}.{sid}")]).unwrap();
    })
    .await;
    host.reply(&bot, "migrate").await;
    let log = calls(&cwd);
    let prompt = log.iter().rev().find(|v| v["method"] == "session/prompt").unwrap();
    assert_eq!(prompt["model"], "gpt-6.1-sol", "the unavailable classifier explicitly falls back");
    assert_eq!(prompt["fast"], "off", "a missing route never enables manual Fast");
    host.restart(|home| {
        let db = rusqlite::Connection::open(home.join("codync.db")).unwrap();
        db.execute(
            "UPDATE kv SET v=?2 WHERE k=?1",
            rusqlite::params![
                format!("routing.{bot}.{sid}"),
                json!({"profile":"terra-high","fast":false,"outcome":"selected"}).to_string()
            ],
        )
        .unwrap();
    })
    .await;
    std::fs::write(cwd.join("ignore-effort"), "").unwrap();
    host.restart(|_| {}).await;
    host.call("send", json!({"botId":bot,"text":"must not run"})).await;
    let deadline = std::time::Instant::now() + Duration::from_secs(15);
    loop {
        if common::bot(&host.snapshot().await, &bot)["status"] == "error" {
            break;
        }
        assert!(std::time::Instant::now() < deadline, "unacknowledged route did not fail");
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert_eq!(calls(&cwd).iter().filter(|v| v["method"] == "session/prompt").count(), 4);
    std::fs::remove_file(cwd.join("ignore-effort")).unwrap();
    std::fs::write(cwd.join("empty-load"), "").unwrap();
    host.restart(|_| {}).await;
    host.call("send", json!({"botId":bot,"text":"empty acknowledgement must not run"})).await;
    let deadline = std::time::Instant::now() + Duration::from_secs(15);
    loop {
        if common::bot(&host.snapshot().await, &bot)["status"] == "error" {
            break;
        }
        assert!(std::time::Instant::now() < deadline);
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert_eq!(calls(&cwd).iter().filter(|v| v["method"] == "session/prompt").count(), 4);
}

#[tokio::test]
async fn routing_switch_is_codex_only_and_older_client_patches_preserve_it() {
    let host = Host::start().await;
    let bot = host
        .call(
            "createBot",
            json!({"name":"Settings", "backend":"codex", "smartRouting":true,
        "connectors":[], "skills":[]}),
        )
        .await["bot"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let updated = host.call("updateBot", json!({"id":bot, "name":"Renamed"})).await;
    assert_eq!(updated["bot"]["smartRouting"], true);
    let updated = host.call("updateBot", json!({"id":bot, "backend":"claude"})).await;
    assert_eq!(updated["bot"]["smartRouting"], false);
    host.refused(
        "createBot",
        json!({"name":"Bad", "backend":"claude", "smartRouting":true,
        "connectors":[], "skills":[]}),
    )
    .await;
}

#[tokio::test]
async fn small_conversations_roll_over_only_after_the_idle_boundary() {
    let mut host = Host::start().await;
    let cwd = host.home.join("idle");
    std::fs::create_dir(&cwd).unwrap();
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/routing_agent.py");
    let bot = host
        .call(
            "createBot",
            json!({"name":"Idle routing","backend":"codex","smartRouting":true,
        "command":common::python_agent(&script),"cwd":cwd,"connectors":[],"skills":[],"notify":false}),
        )
        .await["bot"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    host.reply(&bot, "one").await;
    let original =
        calls(&cwd).iter().find(|v| v["method"] == "session/prompt").unwrap()["sid"].as_str().unwrap().to_owned();
    for (minutes, expect_new) in [(29, false), (31, true)] {
        host.restart(|home| {
            let db = rusqlite::Connection::open(home.join("codync.db")).unwrap();
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis();
            db.execute(
                "INSERT OR REPLACE INTO kv(k,v) VALUES(?1,?2)",
                rusqlite::params![
                    format!("compaction.{bot}.{original}"),
                    json!({"used":1,"size":100_000,"lastTurn":now - minutes * 60_000,"compactions":0}).to_string()
                ],
            )
            .unwrap();
        })
        .await;
        host.reply(&bot, &format!("after {minutes} minutes")).await;
        let log = calls(&cwd);
        let prompt = log.iter().rev().find(|v| v["method"] == "session/prompt").unwrap();
        assert_eq!(prompt["sid"] != original, expect_new, "{log:?}");
    }
    let history = host.call("history", json!({"botId":bot})).await;
    assert!(history.to_string().contains("handoff"));
}
