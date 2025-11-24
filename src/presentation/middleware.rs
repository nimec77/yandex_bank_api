use actix_web::{
    Error, HttpMessage,
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    http::header::{HeaderName, HeaderValue},
};

use std::{
    future::{Ready, ready},
    pin::Pin,
    rc::Rc,
    task::{Context, Poll},
    time::Instant,
};
use tracing::{debug, info, trace};
use uuid::Uuid;

// Request ID Middleware
pub struct RequestIdMiddleware;

impl<S, B> Transform<S, ServiceRequest> for RequestIdMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = RequestIdMiddlewareService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RequestIdMiddlewareService {
            service: Rc::new(service),
        }))
    }
}

pub struct RequestIdMiddlewareService<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for RequestIdMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();
        let request_id = Uuid::new_v4().to_string();
        
        let span = tracing::span!(
            tracing::Level::TRACE,
            "request_id_middleware",
            request_id = %request_id
        );
        let _guard = span.enter();

        trace!("Generating request ID");
        debug!(request_id = %request_id, "Processing request with ID");

        // Add request ID to request extensions
        req.extensions_mut().insert(request_id.clone());

        // Call the service
        let fut = service.call(req);

        Box::pin(async move {
            let mut res = fut.await?;

            // Add request ID to response headers
            res.headers_mut().insert(
                HeaderName::from_static("x-request-id"),
                HeaderValue::from_str(&request_id)
                    .unwrap_or_else(|_| HeaderValue::from_static("unknown")),
            );

            trace!(request_id = %request_id, "Request ID added to response headers");
            Ok(res)
        })
    }
}

// Timing Middleware
pub struct TimingMiddleware;

impl<S, B> Transform<S, ServiceRequest> for TimingMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = TimingMiddlewareService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(TimingMiddlewareService {
            service: Rc::new(service),
        }))
    }
}

pub struct TimingMiddlewareService<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for TimingMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();
        let start = Instant::now();
        let method = req.method().clone();
        let path = req.path().to_string();
        
        // Get request ID from extensions if available (before moving req)
        let request_id = req
            .extensions()
            .get::<String>()
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());
        
        let span = tracing::span!(
            tracing::Level::TRACE,
            "timing_middleware",
            method = %method,
            path = %path,
            request_id = %request_id
        );
        let _guard = span.enter();

        trace!("Starting request timing");

        debug!(
            method = %method,
            path = %path,
            request_id = %request_id,
            "Processing request"
        );

        let fut = service.call(req);

        Box::pin(async move {
            let res = fut.await?;
            let duration = start.elapsed();
            let duration_ms = duration.as_millis();
            
            tracing::Span::current().record("duration_ms", duration_ms as u64);

            // Add timing header
            let mut res = res;
            res.headers_mut().insert(
                HeaderName::from_static("x-response-time"),
                HeaderValue::from_str(&format!("{}ms", duration_ms))
                    .unwrap_or_else(|_| HeaderValue::from_static("0ms")),
            );

            // Log timing information
            info!(
                method = %method,
                path = %path,
                duration_ms = duration_ms,
                request_id = %request_id,
                "Request processed"
            );

            Ok(res)
        })
    }
}
