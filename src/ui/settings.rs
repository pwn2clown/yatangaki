use crate::db::{
    config::{self, ProxyConfig},
    logs,
};
use crate::proxy::certificates::CertificateStore;
use crate::proxy::{
    self, service::ProxyServiceConfig, ProxyCommand, ProxyEvent, ProxyId, ProxyState,
};
use crate::Message;
use iced::widget::{button, column, row, text, vertical_rule, Column, Scrollable};
use iced::{
    futures::{channel::mpsc, SinkExt},
    widget::text_input,
};
use iced::{Element, Length, Subscription, Task};
use std::{collections::HashMap, process::Stdio};

pub struct Proxy {
    auto_start: bool,
    id: ProxyId,
    port: u16,
    status: ProxyState,
    command: Option<mpsc::Sender<ProxyCommand>>,
    config: ProxyServiceConfig,
}

pub struct SettingsTabs {
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

    pub fn proxy_settings_view(&self) -> Element<'_, SettingsMessage> {
        let mut proxy_list = Column::new();
        for (id, proxy) in &self.proxies {
            let mut button = button(
                row!(
                    text(id).width(Length::Fixed(50.0)).size(12.0),
                    text(proxy.port).width(Length::Fixed(150.0)).size(12.0),
                )
                .height(Length::Fixed(16.0)),
            )
            .on_press(SettingsMessage::SelectProxy(proxy.id));

            if self.selected_proxy != Some(*id) {
                button = button.style(button::secondary);
            }

            proxy_list = proxy_list.push(button);
        }

        let mut proxy_table = column![
            row!(
                text("id").width(Length::Fixed(50.0)),
                text("port").width(Length::Fixed(150.0))
            ),
            Scrollable::new(proxy_list).height(Length::Fixed(16.0 * 5.0)),
            row!(
                text_input("enter proxy port", &self.proxy_port_request)
                    .on_input(SettingsMessage::ProxyPortRequest)
                    .width(Length::Fixed(150.0)),
                button("add").on_press(SettingsMessage::AddProxy).height(30)
            )
        ]
        .width(200.0);

        if self.proxy_port_request.parse::<u16>().is_err() && !self.proxy_port_request.is_empty() {
            proxy_table = proxy_table.push(text("error: bad port format"));
        }

        let mut proxy_settings = row!(proxy_table, vertical_rule(1)).spacing(30);

        if let Some(id) = self.selected_proxy {
            let mut config = Column::new();

            if let Some(proxy) = self.proxies.get(&id) {
                config = config.push(match proxy.status {
                    ProxyState::Running => button("stop").on_press(SettingsMessage::StopProxy(id)),
                    _ => button("start").on_press(SettingsMessage::StartProxy(id)),
                });

                if proxy.status == ProxyState::Error {
                    config = config.push(text("an error occured"));
                }
            }

            proxy_settings = proxy_settings.push(config);
        } else {
            proxy_settings = proxy_settings.push(text("no proxy selected"));
        };

        super::commons::bordered_view(proxy_settings.into())
    }

    pub fn view(&self) -> Element<'_, Message> {
        let content: Element<'_, SettingsMessage> = self.proxy_settings_view();
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
            SettingsMessage::AddProxy => {
                if let Ok(port) = self.proxy_port_request.parse::<u16>() {
                    let proxy = ProxyConfig {
                        port,
                        id: self.proxies.len(),
                        auto_start: false,
                    };

                    self.add_proxy(&proxy);
                    self.proxy_port_request = String::default();
                    let _ = config::save_proxy(&proxy)
                        .inspect_err(|e| println!("failed to save proxy: {e}"));
                }
            }
            SettingsMessage::SelectProxy(proxy_id) => {
                let _ = self.selected_proxy.insert(proxy_id);
            }
            SettingsMessage::ProxyPortRequest(port) => {
                self.proxy_port_request = port;
            }
            SettingsMessage::StartProxy(id) => {
                return self.start_proxy_cmd(id);
            }
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
