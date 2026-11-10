#![allow(clippy::unwrap_used)]
use super::*;
use crate::remote::identity::Identity;
use crate::store::{Device, DeviceSource, Scope};
use std::sync::Arc;

/// On the Codync cloud, where a computer can be in an account.
fn hub() -> Arc<Hub> {
    let dir = std::env::temp_dir().join(format!("codync-owner-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let store = Store::open(&dir.join("t.db")).unwrap();
    store.kv_set("test_cloud_url", "https://dev-api.codync.dev").unwrap();
    Hub::new(store, "host".into(), Identity::load_or_create(&dir).unwrap(), "t".into(), 0)
}

fn device(hub: &Hub, key: &str, account: DeviceAccount) {
    hub.store
        .put_device(&Device {
            key: key.into(),
            name: "Phone".into(),
            platform: "ios".into(),
            source: DeviceSource::Local,
            grant_id: None,
            scopes: vec![Scope::Control],
            lease_until: None,
            created_at: 1,
            last_seen_at: None,
            account,
        })
        .unwrap();
}

fn of(user: &str) -> DeviceAccount {
    DeviceAccount::Of(user.into())
}

fn keys(hub: &Hub) -> Vec<String> {
    hub.store.devices().unwrap().into_iter().map(|d| d.key).collect()
}

fn account(id: &str) -> Owner {
    Owner { user_id: id.into(), email: Some(format!("{id}@example.com")) }
}

#[tokio::test]
async fn a_pending_phone_join_rechecks_revocation_scope_and_owner() {
    for change in ["revoked", "screen_only", "owner_changed"] {
        let hub = hub();
        remember(&hub, None).unwrap();
        let key = crypto::b64(&[7; 32]);
        device(&hub, &key, DeviceAccount::Unrecorded);
        let joining = tokio::spawn({
            let (hub, key) = (hub.clone(), key.clone());
            async move {
                let certificate = certificate::test_certificate(&key, "user_a");
                join_from_phone(&hub, &key, &certificate, "claim", "nonce").await
            }
        });
        let request = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                if let Some(request) = hub.pairing_requests.json().first() {
                    break request["requestId"].as_str().unwrap().to_owned();
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        match change {
            "revoked" => assert!(hub.revoke_device(&key).unwrap()),
            "screen_only" => {
                let mut phone = hub.store.device(&key).unwrap();
                phone.scopes = vec![Scope::Screen];
                hub.store.put_device(&phone).unwrap();
            }
            _ => remember(&hub, Some(&account("user_b"))).unwrap(),
        }
        pairing_requests::decide(&hub, &request, true, Some(true)).unwrap();
        let result = joining.await.unwrap();
        assert!(result.is_err(), "{change}: a stale approval signed a claim");
        assert_eq!(*hub.cloud.join_pending_until.locked(), 0, "{change}: no relay join was opened");
        assert_eq!(
            hub.store.device(&key).map(|phone| phone.account),
            (change != "revoked").then_some(DeviceAccount::Unrecorded),
            "{change}: a refused join changed the device account"
        );
    }
}

#[test]
fn an_expired_certificate_or_lease_cannot_finish_a_phone_join() {
    let hub = hub();
    let key = crypto::b64(&[7; 32]);
    device(&hub, &key, DeviceAccount::SignedOut);
    let certificate = certificate::test_certificate(&key, "user_a");
    let finish = |now| {
        while_unchanged(&hub, || {
            finish_phone_join(
                &hub,
                &key,
                &certificate,
                "claim",
                "nonce",
                &ClaimDevice { name: "Test computer".into(), device: crate::service::Device::Desktop },
                now,
            )
        })
    };
    assert!(finish(now_ms() + 60_001).is_err(), "a certificate valid when asked has since expired");
    let mut phone = hub.store.device(&key).unwrap();
    phone.source = DeviceSource::Account;
    phone.lease_until = Some(now_ms() - 1);
    hub.store.put_device(&phone).unwrap();
    hub.cloud_synced.store(true, std::sync::atomic::Ordering::Release);
    assert!(finish(now_ms()).is_err(), "an expired account device cannot join");
    assert_eq!(hub.store.device(&key).unwrap().account, DeviceAccount::SignedOut);
    assert_eq!(*hub.cloud.join_pending_until.locked(), 0);
}

#[test]
fn changing_accounts_drops_the_old_accounts_devices() {
    let hub = hub();
    device(&hub, "a", of("user_a"));
    device(&hub, "b", of("user_b"));
    device(&hub, "legacy", DeviceAccount::Unrecorded);
    // An upgraded computer records what it finds and drops nothing.
    remember(&hub, Some(&account("user_a"))).unwrap();
    assert_eq!(keys(&hub), ["a", "b", "legacy"]);
    assert_eq!(owner(&hub.store).unwrap().user_id, "user_a");
    // The same account again (a new email included) changes nothing.
    remember(&hub, Some(&Owner { user_id: "user_a".into(), email: None })).unwrap();
    assert_eq!(keys(&hub), ["a", "b", "legacy"]);
    // Moving to B keeps only B's devices.
    remember(&hub, Some(&account("user_b"))).unwrap();
    assert_eq!(keys(&hub), ["b"]);
    // Leaving the account drops the rest.
    remember(&hub, None).unwrap();
    assert!(keys(&hub).is_empty(), "leaving drops every device");
    assert!(owner(&hub.store).is_none());
}

#[test]
fn joining_a_first_account_keeps_the_phones_it_had() {
    let hub = hub();
    remember(&hub, None).unwrap();
    device(&hub, "legacy", DeviceAccount::Unrecorded);
    device(&hub, "mine", of("user_a"));
    device(&hub, "theirs", of("user_b"));
    remember(&hub, Some(&account("user_a"))).unwrap();
    assert_eq!(keys(&hub), ["legacy", "mine"]);
}

#[test]
fn phones_with_no_account_go_when_the_computer_joins_one() {
    let hub = hub();
    remember(&hub, None).unwrap();
    device(&hub, "signed-out", DeviceAccount::SignedOut);
    device(&hub, "legacy", DeviceAccount::Unrecorded);
    device(&hub, "mine", of("user_a"));
    remember(&hub, None).unwrap();
    assert_eq!(keys(&hub), ["signed-out", "legacy", "mine"], "they stay while it is in no account");
    remember(&hub, Some(&account("user_a"))).unwrap();
    assert_eq!(keys(&hub), ["legacy", "mine"]);
}

#[test]
fn a_first_record_in_an_account_also_removes_phones_with_no_account() {
    let hub = hub();
    device(&hub, "signed-out", DeviceAccount::SignedOut);
    device(&hub, "legacy", DeviceAccount::Unrecorded);
    remember(&hub, Some(&account("user_a"))).unwrap();
    assert_eq!(keys(&hub), ["legacy"]);
}

#[test]
fn a_join_warns_about_the_phones_it_removes() {
    let hub = hub();
    device(&hub, "signed-out", DeviceAccount::SignedOut);
    device(&hub, "legacy", DeviceAccount::Unrecorded);
    device(&hub, "mine", of("user_a"));
    device(&hub, "theirs", of("user_b"));
    let removed = |key| removed_by_joining(&hub, "user_a", key).len();
    assert_eq!(removed(""), 2, "the signed-out phone and the other account's");
    assert_eq!(removed("signed-out"), 1, "not the phone that asks");
}

#[test]
fn phones_get_the_relay_address_only_for_a_computer_in_an_account() {
    let hub = hub();
    device(&hub, "mine", of("user_a"));
    device(&hub, "signed-out", DeviceAccount::SignedOut);
    assert!(can_join(&hub));
    assert_eq!(url_for_phones(&hub, "mine"), None);
    *hub.cloud.join_pending_until.locked() = now_ms() + 60_000;
    assert!(url_for_phones(&hub, "mine").is_some(), "the phone finishing a join goes on through the relay");
    assert_eq!(url_for_phones(&hub, "signed-out"), None, "a signed-out phone couldn't use it");
    remember(&hub, Some(&account("user_a"))).unwrap();
    assert!(!can_join(&hub));
    assert!(url_for_phones(&hub, "mine").is_some());
    remember(&hub, None).unwrap();
    device(&hub, "mine", of("user_a"));
    assert_eq!(url_for_phones(&hub, "mine"), None, "the claim ended with the join, so leaving closes the relay");
    hub.store.kv_set("test_cloud_token", "relay-token").unwrap();
    assert!(url_for_phones(&hub, "local").is_some(), "a private relay is the user's own");
    assert!(!can_join(&hub), "and has no accounts to join");
}

#[test]
fn a_computer_in_no_account_holds_the_relay_only_for_a_pairing_or_a_join() {
    let hub = hub();
    let address = cloud::current(&hub.store).unwrap();
    assert!(!wants_relay(&hub, &address));
    let code = hub.pairing.locked().issue().code;
    assert!(wants_relay(&hub, &address), "a phone may pair through it");
    assert!(hub.pairing.locked().redeem(&code));
    assert!(!wants_relay(&hub, &address));
    *hub.cloud.join_pending_until.locked() = now_ms() + 60_000;
    assert!(wants_relay(&hub, &address), "the cloud says over it that the claim completed");
    remember(&hub, Some(&account("user_a"))).unwrap();
    assert!(wants_relay(&hub, &address));
    remember(&hub, None).unwrap();
    assert!(!wants_relay(&hub, &address), "the claim ended with the join");
}

#[test]
fn with_the_cloud_off_or_a_private_relay_the_computer_is_in_no_account() {
    let hub = hub();
    remember(&hub, Some(&account("user_a"))).unwrap();
    hub.store.kv_set("test_cloud_token", "relay-token").unwrap();
    assert!(owner(&hub.store).is_none(), "a private relay has no accounts");
    hub.store.kv_set("cloud_enabled", "false").unwrap();
    assert!(owner(&hub.store).is_none(), "nor does a computer with its cloud off");
}

#[test]
fn out_of_an_account_only_a_cloud_computer_keeps_approving_by_itself() {
    let hub = hub();
    cloud::set_approval(&hub, Approval::Auto).unwrap();
    out_of_account(&hub).unwrap();
    assert_eq!(cloud::approval(&hub.store), Approval::Code);
    cloud::set_approval(&hub, Approval::Auto).unwrap();
    crate::remote::managed::set_managed(&hub.store).unwrap();
    out_of_account(&hub).unwrap();
    assert_eq!(cloud::approval(&hub.store), Approval::Auto, "before its first claim completes");
}

#[tokio::test]
async fn cloud_claim_metadata_is_prepared_before_entering_the_signing_transaction() {
    let hub = hub();
    hub.store.kv_set("cloud_managed", "1").unwrap();
    hub.store.kv_set("cloud_name", "Fixture cloud").unwrap();
    let prepared = claim_device(&hub.store).await.unwrap();
    assert_eq!(prepared.name, "Fixture cloud");
    assert!(matches!(prepared.device, crate::service::Device::Cloud));
    let key = crypto::b64(&[7; 32]);
    device(&hub, &key, DeviceAccount::SignedOut);
    let certificate = certificate::test_certificate(&key, "user_a");
    let signed =
        while_unchanged(&hub, || finish_phone_join(&hub, &key, &certificate, "claim", "nonce", &prepared, now_ms()))
            .unwrap();
    assert_eq!(signed["name"], "Fixture cloud");
    assert_eq!(signed["device"], "cloud");
}
