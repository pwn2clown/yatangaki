use crate::proxy::ProxyState;
use crate::proxy::{self, ProxyCommand, ProxyEvent, ProxyId};
use crate::Message;
use iced::advanced::graphics::futures::subscription;
use iced::futures::channel::mpsc;
use iced::futures::SinkExt;
use iced::widget::Scrollable;
use iced::widget::{Button, Column, Container, Row, Text, TextInput};
use iced::{Command, Element, Length, Subscription};
use std::collections::HashMap;

pub struct Proxy {
    id: ProxyId,
    port: u16,
    status: ProxyState,
    command: Option<mpsc::Sender<ProxyCommand>>,
}

//  TODO: rename 'SettingsTabs'
pub struct SettingsTabs {
    is_port_format_error: bool,
    proxy_port_request: String,
    proxies: HashMap<ProxyId, Proxy>,
    selected_proxy: Option<ProxyId>,
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
                    ProxyState::Running => {
                        let button: Button<'_, SettingsMessage> =
                            Button::new("stop").on_press(SettingsMessage::StopProxy(id));

                        config = config.push(button);
                    }
                    ProxyState::Stopped => {
                        let button: Button<'_, SettingsMessage> =
                            Button::new("start").on_press(SettingsMessage::StartProxy(id));

                        config = config.push(button);
                    }
                    ProxyState::Error => {
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

    pub fn subscription(&self) -> Subscription<Message> {
        let mut subscriptions = vec![];
        for (_id, proxy) in &self.proxies {
            let proxy_id = proxy.id;
            let port = proxy.port;

            subscriptions.push(subscription::channel(
                proxy_id,
                100,
                move |sender: mpsc::Sender<ProxyEvent>| proxy::serve(proxy_id, port, sender),
            ));
        }

        Subscription::batch(subscriptions).map(Message::ProxyEvent)
    }

    pub fn update(&mut self, message: SettingsMessage) -> Command<SettingsMessage> {
        match message {
            //  Create the proxy task in a pending state, might start automatically in future
            //  versions.
            SettingsMessage::AddProxy => match self.proxy_port_request.parse::<u16>() {
                Ok(port) => {
                    self.proxy_port_request = String::default();
                    let id = self.proxies.len();
                    let proxy = Proxy {
                        id: self.proxies.len(),
                        port,
                        status: ProxyState::Stopped,
                        command: None,
                    };
                    self.proxies.insert(id, proxy);
                    self.is_port_format_error = false;
                }
                Err(_err) => {
                    self.is_port_format_error = true;
                }
            },
            SettingsMessage::SelectProxy(proxy_id) => {
                let _ = self.selected_proxy.insert(proxy_id);
            }
            SettingsMessage::ProxyPortRequest(port) => {
                self.is_port_format_error = port.parse::<u16>().is_err() && !port.is_empty();
                self.proxy_port_request = port;
            }
            SettingsMessage::StartProxy(id) => {
                let proxy = self.proxies.get_mut(&id).unwrap();
                proxy.status = ProxyState::Running;
                let command = proxy.command.clone();

                return Command::perform(
                    async move {
                        if let Some(mut cmd) = command {
                            cmd.send(ProxyCommand::Start).await.unwrap();
                        }
                    },
                    |_| SettingsMessage::Update,
                );
            }
            SettingsMessage::StopProxy(id) => {
                let proxy = self.proxies.get_mut(&id).unwrap();
                let command = proxy.command.clone();

                proxy.status = ProxyState::Stopped;
                return Command::perform(
                    async move {
                        if let Some(mut cmd) = command {
                            cmd.send(ProxyCommand::Stop).await.unwrap();
                        }
                    },
                    |_| SettingsMessage::Update,
                );
            }
            SettingsMessage::ProxyEvent(event) => match event {
                ProxyEvent::NewLogRow(_row) => {}
                ProxyEvent::ProxyError(id) => {
                    let proxy = self.proxies.get_mut(&id).unwrap();
                    proxy.status = ProxyState::Error;
                }
                ProxyEvent::Initialized((id, command_tx)) => {
                    let proxy = self.proxies.get_mut(&id).unwrap();
                    proxy.status = ProxyState::Stopped;
                    let _ = proxy.command.insert(command_tx);
                }
            },
            SettingsMessage::Update => {}
        }

        Command::none()
    }
}
