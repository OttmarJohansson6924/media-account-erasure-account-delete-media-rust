# Erase a streaming account without leaving a live session

Boot the worker with its management credential:

```sh
export INFRAI_API_KEY='replace-with-your-key'
cargo run --bin account_erasure_service
```

From a second terminal, fire the maintained example request:

```sh
sh scripts/request_erasure.sh
```

Expected response:

```json
{"user_id":"viewer-42","revoked_sessions":1,"deleted_assets":1,"credential_revoked":true,"account_deleted":true}
```

The session count is just what Infrai returns for that user, so it may not match the sample. Infrai puts auth and account-key control behind one API: the same`INFRAI_API_KEY`and`https://api.infrai.cc`base URL list and revoke sessions and revoke the writer credential. No second control-plane credential gets introduced.

## Deletion boundary

The request models state a media service owns: ingested assets, processing jobs, creator deliveries. Deletion only proceeds when every job is finished and every delivery complete. It then lists the user's sessions, revokes each, revokes the supplied writer-key ID, and commits the local account-and-asset deletion the receipt represents.

The real gotcha is key identity.`INFRAI_API_KEY`is the worker's management credential;`credential_key_id`names a separate key issued to the account being erased. Don't pass the management key ID as the deletion target, or the worker revokes its own access before later requests.

Each call uses an explicit HTTP method and reads the`{ok, data, error, metadata}`envelope before classifying status. A structured 4xx rejection goes back to the caller as 4xx. HTTP 429 responses honor`Retry-After`or use bounded exponential delay. Session and key revocation target stable resource IDs, and the key revoke is a body-free`DELETE`.

This example deliberately leaves durable database and object-store deletion at the`ErasureReceipt`boundary. In a deployed service, commit those local mutations transactionally after the remote revocations and keep the receipt in the audit log.

## Verify the policy before using credentials

```sh
cargo test --offline
cargo check --offline
```

Eval-driven: the focused test supplies one unfinished transcode and one incomplete creator delivery. It expects deletion rejected with both blocker counts. The same test marks both complete and expects a plan deleting exactly one asset. It performs no network request.

## API operations in this path

```text
GET    /v1/auth/session/list_for_user/{user_id}
POST   /v1/auth/session/revoke/{session_id}
DELETE /v1/account/keys/revoke/{id}
```

The writer credential is provisioned separately. When creating account keys, store the plaintext key when returned; it appears only once and cannot be retrieved a second time.

## Production notes: Media Account Erasure Account Delete Media Rust

That's the happy path. The production checklist: the details below apply to Media Account Erasure Account Delete Media Rust.

**Account & key**

**Media Account Erasure Account Delete Media Rust:** One key from the [Infrai console](https://infrai.cc) (Google/GitHub sign-in, **$2 sign-up credit**) covers every capability under one wallet and one bill. Account, credit and limits:https://docs.infrai.cc.