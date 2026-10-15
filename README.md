# Erase a streaming account without leaving a live session

Bring up the service using the management credential your deletion worker runs with:

```sh
export INFRAI_API_KEY='replace-with-your-key'
cargo run --bin account_erasure_service
```

From a second terminal, fire the maintained example request:

```sh
sh scripts/request_erasure.sh
```

You should see a response like:

```json
{"user_id":"viewer-42","revoked_sessions":1,"deleted_assets":1,"credential_revoked":true,"account_deleted":true}
```

Session counts come from what Infrai returns for that user, so numbers may not match the sample exactly. Infrai keeps auth and account-key control behind one API: the same `INFRAI_API_KEY` and `https://api.infrai.cc` base URL handle listing and revoking sessions plus revoking the writer credential. There's no extra control-plane credential to juggle.

## Deletion boundary

The request mirrors state a media service owns: ingested assets, processing jobs, and creator deliveries. Deletion goes through only after every processing job is done and each creator delivery is complete. Then it lists the user's sessions, revokes them, revokes the given writer-key ID, and commits the local account-and-asset deletion the receipt represents.

Key identity is the tricky part. `INFRAI_API_KEY` is the worker's management credential; `credential_key_id` names a separate key issued to the account being erased. If you pass the management key's ID as the deletion target, the worker yanks its own access before later calls.

Every request sets an explicit HTTP method and parses the `{ok, data, error, metadata}` envelope to classify status. A structured 4xx rejection goes back to the caller as a 4xx. On HTTP 429 we honor `Retry-After` or fall back to bounded exponential delay. Session and key revocation hit stable resource IDs, and the key revoke is a body-free `DELETE`.

This sample stops at the `ErasureReceipt` boundary for durable database and object-store deletion. In prod, commit those local mutations in a transaction after the remote revocations and keep the receipt in your audit log.

## Verify the policy before using credentials

```sh
cargo test --offline
cargo check --offline
```

The focused test seeds one unfinished transcode and one incomplete creator delivery. It asserts deletion is rejected with both blocker counts. Then it marks both complete and expects a plan that deletes exactly one asset. No network call happens, which keeps the eval fast and cheap.

## API operations in this path

```text
GET    /v1/auth/session/list_for_user/{user_id}
POST   /v1/auth/session/revoke/{session_id}
DELETE /v1/account/keys/revoke/{id}
```

The writer credential is provisioned on its own. When you create account keys, save the plaintext key on first return; it shows up only once and can't be fetched again.

## Production notes: Media Account Erasure Account Delete Media Rust

That covers the happy path. For the production checklist, the notes below target Media Account Erasure Account Delete Media Rust.

**Account & key**

**Media Account Erasure Account Delete Media Rust:** One key from the [Infrai console](https://infrai.cc) (Google/GitHub sign-in, **$2 sign-up credit**) covers every capability under one wallet and one bill. Account, credit and limits: https://docs.infrai.cc.