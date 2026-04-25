use axum::{extract::Request, http::HeaderValue, middleware::Next, response::Response};
use ennoia_observability::{
    RequestContext, REQUEST_ID_HEADER, SPAN_ID_HEADER, TRACEPARENT_HEADER, TRACE_ID_HEADER,
};

pub async fn request_context_middleware(mut req: Request, next: Next) -> Response {
    let context = RequestContext::from_headers(req.headers());
    req.extensions_mut().insert(context.clone());

    let mut response = next.run(req).await;
    if let Ok(value) = HeaderValue::from_str(&context.request_id) {
        response.headers_mut().insert(REQUEST_ID_HEADER, value);
    }
    if let Ok(value) = HeaderValue::from_str(&context.trace_id) {
        response.headers_mut().insert(TRACE_ID_HEADER, value);
    }
    if let Ok(value) = HeaderValue::from_str(&context.span_id) {
        response.headers_mut().insert(SPAN_ID_HEADER, value);
    }
    if let Ok(value) = HeaderValue::from_str(&context.trace_context().to_traceparent()) {
        response.headers_mut().insert(TRACEPARENT_HEADER, value);
    }
    response
}
