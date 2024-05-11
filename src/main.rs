use iced::{executor, Application, Command, Theme};
use iced::{Length, Settings};
use iced_aw::{TabLabel, Tabs};
use proxy_logs::{ProxyLogMessage, ProxyLogs};
use settings::{SettingsMessage, SettingsTabs};

use crate::proxy::ProxyEvent;

mod proxy;
mod proxy_logs;
mod settings;
mod style;

struct App {
    selected_tab: TabId,
    settings_tab: settings::SettingsTabs,
    proxy_logs: proxy_logs::ProxyLogs,
}

#[derive(Debug)]
enum Message {
    TabSelected(TabId),
    SettingsMessage(SettingsMessage),
    ProxyLogMessage(ProxyLogMessage),
    ProxyEvent(ProxyEvent),
}

impl Application for App {
    type Message = Message;
    type Flags = ();
    type Theme = Theme;
    type Executor = executor::Default;

    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
        (
            Self {
                selected_tab: TabId::Settings,
                settings_tab: SettingsTabs::new(),
                proxy_logs: ProxyLogs::default(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        "Yatangaki".into()
    }

    fn update(&mut self, message: Self::Message) -> Command<Message> {
        match message {
            Message::TabSelected(tab_id) => {
                self.selected_tab = tab_id;
            }
            Message::SettingsMessage(message) => {
                return self
                    .settings_tab
                    .update(message)
                    .map(Message::SettingsMessage)
            }
            Message::ProxyLogMessage(message) => {
                println!("updating proxy logs state");
            }
            Message::ProxyEvent(event) => {
                //  TODO: should make fine grained update dispatch to avoid useless clones and
                //  update calls
                let commands = vec![
                    self.settings_tab
                        .update(SettingsMessage::ProxyEvent(event.clone()))
                        .map(Message::SettingsMessage),
                    self.proxy_logs
                        .update(ProxyLogMessage::ProxyEvent(event))
                        .map(Message::ProxyLogMessage),
                ];
                return Command::batch(commands);
            }
        }
        Command::none()
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        self.settings_tab.subscription()
    }

    fn view(&self) -> iced::Element<'_, Self::Message> {
        Tabs::new(Message::TabSelected)
            .text_size(12.0)
            .tab_bar_width(Length::Fixed(200.0))
            .push(
                TabId::Settings,
                TabLabel::Text("Settings".into()),
                self.settings_tab.view(),
            )
            .push(
                TabId::ProxyLogs,
                TabLabel::Text("Proxy logs".into()),
                self.proxy_logs.view(),
            )
            .set_active_tab(&self.selected_tab)
            .into()
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
enum TabId {
    Settings,
    ProxyLogs,
}

fn main() -> iced::Result {
    App::run(Settings::default())
}
