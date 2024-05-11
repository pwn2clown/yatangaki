use http_body_util::Full;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use iced::futures::channel::mpsc;
use iced::futures::SinkExt;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

pub type ProxyId = usize;

#[derive(Debug, Clone)]
pub struct ProxyLogRow {
    pub proxy_id: ProxyId,
    pub url: String,
}

#[derive(Debug, Clone)]
pub enum ProxyEvent {
    Initialized((ProxyId, mpsc::Sender<ProxyCommand>)), //  Create ProxyHandle type for this enum
    ProxyError(ProxyId),
    NewLogRow(ProxyLogRow),
}

#[derive(Debug, Clone)]
pub enum ProxyCommand {
    Stop,
    Start,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ProxyState {
    Running,
    Stopped,
    Error,
}

struct Service {
    id: ProxyId,
    listener: TcpListener,
    //_config: Arc
}

pub async fn serve(id: ProxyId, port: u16, mut sender: mpsc::Sender<ProxyEvent>) -> Infallible {
    let mut state = ProxyState::Stopped;
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let mut maybe_shutdown: Option<oneshot::Sender<()>> = None;

    let (command_tx, mut command) = mpsc::channel::<ProxyCommand>(100);
    sender
        .send(ProxyEvent::Initialized((id, command_tx)))
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
                                let s = Service { id, listener: l };
                                tokio::spawn(service(shutdown_rx, sender.clone(), s));
                            }
                            Err(_err) => {
                                sender.send(ProxyEvent::ProxyError(id)).await.unwrap();
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
) {
    let sender_cloned = sender.clone();
    tokio::select! {
        _ = shutdown => {
            println!("shutting down proxy service");
        }
        _ = async move {

            loop {
                let (stream, _socket_addr) = service.listener.accept().await.unwrap();
                let io = TokioIo::new(stream);
                let s = service_fn(|req| proxify_request(req, service.id, sender_cloned.clone()));

                match http1::Builder::new()
                    .serve_connection(io, s)
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
    req: Request<hyper::body::Incoming>,
    proxy_id: ProxyId,
    mut sender: mpsc::Sender<ProxyEvent>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    sender
        .send(ProxyEvent::NewLogRow(ProxyLogRow {
            proxy_id,
            url: req.uri().to_string(),
        }))
        .await
        .unwrap();
    Ok(Response::default())
}
