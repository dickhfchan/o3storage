use axum::{
    routing::{get, put, post, delete, head},
    Router,
    extract::State,
};
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    trace::TraceLayer,
};
use std::sync::Arc;
use tokio::net::TcpListener;

use crate::{ApiResult, ApiError};
use crate::handlers::{AppState, *};

pub struct Server {
    config: crate::Config,
    app_state: Arc<AppState>,
}

impl Server {
    pub async fn new(
        config: crate::Config,
        storage_engine: Arc<storage::StorageEngine>,
        consensus_manager: Arc<consensus::ConsensusManager>,
        cluster_state: Arc<tokio::sync::RwLock<crate::ClusterState>>,
    ) -> ApiResult<Self> {
        let app_state = Arc::new(AppState {
            storage_engine,
            consensus_manager,
            cluster_state,
        });

        Ok(Self {
            config,
            app_state,
        })
    }

    pub async fn start(&self) -> ApiResult<()> {
        let app = self.create_router();
        
        let addr = self.config.bind_address();
        tracing::info!("Starting API server on {}", addr);
        
        let listener = TcpListener::bind(&addr).await
            .map_err(|e| ApiError::InternalError(format!("Failed to bind to {}: {}", addr, e)))?;
        
        axum::serve(listener, app).await
            .map_err(|e| ApiError::InternalError(format!("Server error: {}", e)))?;
        
        Ok(())
    }

    fn create_router(&self) -> Router {
        let api_routes = Router::new()
            // Bucket operations
            .route("/", get(list_buckets))
            .route("/:bucket", put(create_bucket))
            .route("/:bucket", get(list_objects_v2))
            
            // Object operations
            .route("/:bucket/:key", put(put_object))
            .route("/:bucket/:key", get(get_object))
            .route("/:bucket/:key", head(head_object))
            .route("/:bucket/:key", delete(delete_object))
            
            // Health check
            .route("/health", get(health_check))
            
            .with_state(self.app_state.clone());

        Router::new()
            .nest("/", api_routes)
            .layer(
                ServiceBuilder::new()
                    .layer(TraceLayer::new_for_http())
                    .layer(CorsLayer::permissive())
            )
            .fallback(not_found)
    }
}

async fn not_found() -> ApiResult<axum::response::Response> {
    Err(ApiError::InvalidRequest("Not found".to_string()))
}