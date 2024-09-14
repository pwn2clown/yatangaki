pub mod service;
pub mod tls;

pub mod types {
    use iced::futures::channel::mpsc;

    pub type ProxyId = usize;
    pub type PacketId = usize;

    #[derive(Debug, Clone)]
    pub enum ProxyEvent {
        Initialized((ProxyId, mpsc::Sender<ProxyCommand>)), //  Create ProxyHandle type for this enum
        ProxyError(ProxyId),
        NewHttpLogRow,
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
}
