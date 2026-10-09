use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use media_account_erasure::{
    infrai_client::{InfraiClient, InfraiError},
    AccountErasureService, ErasureError, MediaAccount,
};
use serde_json::json;
use std::{env, sync::Arc};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let key = env::var("INFRAI_API_KEY")?;
    let service = Arc::new(AccountErasureService::new(InfraiClient::new(key)));
    let app = Router::new()
        .route("/accounts/delete", post(delete_account))
        .with_state(service);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    println!("account erasure service listening on http://127.0.0.1:3000");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn delete_account(
    State(service): State<Arc<AccountErasureService>>,
    Json(account): Json<MediaAccount>,
) -> Result<impl IntoResponse, ServiceError> {
    let receipt = service.erase(account).await?;
    Ok((StatusCode::OK, Json(receipt)))
}

struct ServiceError(ErasureError);

impl From<ErasureError> for ServiceError {
    fn from(error: ErasureError) -> Self {
        Self(error)
    }
}

impl IntoResponse for ServiceError {
    fn into_response(self) -> Response {
        let (status, message) = match self.0 {
            ErasureError::Policy(error) => (StatusCode::CONFLICT, error.to_string()),
            ErasureError::Infrai(InfraiError::Rejected { status, code, .. }) => {
                (status, format!("Infrai rejected the operation: {code}"))
            }
            ErasureError::Infrai(error) => {
                eprintln!("account erasure dependency error: {error}");
                (StatusCode::BAD_GATEWAY, "dependency request failed".into())
            }
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}
