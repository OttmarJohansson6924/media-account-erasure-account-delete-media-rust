#!/bin/sh
set -eu

curl -sS -X POST http://127.0.0.1:3000/accounts/delete \
  -H 'Content-Type: application/json' \
  -d '{
    "user_id": "viewer-42",
    "credential_key_id": "key-media-writer",
    "assets": [{"asset_id": "asset-opening-credits"}],
    "processing_jobs": [{"job_id": "transcode-7", "finished": true}],
    "creator_deliveries": [{"delivery_id": "creator-master-9", "delivered": true}]
  }'

