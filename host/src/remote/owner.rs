//! The account this computer belongs to, kept on the computer: pairing checks a phone's account against it
//! even with the cloud unreachable, and a change of account drops the old account's devices
//! (docs/architecture/connections.md: "the host changed its account").

use super::cloud::{self, Approval, Owner, RelayKind};
use super::pairing_requests::{self, Answer, Question};
use super::{certificate, crypto};
use crate::LockExt;
use crate::chat::team::assistant;
use crate::hub::Hub;
use crate::store::{DeviceAccount, DeviceSource, Scope, Store, now_ms};
use anyhow::{Context as _, Result, anyhow, bail};
use serde_json::{Value, json};

const OWNER: &str = "account_owner";
/// How long the relay stays up after signing a claim: the phone completes it, and the cloud says so over the relay.
const JOIN_WAIT_MS: i64 = 5 * 60 * 1000;

/// The account as last seen from the cloud; `None` when the computer isn't in one (or never heard yet), and
/// with the Codync cloud off or a private relay, where it is in no account either (as its status says).
pub fn owner(store: &Store) -> Option<Owner> {
    cloud::current(store).filter(|address| address.kind == RelayKind::Codync)?;
    recorded(store)
}

/// The account last recorded, whichever relay is in use now.
fn recorded(store: &Store) -> Option<Owner> {
    store.kv_get(OWNER).and_then(|v| serde_json::from_str::<Option<Owner>>(&v).ok()).flatten()
}

/// `CloudStatus::joins_account_on_enable`: read from what's saved, as the status says nothing of it with the cloud off.
pub fn joins_account_on_enable(store: &Store) -> bool {
    cloud::when_on(store).is_some_and(|address| address.kind == RelayKind::Codync) && recorded(store).is_none()
}

/// Records the account the cloud reports. When it is another one than before (joining one, moving, or leaving),
/// every device let in for another account goes; joining a first account keeps the phones paired before devices
/// recorded their account. In an account, phones that paired with no account always go, and a claim being completed
/// is over, so it can't outlive the account. The first record after an upgrade only records. The devices go first, so
/// a failure leaves the old account recorded and the next pull tries again.
pub fn remember(hub: &Hub, now: Option<&Owner>) -> Result<()> {
    let saved = serde_json::to_string(&now)?;
    let _change = hub.cloud.account.locked();
    if now.is_some() {
        *hub.cloud.join_pending_until.locked() = 0;
        let gone = hub.store.drop_no_account_devices()?;
        if !gone.is_empty() {
            tracing::info!(devices = gone.len(), "the computer is in an account; phones with no account are gone");
            hub.auth_changed();
        }
    }
    let before = hub.store.kv_get(OWNER);
    if before.as_deref() == Some(saved.as_str()) {
        return Ok(());
    }
    let before_user =
        before.as_deref().and_then(|b| serde_json::from_str::<Option<Owner>>(b).ok().flatten()).map(|o| o.user_id);
    let now_user = now.map(|o| o.user_id.as_str());
    if before_user.as_deref() != now_user && before.is_some() {
        let gone = hub.store.keep_devices_of(now_user, before_user.is_none())?;
        tracing::info!(devices = gone.len(), "the computer changed its account; the old account's devices are gone");
        if !gone.is_empty() {
            hub.auth_changed();
        }
    }
    hub.store.kv_set(OWNER, &saved)?;
    // Phones read the relay address in `hello`, and the relay stays up only in an account.
    hub.cloud.account_changed.send_modify(|n| *n += 1);
    hub.cloud.relay_wake.notify_one();
    Ok(())
}

/// Runs `f` while the account can't change (`remember` waits), so a phone `f` saves against the account it reads
/// can't come in after a change of account dropped the other accounts' devices.
pub fn while_unchanged<T>(hub: &Hub, f: impl FnOnce() -> T) -> T {
    let _change = hub.cloud.account.locked();
    f()
}

/// Out of an account, cloud assistants go off and the 6-digit check comes back on: the account this computer
/// joins next is asked again. A cloud computer Codync runs for its user keeps approving that account's devices: it
/// is out of the account only until its first claim completes.
pub fn out_of_account(hub: &Hub) -> Result<()> {
    if assistant::enabled(&hub.store) {
        assistant::set_enabled(hub, false)?;
    }
    if cloud::approval(&hub.store) == Approval::Auto && !super::managed::is_managed(&hub.store) {
        cloud::set_approval(hub, Approval::Code)?;
    }
    Ok(())
}

/// Whether this computer could join an account: on the Codync cloud, and in none yet.
pub fn can_join(hub: &Hub) -> bool {
    cloud::codync_url(hub).is_ok() && owner(&hub.store).is_none()
}

/// A claim was signed moments ago, and the phone or the app is completing it.
fn joining(hub: &Hub) -> bool {
    now_ms() < *hub.cloud.join_pending_until.locked()
}

/// A private relay is the user's own; the Codync relay is for a computer in an account
/// (docs/plans/optional-sign-in.md).
fn relay_is_open(hub: &Hub, address: &cloud::Address) -> bool {
    address.kind == RelayKind::Private || owner(&hub.store).is_some()
}

/// The relay address the phone with `key` gets in `hello`: on an open relay, or while a claim is being completed if
/// it has an account, as the phone finishing the join goes on through the relay.
pub fn url_for_phones(hub: &Hub, key: &str) -> Option<String> {
    let address = cloud::current(&hub.store)?;
    let has_account = || hub.store.device(key).is_some_and(|d| matches!(d.account, DeviceAccount::Of(_)));
    (relay_is_open(hub, &address) || (joining(hub) && has_account())).then_some(address.url)
}

/// Whether this computer needs its relay socket. In no account it has one only while a phone may be pairing or
/// finishing a join through it, so an unused computer holds nothing open on the Codync relay.
pub fn wants_relay(hub: &Hub, address: &cloud::Address) -> bool {
    relay_is_open(hub, address) || joining(hub) || hub.pairing.locked().open()
}

/// The names of the phones that lose access if this computer joins `user_id`'s account, leaving out `key`, which
/// asks to bring it there.
pub fn removed_by_joining(hub: &Hub, user_id: &str, key: &str) -> Vec<String> {
    let devices = hub.store.devices().unwrap_or_else(|e| {
        tracing::warn!(error = format!("{e:#}"), "couldn't list the phones a join would remove");
        vec![]
    });
    let goes = |d: &crate::store::Device| match &d.account {
        DeviceAccount::Unrecorded => false,
        DeviceAccount::SignedOut => true,
        DeviceAccount::Of(account) => account != user_id,
    };
    devices.into_iter().filter(|d| d.key != key && goes(d)).map(|d| d.name).collect()
}

/// `joinAccount`: a paired phone asks to bring this computer into its account. The computer confirms, then signs the
/// claim the phone made; the phone, which holds the account's session, completes it.
pub async fn join_from_phone(hub: &Hub, key: &str, certificate: &Value, claim_id: &str, nonce: &str) -> Result<Value> {
    if !can_join(hub) {
        bail!("This computer can't join an account now.");
    }
    let account = certificate::check(certificate, key, now_ms())
        .map_err(|_| anyhow!("Couldn't confirm your Codync account. Sign in again."))?;
    let device = hub.store.device(key).context("this phone isn't paired with this computer")?;
    let about = Question::about(hub, key, &device.name, &device.platform, Some(&account))?;
    let answer = pairing_requests::ask(hub, Question { can_join: true, phone_only: false, ..about }).await;
    crate::analytics::join_from_phone_answered(hub, answer == Answer::Join);
    if answer != Answer::Join {
        bail!("The computer stayed out of this account.");
    }
    // Finish the only asynchronous preparation first. No account or device checked before the approval is trusted
    // after that wait; revocation and a change of account must win over an old approval card.
    let device = claim_device(&hub.store).await?;
    while_unchanged(hub, || finish_phone_join(hub, key, certificate, claim_id, nonce, &device, now_ms()))
}

/// Called under the account lock. The store also holds its device lock until authorization, signing and the account
/// update finish, so a revoked device cannot be resurrected or receive a claim from a stale request.
fn finish_phone_join(
    hub: &Hub,
    key: &str,
    certificate: &Value,
    claim_id: &str,
    nonce: &str,
    device: &ClaimDevice,
    now: i64,
) -> Result<Value> {
    if !can_join(hub) {
        bail!("This computer can't join an account now.");
    }
    let account = certificate::check(certificate, key, now)
        .map_err(|_| anyhow!("Couldn't confirm your Codync account. Sign in again."))?;
    check_claim(claim_id, nonce, &account.user_id)?;
    let signed = hub.store.set_device_account(key, &account.user_id, |phone| {
        if !phone.scopes.contains(&Scope::Control)
            || (phone.source == DeviceSource::Account
                && (!hub.cloud_synced.load(std::sync::atomic::Ordering::Acquire)
                    || phone.lease_until.is_none_or(|until| until <= now)))
        {
            bail!("This phone is no longer allowed to add the computer to an account.");
        }
        Ok(signed_claim(hub, claim_id, nonce, &account.user_id, device))
    })?;
    note_signed(hub);
    Ok(signed)
}

/// Signs this computer into an account claim (§4.2 A) for a signed-in app to complete: the desktop app's
/// Add to account, or a phone pairing with a computer in no account or adding it later.
pub async fn sign_claim(hub: &Hub, claim_id: &str, nonce: &str, user_id: &str) -> Result<Value> {
    cloud::require_accounts(&hub.store)?;
    check_claim(claim_id, nonce, user_id)?;
    let device = claim_device(&hub.store).await?;
    let signed = signed_claim(hub, claim_id, nonce, user_id, &device);
    note_signed(hub);
    Ok(signed)
}

fn check_claim(claim_id: &str, nonce: &str, user_id: &str) -> Result<()> {
    for (k, v) in [("claimId", claim_id), ("nonce", nonce), ("userId", user_id)] {
        // Fields are newline-separated in the signed string: a newline would forge another field.
        if v.is_empty() || v.contains('\n') {
            bail!("`{k}` is invalid");
        }
    }
    Ok(())
}

fn note_signed(hub: &Hub) {
    *hub.cloud.join_pending_until.locked() = now_ms() + JOIN_WAIT_MS;
    hub.cloud.relay_wake.notify_one();
}

struct ClaimDevice {
    name: String,
    device: crate::service::Device,
}

/// Prepare Store-dependent and asynchronous identity before taking the account/device locks.
async fn claim_device(store: &Store) -> Result<ClaimDevice> {
    Ok(ClaimDevice {
        name: super::managed::computer_name(store),
        device: super::managed::computer_device(store).await?,
    })
}

/// Pure signing: no store access or await, so a phone join can hold both account and device locks.
fn signed_claim(hub: &Hub, claim_id: &str, nonce: &str, user_id: &str, device: &ClaimDevice) -> Value {
    let computer_id = hub.identity.computer_id();
    let box_key = hub.identity.box_pub_b64();
    let input = crypto::claim_input(claim_id, nonce, user_id, &computer_id, &box_key);
    json!({
        "computerId": computer_id,
        "signKey": hub.identity.sign_pub_b64(),
        "boxKey": box_key,
        "name": device.name,
        "platform": std::env::consts::OS,
        "device": device.device,
        "version": env!("CARGO_PKG_VERSION"),
        "sig": crypto::b64(&hub.identity.sign(input.as_bytes())),
    })
}

#[cfg(test)]
mod tests;
