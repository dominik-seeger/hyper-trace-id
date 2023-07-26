#[cfg(feature = "axum")]
mod extract;

mod layer;

pub use crate::layer::SetTraceIdLayer;
use std::fmt::{Display, Formatter};
use uuid::Uuid;

/// Make a type usable as trace id.
///
/// ```
/// use std::fmt::{Display, Formatter};
/// use hyper_trace_id::MakeTraceId;
/// use uuid::Uuid;
///
/// #[derive(Clone)]
/// struct MyTraceId {
///     id: String,
/// }
///
/// impl Display for MyTraceId {
///     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
///         write!(f, "{}", self.id)
///     }
/// }
///
/// impl MakeTraceId for MyTraceId {
///     fn make_trace_id() -> Self {
///         Self {
///             id: Uuid::new_v4().to_string()
///         }
///     }
/// }
/// ```
pub trait MakeTraceId: Send + Sync + Display + Clone {
    fn make_trace_id() -> Self;
}

impl MakeTraceId for String {
    fn make_trace_id() -> Self {
        Uuid::new_v4().to_string()
    }
}

#[derive(Debug, Clone)]
pub struct TraceId<T>
where
    T: MakeTraceId,
{
    pub id: T,
}

impl<T> TraceId<T>
where
    T: MakeTraceId,
{
    pub(crate) fn new() -> Self {
        TraceId {
            id: T::make_trace_id(),
        }
    }
}

impl<T> Display for TraceId<T>
where
    T: MakeTraceId,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}
