pub mod certificates;
pub mod service;

use iced::futures::channel::mpsc;

pub type ProxyId = usize;
pub type PacketId = usize;

#[derive(Debug, Clone)]
pub enum ProxyEvent {
    Initialized((ProxyId, mpsc::Sender<ProxyCommand>)), //  Create ProxyHandle type for this enum
    ProxyError(ProxyId),
    NewRequestLogRow,
    NewResponseLogRow,
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
