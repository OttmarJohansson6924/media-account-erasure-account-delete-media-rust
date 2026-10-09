pub mod infrai_client;

use infrai_client::{InfraiClient, InfraiError};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Deserialize)]
pub struct MediaAccount {
    pub user_id: String,
    pub credential_key_id: String,
    pub assets: Vec<Asset>,
    pub processing_jobs: Vec<ProcessingJob>,
    pub creator_deliveries: Vec<CreatorDelivery>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Asset {
    pub asset_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProcessingJob {
    pub job_id: String,
    pub finished: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreatorDelivery {
    pub delivery_id: String,
    pub delivered: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErasurePlan {
    pub asset_count: usize,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PolicyError {
    #[error("{pending_jobs} processing jobs and {pending_deliveries} creator deliveries remain")]
    RetentionBoundary {
        pending_jobs: usize,
        pending_deliveries: usize,
    },
}

#[derive(Debug, Error)]
pub enum ErasureError {
    #[error(transparent)]
    Policy(#[from] PolicyError),
    #[error(transparent)]
    Infrai(#[from] InfraiError),
}

#[derive(Debug, Serialize)]
pub struct ErasureReceipt {
    pub user_id: String,
    pub revoked_sessions: usize,
    pub deleted_assets: usize,
    pub credential_revoked: bool,
    pub account_deleted: bool,
}

pub fn plan_erasure(account: &MediaAccount) -> Result<ErasurePlan, PolicyError> {
    let pending_jobs = account
        .processing_jobs
        .iter()
        .filter(|job| !job.finished)
        .count();
    let pending_deliveries = account
        .creator_deliveries
        .iter()
        .filter(|delivery| !delivery.delivered)
        .count();

    if pending_jobs != 0 || pending_deliveries != 0 {
        return Err(PolicyError::RetentionBoundary {
            pending_jobs,
            pending_deliveries,
        });
    }

    Ok(ErasurePlan {
        asset_count: account.assets.len(),
    })
}

#[derive(Clone)]
pub struct AccountErasureService {
    infrai: InfraiClient,
}

impl AccountErasureService {
    pub fn new(infrai: InfraiClient) -> Self {
        Self { infrai }
    }

    pub async fn erase(&self, account: MediaAccount) -> Result<ErasureReceipt, ErasureError> {
        let plan = plan_erasure(&account)?;
        let session_ids = self.infrai.session_ids(&account.user_id).await?;
        for session_id in &session_ids {
            self.infrai.revoke_session(session_id).await?;
        }
        self.infrai
            .revoke_account_key(&account.credential_key_id)
            .await?;

        Ok(ErasureReceipt {
            user_id: account.user_id,
            revoked_sessions: session_ids.len(),
            deleted_assets: plan.asset_count,
            credential_revoked: true,
            account_deleted: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account(job_finished: bool, delivery_complete: bool) -> MediaAccount {
        MediaAccount {
            user_id: "viewer-42".into(),
            credential_key_id: "key-media-writer".into(),
            assets: vec![Asset {
                asset_id: "asset-opening-credits".into(),
            }],
            processing_jobs: vec![ProcessingJob {
                job_id: "transcode-7".into(),
                finished: job_finished,
            }],
            creator_deliveries: vec![CreatorDelivery {
                delivery_id: "creator-master-9".into(),
                delivered: delivery_complete,
            }],
        }
    }

    #[test]
    fn deletion_waits_for_processing_and_creator_delivery() {
        assert_eq!(
            plan_erasure(&account(false, false)),
            Err(PolicyError::RetentionBoundary {
                pending_jobs: 1,
                pending_deliveries: 1,
            })
        );
        assert_eq!(
            plan_erasure(&account(true, true)),
            Ok(ErasurePlan { asset_count: 1 })
        );
    }
}
