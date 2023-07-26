[![License](https://img.shields.io/crates/l/hyper-trace-id.svg)](https://choosealicense.com/licenses/mit/)
[![Crates.io](https://img.shields.io/crates/v/hyper-trace-id.svg)](https://crates.io/crates/hyper-trace-id)
[![Docs.rs](https://docs.rs/hyper-trace-id/badge.svg)](https://docs.rs/hyper-trace-id)


# hyper-trace-id

[hyper] ([axum], [warp], [poem], etc.) middleware for adding trace ids to requests.

# Basic Usage

Adding the `SetTraceIdLayer<T>` layer will make `TraceId<T>` available via the request and 
response extensions. For special use-cases (e.g. lazily generating trace ids only in case of http errors) you can implement `MakeTraceId` on your own types.

```rust
use std::convert::Infallible;
use hyper::{Body, Request, Response};
use tower::ServiceBuilder;
use hyper_trace_id::{SetTraceIdLayer, TraceId};

let trace_id_header = "x-trace-id";
let svc = ServiceBuilder::new()
    .layer(SetTraceIdLayer::<String>::new().with_header_name(trace_id_header))
    .service_fn(|_req: Request<Body>| async {
        let res: Result<Response<Body>, Infallible> = Ok(Response::new(Body::empty()));
        res
    });
```

# Use with [axum]
For axum users, the crate optionally provides an extractor (via the `axum` feature) to access the trace id in a handler.

```rust
use axum::{routing::get, Router};
use axum_trace_id::{SetTraceIdLayer, TraceId};

let app: Router = Router::new()
     .route(
         "/",
         get(|trace_id: TraceId<String>| async move { trace_id.to_string() }),
     )
     .layer(SetTraceIdLayer::<String>::new());
```

# Use with [tracing]
To use with [tracing], you can access the requests tracing id via the extensions.

```rust
use axum::{http::Request, routing::get, Router};
use axum_trace_id::{SetTraceIdLayer, TraceId};
use tower_http::trace::TraceLayer;
use tracing::info_span;

let app = Router::new()
    .route("/", get(|| async { "" }))
    .layer(TraceLayer::new_for_http().make_span_with(|request: &Request<_>| {
        let trace_id = request.extensions().get::<TraceId<String>>().unwrap();

        info_span!("http_request", trace_id = trace_id)
    }));
```

# License

This project is licensed under the [MIT license](LICENSE).

[poem]: https://github.com/poem-web/poem
[warp]: https://github.com/seanmonstar/warp
[hyper]: https://hyper.rs/
[axum]: https://crates.io/crates/axum
[tracing]: https://crates.io/crates/tracing