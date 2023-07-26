use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::{MakeTraceId, TraceId};

const MISSING_TRACE_ID_ERROR: &str = "Unable to extract TraceId: Missing TraceId extension.";

pub enum TraceIdRejection {
    MissingTraceId,
}

impl IntoResponse for TraceIdRejection {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            MISSING_TRACE_ID_ERROR.to_string(),
        )
            .into_response()
    }
}

#[async_trait]
impl<S, T> FromRequestParts<S> for TraceId<T>
where
    S: Send + Sync,
    T: MakeTraceId + 'static,
{
    type Rejection = TraceIdRejection;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        match parts.extensions.get::<Self>() {
            None => Err(TraceIdRejection::MissingTraceId),
            Some(trace_id) => Ok(trace_id.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layer::SetTraceIdLayer;
    use axum::{body::Body, http::Request, routing::get, Router};
    use std::fmt::{Display, Formatter};
    use tower::ServiceExt;

    #[derive(Debug, Clone)]
    struct MockTraceId {
        id: String,
    }

    impl Display for MockTraceId {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.id)
        }
    }

    impl MakeTraceId for MockTraceId {
        fn make_trace_id() -> Self {
            Self {
                id: String::from("mock_id"),
            }
        }
    }

    #[tokio::test]
    async fn trace_id_rejection() {
        let app = Router::new().route("/", get(|_trace_id: TraceId<String>| async { "" }));
        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        assert_eq!(&body[..], MISSING_TRACE_ID_ERROR.as_bytes());
    }

    #[tokio::test]
    async fn trace_id_string() {
        async fn handle(trace_id: TraceId<String>) -> impl IntoResponse {
            format!("TraceId={trace_id}")
        }

        let app = Router::new()
            .route("/", get(handle))
            .layer(SetTraceIdLayer::<String>::new());

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn trace_id_extracted() {
        async fn handle(trace_id: TraceId<MockTraceId>) -> impl IntoResponse {
            format!("TraceId={trace_id}")
        }

        let expected_uid = "mock_id";
        let app = Router::new()
            .route("/", get(handle))
            .layer(SetTraceIdLayer::<MockTraceId>::new());

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        assert_eq!(
            String::from_utf8(body.to_vec()).unwrap(),
            format!("TraceId={expected_uid}")
        );
    }
}
