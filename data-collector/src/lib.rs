use actix_web::dev::{
    MessageBody, ResponseBody, Service, ServiceRequest, ServiceResponse, Transform,
};
use actix_web::error::{Error, Result};
use actix_web::http::header::{HOST, USER_AGENT};
use chrono::{SecondsFormat, Utc};
use futures::future::{ok, Ready};
use futures::stream::StreamExt;
use kiln_lib::validation::ValidationError;
use serde::{Deserialize, Serialize};
use slog::{error, info, o, Logger};
use slog_derive::SerdeValue;
use std::borrow::ToOwned;
use std::cell::RefCell;
use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::str::FromStr;
use uuid::Uuid;

#[derive(Clone, SerdeValue, Serialize, Deserialize)]
struct EventType(Vec<String>);

pub struct StructuredLogger(Rc<Inner>);

struct Inner {
    logger: Logger,
    exclude: HashSet<String>,
}

impl StructuredLogger {
    #[must_use]
    pub fn new(logger: Logger) -> StructuredLogger {
        StructuredLogger(Rc::new(Inner {
            logger,
            exclude: HashSet::new(),
        }))
    }

    pub fn exclude<T: Into<String>>(mut self, path: T) -> Self {
        Rc::get_mut(&mut self.0)
            .unwrap()
            .exclude
            .insert(path.into());
        self
    }
}

impl<S, B> Transform<S, ServiceRequest> for StructuredLogger
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: MessageBody + 'static + Unpin,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = StructuredLoggerMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(StructuredLoggerMiddleware {
            service: Rc::new(RefCell::new(service)),
            inner: self.0.clone(),
        })
    }
}

pub struct StructuredLoggerMiddleware<S> {
    inner: Rc<Inner>,
    service: Rc<RefCell<S>>,
}

impl<S, B> Service<ServiceRequest> for StructuredLoggerMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: MessageBody + 'static + Unpin,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    actix_service::forward_ready!(service);

    fn call(&self, mut req: ServiceRequest) -> Self::Future {
        let event_start = Utc::now();
        let svc = self.service.clone();
        let is_exclude = self.inner.exclude.contains(req.path());
        let logger = self.inner.clone().logger.clone();

        Box::pin(async move {
            // check the exclude-list if to skip this path…

            // …but collect other fields nevertheless, to log errors etc.
            let user_agent = req
                .headers()
                .get(USER_AGENT)
                .and_then(|v| v.to_str().ok())
                .map(|v| v.to_string());

            let source_ip = req
                .connection_info()
                .realip_remote_addr()
                .and_then(|conn_info| std::net::SocketAddr::from_str(conn_info).ok())
                .map(|socket_addr| socket_addr.ip().to_string());

            let url_domain = req
                .headers()
                .get(HOST)
                .and_then(|v| v.to_str().ok())
                .map(|v| v.to_string());

            let http_version = format!("{:?}", req.version());
            let http_request_method = req.path().to_owned();
            let url_path = req.path().to_owned();
            let url_query = req.query_string().to_string();

            let transaction_id = Uuid::new_v4();
            let event_id = Uuid::new_v4();

            req.head_mut().extensions_mut().insert(transaction_id);

            // read request body
            let (req_http, mut payload) = req.into_parts();

            let mut req_body_bytes_mut = bytes::BytesMut::new();
            while let Some(chunk) = payload.by_ref().next().await {
                req_body_bytes_mut.extend_from_slice(&chunk?);
            }
            let req_body_bytes = req_body_bytes_mut.freeze();
            let req_body = String::from_utf8(req_body_bytes.clone().to_vec()).unwrap();

            req = ServiceRequest::from_parts(req_http, payload);

            let resp = svc.call(req).await;

            match resp {
                Err(err) => {
                    println!("Err!");
                    let validation_err: Option<&ValidationError> = err.as_error();
                    let error_code = validation_err.map_or(0, |v| v.error_code);
                    let error_message = validation_err
                        .map_or_else(|| err.to_string(), |v| v.error_message.to_string());

                    let event_end = Utc::now();
                    error!(logger, "Error processing Tool Report";
                        o!(
                            "http.version" => http_version,
                            "url.domain" => url_domain,
                            "source.ip" => source_ip,
                            "user_agent.original" => user_agent,
                            "http.request.method" => http_request_method,
                            "http.request.body.bytes" => req_body_bytes.len(),
                            "http.request.body.content" => req_body,
                            "url.path" => url_path,
                            "url.query" => format!("?{}", url_query),
                            "event.start" => event_start.to_rfc3339_opts(SecondsFormat::Secs, true),
                            "event.end" => event_end.to_rfc3339_opts(SecondsFormat::Secs, true),
                            "event.duration" => event_end.signed_duration_since(event_start).num_nanoseconds(),
                            "event.type" => EventType(vec!("access".to_string(), "error".to_string())),
                            "error.code" => error_code,
                            "error.message" => error_message,
                            "transaction.id" => transaction_id.to_hyphenated().to_string(),
                            "event.id" => event_id.to_hyphenated().to_string(),
                        )
                    );

                    Err(err)
                }
                Ok(mut resp) => {
                    if !is_exclude {
                        // read response body
                        let mut stream = resp.take_body();

                        let mut resp_body_bytes_mut = bytes::BytesMut::new();
                        while let Some(chunk) = stream.next().await {
                            resp_body_bytes_mut.extend_from_slice(&chunk?);
                        }
                        let resp_body_bytes = resp_body_bytes_mut.freeze();
                        let resp_body_bytes_len = resp_body_bytes.len();
                        let resp_body =
                            String::from_utf8(resp_body_bytes.clone().to_vec()).unwrap();

                        // put bytes back into response body
                        let resp: Self::Response = resp.map_body(move |_, _| {
                            ResponseBody::Other(actix_web::dev::Body::from_slice(&resp_body_bytes))
                        });
                        let event_end = Utc::now();
                        if let Some(err) = resp.response().error() {
                            let validation_err: Option<&ValidationError> = err.as_error();
                            let error_code = validation_err.map_or(0, |v| v.error_code);
                            let error_message = validation_err
                                .map_or_else(|| err.to_string(), |v| v.error_message.to_string());

                            let event_end = Utc::now();
                            error!(logger, "Error processing Tool report";
                                o!(
                                    "http.version" => http_version,
                                    "url.domain" => url_domain,
                                    "source.ip" => source_ip,
                                    "user_agent.original" => user_agent,
                                    "http.request.method" => http_request_method,
                                    "http.request.body.bytes" => req_body_bytes.len(),
                                    "http.request.body.content" => req_body,
                                    "http.response.body.bytes" => resp_body_bytes_len,
                                    "http.response.body.content" => resp_body,
                                    "http.response.status_code" => resp.status().as_str(),
                                    "url.path" => url_path,
                                    "url.query" => format!("?{}", url_query),
                                    "event.start" => event_start.to_rfc3339_opts(SecondsFormat::Secs, true),
                                    "event.end" => event_end.to_rfc3339_opts(SecondsFormat::Secs, true),
                                    "event.duration" => event_end.signed_duration_since(event_start).num_nanoseconds(),
                                    "event.type" => EventType(vec!("access".to_string(), "error".to_string())),
                                    "error.code" => error_code,
                                    "error.message" => error_message,
                                    "transaction.id" => transaction_id.to_hyphenated().to_string(),
                                    "event.id" => event_id.to_hyphenated().to_string(),
                                )
                            );
                            return Ok(resp);
                        } else {
                            info!(logger, "Tool report received";
                                o!(
                                    "http.version" => http_version,
                                    "url.domain" => url_domain,
                                    "source.ip" => source_ip,
                                    "user_agent.original" => user_agent,
                                    "http.request.method" => http_request_method,
                                    "http.request.body.bytes" => req_body_bytes.len(),
                                    "http.request.body.content" => req_body,
                                    "http.response.body.bytes" => resp_body_bytes_len,
                                    "http.response.body.content" => resp_body,
                                    "http.response.status_code" => resp.status().as_str(),
                                    "url.path" => url_path,
                                    "url.query" => format!("?{}", url_query),
                                    "event.start" => event_start.to_rfc3339_opts(SecondsFormat::Secs, true),
                                    "event.end" => event_end.to_rfc3339_opts(SecondsFormat::Secs, true),
                                    "event.duration" => event_end.signed_duration_since(event_start).num_nanoseconds(),
                                    "event.type" => EventType(vec!("access".to_string())),
                                    "transaction.id" => transaction_id.to_hyphenated().to_string(),
                                    "event.id" => event_id.to_hyphenated().to_string(),
                                )
                            );
                            return Ok(resp);
                        }
                    } else {
                        Ok(resp)
                    }
                }
            }
        })
    }
}
