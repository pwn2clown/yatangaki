use crate::db::{
    config::{self, ProxyConfig},
    logs,
};
use crate::proxy::certificates::CertificateStore;
use crate::proxy::{
    self, service::ProxyServiceConfig, ProxyCommand, ProxyEvent, ProxyId, ProxyState,
};
use crate::Message;
use iced::futures::{channel::mpsc, SinkExt};
use iced::widget::{button, Button, Column, Container, Row, Scrollable, Text, TextInput};
use iced::{Element, Length, Subscription, Task};
use std::collections::HashMap;

pub struct Proxy {
    auto_start: bool,
    id: ProxyId,
    port: u16,
    status: ProxyState,
    command: Option<mpsc::Sender<ProxyCommand>>,
    config: ProxyServiceConfig,
}

pub struct SettingsTabs {
    is_port_format_error: bool,
    proxy_port_request: String,
    proxies: HashMap<ProxyId, Proxy>,
    selected_proxy: Option<ProxyId>,
    certificate_store: CertificateStore,
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
    pub fn new(certificate_store: CertificateStore) -> Self {
        if let Err(e) = logs::create_project_db("test") {
            println!("failed to create database {e:#?}");
        }

        let mut settings = Self {
            is_port_format_error: false,
            proxy_port_request: String::default(),
            proxies: HashMap::default(),
            selected_proxy: None,
            certificate_store,
        };

        config::init_config().unwrap();
        for proxy_config in &config::load_proxies().unwrap() {
            settings.add_proxy(proxy_config);
        }

        settings
    }

    fn add_proxy(&mut self, proxy_config: &ProxyConfig) {
        self.proxies.insert(
            proxy_config.id,
            Proxy {
                auto_start: true,
                id: proxy_config.id,
                port: proxy_config.port,
                status: ProxyState::Stopped,
                command: None,
                config: ProxyServiceConfig::from(self.certificate_store.clone()),
            },
        );
    }

    pub fn proxy_settings_view(&self) -> Row<'_, SettingsMessage> {
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

            let mut button = Button::new(row).on_press(SettingsMessage::SelectProxy(proxy.id));
            if self.selected_proxy != Some(*id) {
                button = button.style(button::secondary);
            }

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
            let mut config = Column::new();

            if let Some(proxy) = self.proxies.get(&id) {
                let button: Button<'_, SettingsMessage> = match proxy.status {
                    ProxyState::Running => {
                        Button::new("stop").on_press(SettingsMessage::StopProxy(id))
                    }
                    ProxyState::Stopped | ProxyState::Error => {
                        Button::new("start").on_press(SettingsMessage::StartProxy(id))
                    }
                };

                config = config.push(button);
                if proxy.status == ProxyState::Error {
                    config = config.push(Text::new("an error occured"));
                }
            }

            proxy_settings = proxy_settings.push(config);
        } else {
            proxy_settings = proxy_settings.push(Text::new("no proxy selected"));
        };

        proxy_settings
    }

    fn start_proxy_cmd(&mut self, id: ProxyId) -> Task<SettingsMessage> {
        let proxy = self.proxies.get_mut(&id).unwrap();
        proxy.status = ProxyState::Running;
        let command = proxy.command.clone();

        Task::perform(
            async move {
                if let Some(mut cmd) = command {
                    cmd.send(ProxyCommand::Start).await.unwrap();
                }
            },
            |_| SettingsMessage::Update,
        )
    }

    pub fn view(&self) -> Element<'_, Message> {
        let content: Element<'_, SettingsMessage> = Container::new(self.proxy_settings_view())
            .padding(20.0)
            .into();
        content.map(Message::SettingsMessage)
    }

    //  https://github.com/iced-rs/iced/blob/master/futures/src/subscription.rs
    //  https://github.com/iced-rs/iced/blob/master/futures/src/stream.rs
    pub fn subscription(&self) -> Subscription<Message> {
        let mut subscriptions = vec![];
        for proxy in self.proxies.values() {
            let proxy_id = proxy.id;
            let port = proxy.port;
            let proxy_service_config = ProxyServiceConfig::from(self.certificate_store.clone());

            let stream =
                iced::stream::channel(100, move |sender: mpsc::Sender<ProxyEvent>| async move {
                    proxy::service::serve(proxy_id, port, sender, proxy_service_config).await
                });

            subscriptions.push(Subscription::run_with_id(proxy_id, stream));
        }

        Subscription::batch(subscriptions).map(Message::ProxyEvent)
    }

    pub fn update(&mut self, message: SettingsMessage) -> Task<SettingsMessage> {
        match message {
            SettingsMessage::AddProxy => match self.proxy_port_request.parse::<u16>() {
                Ok(port) => {
                    let proxy = ProxyConfig {
                        port,
                        id: self.proxies.len(),
                        auto_start: false,
                    };

                    self.add_proxy(&proxy);

                    self.proxy_port_request = String::default();
                    self.is_port_format_error = false;

                    let _ = config::save_proxy(&proxy)
                        .inspect_err(|e| println!("failed to save proxy: {e}"));
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
            SettingsMessage::StartProxy(_id) => {}
            SettingsMessage::StopProxy(id) => {
                let proxy = self.proxies.get_mut(&id).unwrap();
                let command = proxy.command.clone();

                proxy.status = ProxyState::Stopped;
                return Task::perform(
                    async move {
                        if let Some(mut cmd) = command {
                            cmd.send(ProxyCommand::Stop).await.unwrap();
                        }
                    },
                    |_| SettingsMessage::Update,
                );
            }
            SettingsMessage::ProxyEvent(event) => match event {
                ProxyEvent::ProxyError(id) => {
                    let proxy = self.proxies.get_mut(&id).unwrap();
                    proxy.status = ProxyState::Error;
                }
                ProxyEvent::Initialized((id, command_tx)) => {
                    let proxy = self.proxies.get_mut(&id).unwrap();
                    proxy.status = ProxyState::Stopped;
                    let _ = proxy.command.insert(command_tx);

                    if proxy.auto_start {
                        return self.start_proxy_cmd(id);
                    }
                }
                _ => {}
            },
            SettingsMessage::Update => {}
        }

        Task::none()
    }
}
