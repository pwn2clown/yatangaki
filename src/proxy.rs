use crate::settings::SettingsMessage;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use iced::futures::channel::mpsc;
use iced::futures::SinkExt;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

pub type ProxyId = usize;

pub struct ProxyLogRow {
    proxy_id: ProxyId,
    url: String,
}

#[derive(Debug, Clone)]
pub enum ProxyEvent {
    ProxyError(ProxyId),
    NewLogRow(ProxyId),
}

#[derive(Debug, Clone)]
pub enum ProxyCommand {
    Stop,
    Start,
    Intercept,
}

#[derive(Debug, PartialEq, Eq)]
enum ProxyState {
    Running,
    Stopped,
}

struct Service {
    id: ProxyId,
    listener: TcpListener,
    //_config: Arc
}

//  FIXME: proxy should not manipulate SettingsMessage and should only notify with ProxyEvent type.
pub async fn serve(
    id: ProxyId,
    port: u16,
    mut command: mpsc::Receiver<ProxyCommand>,
    mut sender: mpsc::Sender<SettingsMessage>,
) {
    let mut state = ProxyState::Stopped;
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let mut maybe_shutdown: Option<oneshot::Sender<()>> = None;

    loop {
        if let Ok(Some(cmd)) = command.try_next() {
            println!("received command on proxy {id} -> {cmd:#?} on state {state:#?}");

            match cmd {
                ProxyCommand::Stop => {
                    println!("stopping proxy {id}");
                    if let Some(shutdown) = maybe_shutdown.take() {
                        shutdown.send(()).unwrap();
                        state = ProxyState::Stopped;
                    }
                }
                ProxyCommand::Start => {
                    if state == ProxyState::Stopped {
                        println!("binding port...");
                        match TcpListener::bind(addr).await {
                            Ok(l) => {
                                let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
                                let _ = maybe_shutdown.insert(shutdown_tx);
                                state = ProxyState::Running;
                                println!("proxy port bound!");

                                let s = Service { id: 0, listener: l };
                                tokio::spawn(service(shutdown_rx, sender.clone(), s));
                            }
                            Err(_err) => {
                                sender
                                    .send(SettingsMessage::ProxyEvent(ProxyEvent::ProxyError(id)))
                                    .await
                                    .unwrap();
                            }
                        }
                    }
                }
                ProxyCommand::Intercept => {}
            }
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

async fn service(
    shutdown: oneshot::Receiver<()>,
    sender: mpsc::Sender<SettingsMessage>,
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
                println!("connection accepted");


                let io = TokioIo::new(stream);
                let s = service_fn(|req| proxify_request(req, sender_cloned.clone()));

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
    sender: mpsc::Sender<SettingsMessage>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    Ok(Response::default())
}
