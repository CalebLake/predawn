use std::sync::{Arc, Mutex};

use hyper::body::Incoming;
use predawn_core::{error::Error, into_response::IntoResponse};
use tower::{Layer, Service};

use self::private::{HandlerToService, ServiceToHandler};
use super::Middleware;
use crate::handler::Handler;

pub trait TowerLayerCompatExt {
    fn compat(self) -> TowerCompatMiddleware<Self>
    where
        Self: Sized,
    {
        TowerCompatMiddleware(self)
    }
}

impl<L> TowerLayerCompatExt for L {}

pub struct TowerCompatMiddleware<L>(L);

impl<H, L> Middleware<H> for TowerCompatMiddleware<L>
where
    H: Handler,
    L: Layer<HandlerToService<H>>,
    L::Service: Service<http::Request<Incoming>> + Send + Sync + 'static,
    <L::Service as Service<http::Request<Incoming>>>::Future: Send,
    <L::Service as Service<http::Request<Incoming>>>::Response: IntoResponse,
    <L::Service as Service<http::Request<Incoming>>>::Error: Into<Error>,
{
    type Output = ServiceToHandler<L::Service>;

    fn transform(self, input: H) -> Self::Output {
        let svc = self.0.layer(HandlerToService(Arc::new(input)));
        ServiceToHandler(Arc::new(Mutex::new(svc)))
    }
}

mod private {
    use std::{
        future::poll_fn,
        sync::{Arc, Mutex},
        task::{Context, Poll},
    };

    use futures_util::{future::BoxFuture, FutureExt};
    use hyper::body::Incoming;
    use predawn_core::{
        error::Error, into_response::IntoResponse, request::Request, response::Response,
    };
    use tower::Service;

    use crate::handler::Handler;

    pub struct HandlerToService<H>(pub Arc<H>);

    impl<H> Service<http::Request<Incoming>> for HandlerToService<H>
    where
        H: Handler,
    {
        type Error = Error;
        type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;
        type Response = Response;

        fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, req: http::Request<Incoming>) -> Self::Future {
            let handler = self.0.clone();

            let req = Request::try_from(req).expect("not found some element in request extensions");

            async move { handler.call(req).await }.boxed()
        }
    }

    pub struct ServiceToHandler<S>(pub Arc<Mutex<S>>);

    impl<S> Handler for ServiceToHandler<S>
    where
        S: Service<http::Request<Incoming>> + Send + Sync + 'static,
        S::Response: IntoResponse,
        S::Error: Into<Error>,
        S::Future: Send,
    {
        async fn call(&self, req: Request) -> Result<Response, Error> {
            let svc = self.0.clone();

            poll_fn(|cx| svc.lock().unwrap().poll_ready(cx))
                .await
                .map_err(Into::into)?;

            let fut = svc
                .lock()
                .unwrap()
                .call(http::Request::<Incoming>::from(req));

            Ok(fut.await.map_err(Into::into)?.into_response()?)
        }
    }
}
