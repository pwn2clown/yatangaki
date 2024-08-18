use crate::proxy::certificates::CertificateStore;
use crate::proxy::ProxyEvent;
use iced::{executor, Application, Command, Length, Settings, Theme};
use iced_aw::{TabLabel, Tabs};
use ui::proxy_logs::{ProxyLogMessage, ProxyLogs};
use ui::settings::{SettingsMessage, SettingsTabs};
use ui::start_menu::{StartMenu, StartMenuMessage};

mod db;
mod proxy;
mod ui;

enum AppState {
    Menu,
    ProjectLoaded(String),
}

struct App {
    loaded_project: Option<String>,
    state: AppState,
    selected_tab: TabId,
    settings_tab: ui::settings::SettingsTabs,
    proxy_logs: ui::proxy_logs::ProxyLogs,
    start_menu: StartMenu,
}

#[derive(Debug)]
enum Message {
    TabSelected(TabId),
    SettingsMessage(SettingsMessage),
    ProxyLogMessage(ProxyLogMessage),
    ProxyEvent(ProxyEvent),
    StartMenuEvent(StartMenuMessage),
}

impl Application for App {
    type Message = Message;
    type Flags = ();
    type Theme = Theme;
    type Executor = executor::Default;

    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
        //  Certificate store should be loaded afterwards in order to display potential errors
        let certificate_store = CertificateStore::generate().unwrap();

        (
            Self {
                loaded_project: None,
                state: AppState::Menu,
                selected_tab: TabId::Settings,
                settings_tab: SettingsTabs::new(certificate_store),
                proxy_logs: ProxyLogs::new(),
                start_menu: StartMenu::new(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        let mut title = "Yatangaki".to_owned();

        if let Some(project_name) = &self.loaded_project {
            title.push_str(&format!("Yatangaki - [{project_name}]"));
        }

        title
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
                return self
                    .proxy_logs
                    .update(message)
                    .map(Message::ProxyLogMessage);
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
            Message::StartMenuEvent(event) => match event {
                StartMenuMessage::LoadSelectedProject(Some(project_name)) => {
                    if let Err(e) = db::logs::create_project_db(&project_name) {
                        println!("failed to create net log db: {e}");
                    }

                    self.state = AppState::ProjectLoaded(project_name);
                }
                _ => return self.start_menu.update(event).map(Message::StartMenuEvent),
            },
        }
        Command::none()
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        self.settings_tab.subscription()
    }

    fn view(&self) -> iced::Element<'_, Self::Message> {
        match &self.state {
            AppState::Menu => self.start_menu.view(),
            AppState::ProjectLoaded(_project_name) => Tabs::new(Message::TabSelected)
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
                .into(),
        }
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
