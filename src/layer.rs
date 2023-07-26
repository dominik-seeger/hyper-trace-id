use std::marker::PhantomData;
use std::str::FromStr;
use std::task::{Context, Poll};

use futures::future::BoxFuture;
use hyper::http::{HeaderName, HeaderValue, Request, Response};
use tower::{Layer, Service};

use crate::{MakeTraceId, TraceId};

/// Add the TraceId<T> extension to requests and optionally include trace ids in request and response headers.
///
/// ```
/// use std::convert::Infallible;
/// use hyper::{Body, Request, Response};
/// use tower::ServiceBuilder;
/// use hyper_trace_id::{SetTraceIdLayer, TraceId};
///
/// let trace_id_header = "x-trace-id";
/// let svc = ServiceBuilder::new()
///     .layer(SetTraceIdLayer::<String>::new().with_header_name(trace_id_header))
///     .service_fn(|_req: Request<Body>| async {
///         let res: Result<Response<Body>, Infallible> = Ok(Response::new(Body::empty()));
///         res
///     });
///
/// ```
#[derive(Debug, Clone)]
pub struct SetTraceIdLayer<T>
where
    T: MakeTraceId,
{
    header_name: Option<HeaderName>,
    _phantom: PhantomData<T>,
}

impl<T> SetTraceIdLayer<T>
where
    T: MakeTraceId,
{
    pub fn new() -> Self {
        Self {
            header_name: None,
            _phantom: Default::default(),
        }
    }

    pub fn with_header_name(self, header_name: &str) -> Self {
        Self {
            header_name: Some(HeaderName::from_str(header_name).unwrap()),
            _phantom: Default::default(),
        }
    }
}

impl<T> Default for SetTraceIdLayer<T>
where
    T: MakeTraceId,
{
    fn default() -> Self {
        SetTraceIdLayer::new()
    }
}

impl<S, T> Layer<S> for SetTraceIdLayer<T>
where
    T: MakeTraceId,
{
    type Service = TraceIdMiddleware<S, T>;

    fn layer(&self, inner: S) -> Self::Service {
        TraceIdMiddleware {
            inner,
            header_name: self.header_name.clone(),
            _phantom: Default::default(),
        }
    }
}

#[derive(Clone)]
pub struct TraceIdMiddleware<S, T> {
    inner: S,
    header_name: Option<HeaderName>,
    _phantom: PhantomData<T>,
}

impl<S, T, Rq, Rs> Service<Request<Rq>> for TraceIdMiddleware<S, T>
where
    S: Service<Request<Rq>, Response = Response<Rs>> + Send + 'static,
    S::Future: Send + 'static,
    T: MakeTraceId + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<Rq>) -> Self::Future {
        let trace_id = TraceId::<T>::new();
        req.extensions_mut().insert(trace_id.clone());

        // Add TraceId header to request
        let mut header_val: Option<HeaderValue> = None;
        if let Some(header_name) = self.header_name.clone() {
            header_val = Some(
                HeaderValue::try_from(trace_id.id.to_string())
                    .unwrap_or(HeaderValue::from_static("unavailable")),
            );
            req.headers_mut()
                .insert(header_name, header_val.clone().unwrap());
        }

        let future = self.inner.call(req);
        let moved_header_name = self.header_name.clone();
        Box::pin(async move {
            let mut response: Response<Rs> = future.await?;

            // Add TraceId header to response
            if let Some(header_name) = moved_header_name {
                response
                    .headers_mut()
                    .insert(header_name, header_val.unwrap());
            }

            Ok(response)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::body::Body;
    use std::cell::RefCell;
    use std::convert::Infallible;
    use std::sync::{Arc, Mutex};
    use tower::{ServiceBuilder, ServiceExt};

    #[tokio::test]
    async fn test_extension_not_added() {
        // Gets set to 1 when the assert_no_trace_id was called.
        let call_arc = Arc::new(RefCell::new(0));

        let assert_no_trace_id = |mut req: Request<Body>| -> Request<Body> {
            call_arc.replace(1);
            assert!(req.extensions_mut().get::<TraceId<String>>().is_none());
            req
        };

        let test_svc = ServiceBuilder::new()
            .map_request(assert_no_trace_id)
            .service_fn(|_req: Request<Body>| async {
                let res: Result<(), Infallible> = Ok(());
                res
            });

        let req = Request::new(Body::empty());
        test_svc.oneshot(req).await.unwrap();

        // Assert that assert_no_trace_id was actually called
        assert_eq!(call_arc.take(), 1)
    }

    #[tokio::test]
    async fn test_extension_added() {
        // Gets set to 1 when the assert_trace_id was called.
        let call_arc = Arc::new(Mutex::new(0));

        let moved_call_arc = call_arc.clone();
        let assert_trace_id = move |mut req: Request<Body>| -> Request<Body> {
            let mut calls = moved_call_arc.lock().unwrap();
            *calls = 1;
            assert!(req.extensions_mut().get::<TraceId<String>>().is_some());
            req
        };

        let test_svc = ServiceBuilder::new()
            .layer(SetTraceIdLayer::<String>::new())
            .map_request(assert_trace_id)
            .service_fn(|_req: Request<Body>| async {
                let res: Result<Response<Body>, Infallible> = Ok(Response::new(Body::empty()));
                res
            });

        let req = Request::new(Body::empty());
        test_svc.oneshot(req).await.unwrap();

        // Assert that assert_trace_id was actually called
        let calls = call_arc.lock().unwrap();
        assert_eq!(*calls, 1);
    }

    #[tokio::test]
    async fn test_header_added() {
        let header_name = "x-trace-id";

        // Gets set to 1 when the assert_trace_id was called.
        let call_arc = Arc::new(Mutex::new(0));

        let moved_call_arc = call_arc.clone();
        let assert_trace_id = move |mut req: Request<Body>| -> Request<Body> {
            let mut calls = moved_call_arc.lock().unwrap();
            *calls = 1;
            assert!(req.extensions_mut().get::<TraceId<String>>().is_some());
            req
        };

        let test_svc = ServiceBuilder::new()
            .layer(SetTraceIdLayer::<String>::new().with_header_name(header_name))
            .map_request(assert_trace_id)
            .service_fn(|_req: Request<Body>| async {
                let res: Result<Response<Body>, Infallible> = Ok(Response::new(Body::empty()));
                res
            });

        let req = Request::new(Body::empty());
        let resp = test_svc.oneshot(req).await.unwrap();

        assert!(resp.headers().get(header_name).is_some());
    }
}
