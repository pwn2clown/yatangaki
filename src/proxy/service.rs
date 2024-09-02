use super::{ProxyCommand, ProxyEvent, ProxyId, ProxyState};
use crate::db::logs;
use crate::proxy::certificates::CertificateStore;
use http::uri::{Authority, Parts, Scheme};
use http_body_util::{BodyExt, Full};
use hyper::body::{Bytes, Incoming};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode, Uri};
use hyper_util::rt::TokioIo;
use iced::futures::{channel::mpsc, SinkExt};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

#[derive(Clone)]
pub struct ProxyServiceConfig {
    certificates: CertificateStore,
}

impl ProxyServiceConfig {
    pub fn from(store: CertificateStore) -> Self {
        Self {
            certificates: store.clone(),
        }
    }
}

#[derive(Clone)]
struct Service {
    proxy_id: ProxyId,
    config: ProxyServiceConfig,
}

pub async fn serve(
    proxy_id: ProxyId,
    port: u16,
    mut sender: mpsc::Sender<ProxyEvent>,
    config: ProxyServiceConfig,
) {
    let mut state = ProxyState::Stopped;
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let mut maybe_shutdown: Option<oneshot::Sender<()>> = None;

    let (command_tx, mut command) = mpsc::channel::<ProxyCommand>(100);
    sender
        .send(ProxyEvent::Initialized((proxy_id, command_tx)))
        .await
        .unwrap();

    loop {
        if let Ok(Some(cmd)) = command.try_next() {
            match cmd {
                ProxyCommand::Stop => {
                    if let Some(shutdown) = maybe_shutdown.take() {
                        shutdown.send(()).unwrap();
                        state = ProxyState::Stopped;
                    }
                }
                ProxyCommand::Start => {
                    if state == ProxyState::Stopped {
                        match TcpListener::bind(addr).await {
                            Ok(l) => {
                                let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
                                let _ = maybe_shutdown.insert(shutdown_tx);
                                state = ProxyState::Running;
                                let s = Service {
                                    proxy_id,
                                    config: config.clone(),
                                };
                                tokio::spawn(service(shutdown_rx, sender.clone(), s, l));
                            }
                            Err(_err) => {
                                sender.send(ProxyEvent::ProxyError(proxy_id)).await.unwrap();
                            }
                        }
                    }
                }
            }
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

async fn service(
    shutdown: oneshot::Receiver<()>,
    sender: mpsc::Sender<ProxyEvent>,
    service: Service,
    listener: TcpListener,
) {
    let sender_cloned = sender.clone();
    tokio::select! {
        _ = shutdown => {
            println!("shutting down proxy service");
        }
        _ = async move {

            loop {
                let (stream, _socket_addr) = listener.accept().await.unwrap();

                match http1::Builder::new()
                    .serve_connection(TokioIo::new(stream), service_fn(|req| proxify_request(req, sender_cloned.clone(), service.clone())))
                    .with_upgrades()
                    .await
                {
                    Ok(_o) => println!("connection accepted"),
                    Err(err) => println!("http service error: {err:#?}"),
                }
        }} => {}
    }
}

async fn proxify_request(
    mut req: Request<hyper::body::Incoming>,
    sender: mpsc::Sender<ProxyEvent>,
    mut service: Service,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    if req.uri().scheme_str() == Some("https") {
        let mut res = Response::default();
        *res.status_mut() = StatusCode::BAD_REQUEST;
        return Ok(res);
    }

    let authority = req.uri().authority();

    if authority.is_none() {
        let mut res = Response::default();
        *res.status_mut() = StatusCode::BAD_REQUEST;
        return Ok(res);
    };

    let authority = authority.unwrap().to_owned();
    let host = authority.host().to_owned();

    if *req.method() == Method::CONNECT {
        tokio::spawn(async move {
            match hyper::upgrade::on(&mut req).await {
                Ok(to_upgrade) => {
                    let acceptor = service.config.certificates.tls_acceptor(&host).unwrap();

                    //  Acceptor will generate an error if the client rejects the certificate
                    if let Ok(stream) = acceptor.accept(TokioIo::new(to_upgrade)).await {
                        let _ = http1::Builder::new()
                            .serve_connection(
                                TokioIo::new(stream),
                                service_fn(move |req| {
                                    forward_packet(
                                        req,
                                        sender.clone(),
                                        Scheme::HTTPS,
                                        authority.clone(),
                                        service.proxy_id,
                                    )
                                }),
                            )
                            .with_upgrades()
                            .await;
                    };
                }
                Err(_err) => {
                    println!("failed to upgrade protocol");
                }
            }
        });

        return Ok(Response::default());
    }

    forward_packet(req, sender, Scheme::HTTP, authority, service.proxy_id).await
}

async fn forward_packet(
    req: Request<Incoming>,
    mut sender: mpsc::Sender<ProxyEvent>,
    scheme: Scheme,
    authority: Authority,
    proxy_id: usize,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    //  Building full uri for client
    let mut uri_parts = Parts::default();
    uri_parts.path_and_query = req.uri().path_and_query().cloned();
    uri_parts.scheme = Some(scheme);
    uri_parts.authority = Some(authority);

    let full_uri = Uri::from_parts(uri_parts).unwrap();

    //  Build reqwest compatible request
    let (parts, body_stream) = req.into_parts();
    let full_body = body_stream.collect().await.unwrap().to_bytes();
    let mut full_req = http::request::Request::from_parts(parts, full_body);
    *full_req.uri_mut() = full_uri;

    let maybe_packet_id = logs::insert_request(&full_req, proxy_id);

    let _ = sender.send(ProxyEvent::NewRequestLogRow).await;

    //  Note: Host header MUST be removed as reqwest will set it itself. Keeping it will lead to
    //  protocol errors with HTTP/2.
    full_req.headers_mut().remove(http::header::HOST);
    let reqwest_req = reqwest::Request::try_from(full_req).unwrap();

    let http_client = reqwest::Client::new();
    let Ok(response) = http_client.execute(reqwest_req).await else {
        let mut res = Response::default();
        *res.status_mut() = StatusCode::SERVICE_UNAVAILABLE;
        return Ok(res);
    };

    let mut hyper_response = Response::default();

    *hyper_response.status_mut() = response.status();
    *hyper_response.headers_mut() = response.headers().clone();
    *hyper_response.version_mut() = response.version();
    *hyper_response.extensions_mut() = response.extensions().clone();

    let body_bytes = response.bytes().await.unwrap();
    *hyper_response.body_mut() = Full::new(body_bytes.clone());

    match maybe_packet_id {
        Ok(packet_id) => {
            let _ = logs::insert_response(&hyper_response, body_bytes, packet_id);
        }
        Err(e) => {
            println!("failed to save request: {e:#?}");
        }
    }

    let _ = sender.send(ProxyEvent::NewResponseLogRow).await;

    Ok(hyper_response)
}
