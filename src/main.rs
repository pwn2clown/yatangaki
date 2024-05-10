use iced::advanced::widget::Text;
use iced::{executor, Application, Command, Subscription, Theme};
use iced::{Length, Settings};
use iced_aw::{TabLabel, Tabs};
use settings::{SettingsMessage, SettingsTabs};

mod proxy;
mod settings;

struct App {
    selected_tab: TabId,
    settings_tab: settings::SettingsTabs,
}

#[derive(Debug)]
enum Message {
    TabSelected(TabId),
    SettingsMessage(SettingsMessage),
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
                Command::none()
            }
            Message::SettingsMessage(message) => self
                .settings_tab
                .update(message)
                .map(Message::SettingsMessage),
        }
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        Subscription::none()
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
                Text::new("logs"),
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
