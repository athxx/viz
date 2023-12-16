use std::{any::Any, panic::AssertUnwindSafe};

use futures_util::FutureExt;

use crate::{async_trait, Handler, IntoResponse, Response, Result};

/// Catches unwinding panics while calling the handler.
#[derive(Debug, Clone)]
pub struct CatchUnwind<H, F> {
    h: H,
    f: F,
}

impl<H, F> CatchUnwind<H, F> {
    /// Creates an [`CatchUnwind`] handler.
    #[inline]
    pub fn new(h: H, f: F) -> Self {
        Self { h, f }
    }
}

#[async_trait]
impl<H, F, I, O, R> Handler<I> for CatchUnwind<H, F>
where
    I: Send + 'static,
    H: Handler<I, Output = Result<O>> + Clone,
    O: IntoResponse + Send,
    F: Handler<Box<dyn Any + Send>, Output = R> + Clone,
    R: IntoResponse,
{
    type Output = Result<Response>;

    async fn call(&self, i: I) -> Self::Output {
        match AssertUnwindSafe(self.h.call(i)).catch_unwind().await {
            Ok(r) => r.map(IntoResponse::into_response),
            Err(e) => Ok(self.f.call(e).await.into_response()),
        }
    }
}
