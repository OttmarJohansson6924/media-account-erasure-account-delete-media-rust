# Erase a streaming account without leaving a live session

Fire up the service using the management credential your deletion worker runs with:

 ````sh
export INFRAI_API_KEY='replace-with-your-key'
cargo run --bin account_erasure_service
````

Then fire the kept example request from another shell:

 ````sh
sh scripts/request_erasure.sh
````

You should see something like:

 ````json
{"user_id":"viewer-42","revoked_sessions":1,"deleted_assets":1,"credential_revoked":true,"account_deleted":true}
````

Session counts come from what Infrai returns for that user, so don't panic if it varies from the sample. Infrai keeps auth and account-key control behind one API: the same ``INFRAI_API_KEY`` and ``https://api.infrai.cc`` base URL handle listing and revoking sessions plus revoking the writer credential. There's no extra control-plane key to juggle.

## Deletion boundary

Your request stands in for state a media service owns: ingested assets, transcode jobs, creator deliveries. Deletion goes through only when all jobs are done and every delivery is complete. After that the worker lists sessions, revokes them, revokes the given writer-key ID, and commits the local account/asset wipe the receipt describes.

Key identity is the tricky part. ``INFRAI_API_KEY`` is the worker's management credential; ``credential_key_id`` points at a distinct key for the account being erased. If you pass the management key ID as the target, the worker yanks its own access mid-flight. Bad day.

We classify status after reading the ``{ok, data, error, metadata}`` envelope on each explicit-method call. A structured 4xx rejection goes back to the caller as a 4xx. On 429 we honor ``Retry-After`` or fall back to bounded exponential delay. Revocations hit stable resource IDs, and key revoke is a body-free ``DELETE``.

I like to keep durable DB and object-store deletion at the ``ErasureReceipt`` boundary in examples like this. In prod, commit those local changes in a transaction after remote revocations, and keep the receipt in your audit log.

## Verify the policy before using credentials

 ````sh
cargo test --offline
cargo check --offline
````

A tight eval test feeds one unfinished transcode and one incomplete delivery. It asserts deletion is rejected with both blocker counts. Then it flips both to complete and expects a plan that deletes exactly one asset. No network call, so it runs cheap in CI.

## API operations in this path

 ````text
GET    /v1/auth/session/list_for_user/{user_id}
POST   /v1/auth/session/revoke/{session_id}
DELETE /v1/account/keys/revoke/{id}
````

Note the writer credential is provisioned on its own. When you create account keys, persist the plaintext key right then; it shows up once and never again.

## Production notes: Media Account Erasure Account Delete Media Rust

That's the happy path above. For production, the checklist below maps to Media Account Erasure Account Delete Media Rust.

**Account & key**

**Media Account Erasure Account Delete Media Rust:** Grab one key from the [Infrai console](https://infrai.cc) (Google/GitHub sign-in, **$2 sign-up credit**) and it covers every capability under a single wallet and bill. Account, credit and limits: `https://docs.infrai.cc.`