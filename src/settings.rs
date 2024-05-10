use crate::proxy::{self, ProxyCommand, ProxyEvent, ProxyId};
use crate::Message;
use iced::futures::channel::mpsc;
use iced::futures::SinkExt;
use iced::widget::Scrollable;
use iced::widget::{Button, Column, Container, Row, Text, TextInput};
use iced::{command, Command, Element, Length};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub enum ProxyStatus {
    Started,
    Stopped,
    Error,
}

pub struct Proxy {
    id: ProxyId,
    port: u16,
    status: ProxyStatus,
    command: mpsc::Sender<ProxyCommand>,
}

pub struct SettingsTabs {
    is_port_format_error: bool,
    proxy_port_request: String,
    proxies: HashMap<ProxyId, Proxy>,
    selected_proxy: Option<ProxyId>,
    packet_id: Arc<Mutex<usize>>,
}

#[derive(Debug, Clone)]
pub enum SettingsMessage {
    AddProxy,
    SelectProxy(ProxyId),
    ProxyPortRequest(String),
    StartProxy(ProxyId),
    StopProxy(ProxyId),
    ProxyEvent(ProxyEvent),
    Update,
}

impl SettingsTabs {
    pub fn new() -> Self {
        Self {
            is_port_format_error: false,
            proxy_port_request: String::default(),
            proxies: HashMap::default(),
            selected_proxy: None,
            packet_id: Arc::new(Mutex::new(0)),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let mut proxy_table = Column::new().width(200.0);
        let header_row = Row::new()
            .push(Text::new("id").width(Length::Fixed(50.0)))
            .push(Text::new("port").width(Length::Fixed(150.0)));

        proxy_table = proxy_table.push(header_row);

        let mut proxy_list = Column::new();
        for (id, proxy) in &self.proxies {
            let row = Row::new()
                .push(
                    Text::new(id.to_string())
                        .width(Length::Fixed(50.0))
                        .size(12.0),
                )
                .push(
                    Text::new(proxy.port.to_string())
                        .width(Length::Fixed(150.0))
                        .size(12.0),
                )
                .height(Length::Fixed(16.0));

            let button = Button::new(row).on_press(SettingsMessage::SelectProxy(proxy.id));
            proxy_list = proxy_list.push(button);
        }

        proxy_table =
            proxy_table.push(Scrollable::new(proxy_list).height(Length::Fixed(16.0 * 5.0)));

        let proxy_port_field = TextInput::new("enter proxy port", &self.proxy_port_request)
            .on_input(SettingsMessage::ProxyPortRequest)
            .width(Length::Fixed(150.0));

        let submit_proxy: Button<'_, SettingsMessage> =
            Button::new("add").on_press(SettingsMessage::AddProxy);

        proxy_table = proxy_table.push(Row::new().push(proxy_port_field).push(submit_proxy));

        if self.is_port_format_error {
            proxy_table = proxy_table.push(Text::new("error: bad port format"));
        }

        let mut proxy_settings = Row::new().push(proxy_table).spacing(30);

        if let Some(id) = self.selected_proxy {
            let mut config = Column::new().push(Text::new(format!("selected proxy with id {id}")));

            if let Some(proxy) = self.proxies.get(&id) {
                match proxy.status {
                    ProxyStatus::Started => {
                        let button: Button<'_, SettingsMessage> =
                            Button::new("stop").on_press(SettingsMessage::StopProxy(id));

                        config = config.push(button);
                    }
                    ProxyStatus::Stopped => {
                        let button: Button<'_, SettingsMessage> =
                            Button::new("start").on_press(SettingsMessage::StartProxy(id));

                        config = config.push(button);
                    }
                    ProxyStatus::Error => {
                        let button: Button<'_, SettingsMessage> =
                            Button::new("start").on_press(SettingsMessage::StartProxy(id));

                        config = config.push(button);
                        config = config.push(Text::new("an error occured :c"));
                    }
                }
            }

            proxy_settings = proxy_settings.push(config);
        } else {
            proxy_settings = proxy_settings.push(Text::new("no proxy selected"));
        };

        let content: Element<'_, SettingsMessage> =
            Container::new(proxy_settings).padding(20.0).into();
        content.map(Message::SettingsMessage)
    }

    pub fn update(&mut self, message: SettingsMessage) -> Command<SettingsMessage> {
        match message {
            //  Create the proxy task in a pending state, might start automatically later on.
            SettingsMessage::AddProxy => {
                let (proxy_command_tx, proxy_command_rx) = mpsc::channel::<ProxyCommand>(100);
                match self.proxy_port_request.parse::<u16>() {
                    Ok(port) => {
                        self.proxy_port_request = String::default();
                        let id = self.proxies.len();
                        let proxy = Proxy {
                            id: self.proxies.len(),
                            port,
                            status: ProxyStatus::Stopped,
                            command: proxy_command_tx, //  Consider not using options here.
                        };
                        self.proxies.insert(id, proxy);
                        self.is_port_format_error = false;

                        return command::channel(
                            100,
                            move |sender: mpsc::Sender<SettingsMessage>| async move {
                                proxy::serve(id, port, proxy_command_rx, sender).await
                            },
                        );
                    }
                    Err(_err) => {
                        self.is_port_format_error = true;
                        return Command::none();
                    }
                }
            }
            SettingsMessage::SelectProxy(proxy_id) => {
                let _ = self.selected_proxy.insert(proxy_id);
            }
            SettingsMessage::ProxyPortRequest(port) => {
                if port.parse::<u16>().is_err() && !port.is_empty() {
                    self.is_port_format_error = true;
                } else {
                    self.is_port_format_error = false;
                }

                self.proxy_port_request = port;
            }
            SettingsMessage::StartProxy(id) => {
                let proxy = self.proxies.get_mut(&id).unwrap();
                proxy.status = ProxyStatus::Started;
                let mut command = proxy.command.clone();
                return Command::perform(
                    async move { command.send(ProxyCommand::Start).await },
                    |_| SettingsMessage::Update,
                );
            }
            SettingsMessage::StopProxy(id) => {
                let proxy = self.proxies.get_mut(&id).unwrap();
                let mut command = proxy.command.clone();

                proxy.status = ProxyStatus::Stopped;
                return Command::perform(
                    async move { command.send(ProxyCommand::Stop).await },
                    |_| SettingsMessage::Update,
                );
            }
            SettingsMessage::ProxyEvent(event) => match event {
                ProxyEvent::NewLogRow(id) => println!("recived event from proxy {id}"),
                ProxyEvent::ProxyError(id) => {
                    let proxy = self.proxies.get_mut(&id).unwrap();
                    proxy.status = ProxyStatus::Error;
                }
            },
            SettingsMessage::Update => {}
        }

        Command::none()
    }
}
