//! Route

use core::fmt;

use viz_core::{
    BoxHandler, Handler, HandlerExt, IntoResponse, Method, Next, Request, Responder, Response,
    Result, Transform,
};

macro_rules! repeat {
    ($macro:ident $($name:ident $verb:tt )+) => {
        $(
            $macro!($name $verb);
        )+
    };
}

macro_rules! export_internal_verb {
    ($name:ident $verb:tt) => {
        #[doc = concat!(" Appends a route, handle HTTP verb `", stringify!($verb), "`.")]
        pub fn $name<H, O>(self, handler: H) -> Self
        where
            H: Handler<Request, Output = Result<O>> + Clone,
            O: IntoResponse + Send + Sync + 'static,
        {
            self.on(Method::$verb, handler)
        }
    };
}

macro_rules! export_verb {
    ($name:ident $verb:ty) => {
        #[doc = concat!(" Appends a route, handle HTTP verb `", stringify!($verb), "`.")]
        pub fn $name<H, O>(handler: H) -> Route
        where
            H: Handler<Request, Output = Result<O>> + Clone,
            O: IntoResponse + Send + Sync + 'static,
        {
            Route::new().$name(handler)
        }
    };
}

#[derive(Clone, Default)]
pub struct Route {
    pub(crate) methods: Vec<(Method, BoxHandler)>,
}

impl Route {
    pub fn new() -> Self {
        Self {
            methods: Vec::new(),
        }
    }

    pub fn push(mut self, method: Method, handler: BoxHandler) -> Self {
        match self
            .methods
            .iter_mut()
            .find(|(m, _)| m == method)
            .map(|(_, e)| e)
        {
            Some(h) => *h = handler,
            None => self.methods.push((method, handler)),
        }

        self
    }

    /// Appends a route, with a HTTP verb and handler.
    pub fn on<H, O>(self, method: Method, handler: H) -> Self
    where
        H: Handler<Request, Output = Result<O>> + Clone,
        O: IntoResponse + Send + Sync + 'static,
    {
        self.push(method, Responder::new(handler).boxed())
    }

    /// Appends a route, with a HTTP verb and handler.
    pub fn any<H, O>(self, handler: H) -> Self
    where
        H: Handler<Request, Output = Result<O>> + Clone,
        O: IntoResponse + Send + Sync + 'static,
    {
        [
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::HEAD,
            Method::OPTIONS,
            Method::CONNECT,
            Method::PATCH,
            Method::TRACE,
        ]
        .into_iter()
        .fold(self, |route, method| route.on(method, handler.clone()))
    }

    repeat!(
        export_internal_verb
        get GET
        post POST
        put PUT
        delete DELETE
        head HEAD
        options OPTIONS
        connect CONNECT
        patch PATCH
        trace TRACE
    );

    pub fn with<T>(self, t: T) -> Self
    where
        T: Transform<BoxHandler>,
        T::Output: Handler<Request, Output = Result<Response>>,
    {
        self.into_iter()
            .map(|(method, handler)| (method, t.transform(handler).boxed()))
            .collect()
    }

    pub fn with_handler<F>(self, f: F) -> Self
    where
        F: Handler<Next<Request, BoxHandler>, Output = Result<Response>> + Clone,
    {
        self.into_iter()
            .map(|(method, handler)| (method, handler.around(f.clone()).boxed()))
            .collect()
    }

    pub fn map_handler<F>(self, f: F) -> Self
    where
        F: Fn(BoxHandler) -> BoxHandler,
    {
        self.into_iter()
            .map(|(method, handler)| (method, f(handler)))
            .collect()
    }
}

impl IntoIterator for Route {
    type Item = (Method, BoxHandler);

    type IntoIter = std::vec::IntoIter<(Method, BoxHandler)>;

    fn into_iter(self) -> Self::IntoIter {
        self.methods.into_iter()
    }
}

impl FromIterator<(Method, BoxHandler)> for Route {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = (Method, BoxHandler)>,
    {
        Self {
            methods: iter.into_iter().collect(),
        }
    }
}

/// Appends a route, with a HTTP verb and handler.
pub fn on<H, O>(method: Method, handler: H) -> Route
where
    H: Handler<Request, Output = Result<O>> + Clone,
    O: IntoResponse + Send + Sync + 'static,
{
    Route::new().on(method, handler)
}

repeat!(
    export_verb
    get GET
    post POST
    put PUT
    delete DELETE
    head HEAD
    options OPTIONS
    connect CONNECT
    patch PATCH
    trace TRACE
);

/// Appends a route, with handler by any HTTP verbs.
pub fn any<H, O>(handler: H) -> Route
where
    H: Handler<Request, Output = Result<O>> + Clone,
    O: IntoResponse + Send + Sync + 'static,
{
    Route::new().any(handler)
}

#[cfg(feature = "ext")]
mod ext;
#[cfg(feature = "ext")]
pub use ext::*;

impl fmt::Debug for Route {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Route")
            .field(
                "methods",
                &self
                    .methods
                    .iter()
                    .map(|(m, _)| m)
                    .collect::<Vec<&Method>>(),
            )
            .finish()
    }
}

#[cfg(test)]
#[allow(dead_code)]
mod tests {
    use super::Route;
    use std::sync::Arc;
    use viz_core::{
        async_trait,
        handler::Transform,
        types::{Data, Query},
        Handler, HandlerExt, IntoResponse, Method, Next, Request, Response, Result,
    };

    #[tokio::test]
    async fn route() -> anyhow::Result<()> {
        async fn handler(_: Request) -> Result<impl IntoResponse> {
            Ok(())
        }

        struct Logger;

        impl Logger {
            fn new() -> Self {
                Self
            }
        }

        impl<H: Clone> Transform<H> for Logger {
            type Output = LoggerHandler<H>;

            fn transform(&self, h: H) -> Self::Output {
                LoggerHandler(h)
            }
        }

        #[derive(Clone)]
        struct LoggerHandler<H>(H);

        #[async_trait]
        impl<H> Handler<Request> for LoggerHandler<H>
        where
            H: Handler<Request> + Clone,
        {
            type Output = H::Output;

            async fn call(&self, req: Request) -> Self::Output {
                self.0.call(req).await
            }
        }

        async fn before(req: Request) -> Result<Request> {
            Ok(req)
        }

        async fn after(res: Result<Response>) -> Result<Response> {
            res
        }

        async fn around<H, O>((req, handler): Next<Request, H>) -> Result<Response>
        where
            H: Handler<Request, Output = Result<O>> + Clone,
            O: IntoResponse + Send + Sync + 'static,
        {
            handler.call(req).await.map(IntoResponse::into_response)
        }

        async fn around_1<H, O>((req, handler): Next<Request, H>) -> Result<Response>
        where
            H: Handler<Request, Output = Result<O>> + Clone,
            O: IntoResponse + Send + Sync + 'static,
        {
            handler.call(req).await.map(IntoResponse::into_response)
        }

        async fn around_2<H>((req, handler): Next<Request, H>) -> Result<Response>
        where
            H: Handler<Request, Output = Result<Response>> + Clone,
        {
            handler.call(req).await
        }

        #[derive(Clone)]
        struct Around2 {
            name: String,
        }

        #[async_trait]
        impl<H, I, O> Handler<Next<I, H>> for Around2
        where
            I: Send + 'static,
            H: Handler<I, Output = Result<O>> + Clone,
        {
            type Output = H::Output;

            async fn call(&self, (i, h): Next<I, H>) -> Self::Output {
                h.call(i).await
            }
        }

        #[derive(Clone)]
        struct Around3 {
            name: String,
        }

        #[async_trait]
        impl<H, O> Handler<Next<Request, H>> for Around3
        where
            H: Handler<Request, Output = Result<O>> + Clone,
            O: IntoResponse,
        {
            type Output = Result<Response>;

            async fn call(&self, (i, h): Next<Request, H>) -> Self::Output {
                h.call(i).await.map(IntoResponse::into_response)
            }
        }

        #[derive(Clone)]
        struct Around4 {
            name: String,
        }

        #[async_trait]
        impl<H> Handler<Next<Request, H>> for Around4
        where
            H: Handler<Request, Output = Result<Response>> + Clone,
        {
            type Output = Result<Response>;

            async fn call(&self, (i, h): Next<Request, H>) -> Self::Output {
                h.call(i).await
            }
        }

        async fn ext(_: Query<usize>, _: Data<Arc<String>>) -> Result<impl IntoResponse> {
            Ok(vec![233])
        }

        let route = Route::new()
            .any_ext(ext)
            .on(Method::GET, handler.before(before))
            .on(Method::POST, handler.after(after))
            .put(handler.around(Around2 {
                name: "handler around".to_string(),
            }))
            .with(Logger::new())
            .map_handler(|handler| {
                handler
                    .before(before)
                    .around(Around4 {
                        name: "4".to_string(),
                    })
                    .after(after)
                    .around(around_2)
                    .around(Around2 {
                        name: "2".to_string(),
                    })
                    .around(around)
                    .around(around_1)
                    .around(Around3 {
                        name: "3".to_string(),
                    })
                    .with(Logger::new())
                    .boxed()
            })
            .with_handler(around)
            .with_handler(around_1)
            .with_handler(around_2)
            .with_handler(Around2 {
                name: "2 with handler".to_string(),
            })
            .with_handler(Around3 {
                name: "3 with handler".to_string(),
            })
            .with_handler(Around4 {
                name: "4 with handler".to_string(),
            })
            // .with(viz_core::middleware::cookie::Config::new())
            .into_iter()
            .map(|(method, handler)| (method, handler))
            // .filter(|(method, _)| method != Method::GET)
            .collect::<Route>();

        let (_, h) = route
            .methods
            .iter()
            .find(|(m, _)| m == Method::GET)
            .unwrap();

        let res = match h.call(Request::default()).await {
            Ok(r) => r,
            Err(e) => e.into_response(),
        };
        assert_eq!(hyper::body::to_bytes(res.into_body()).await?, "");

        Ok(())
    }
}