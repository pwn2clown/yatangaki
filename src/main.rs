use crate::proxy::certificates::CertificateStore;
use crate::proxy::ProxyEvent;
use iced::{Length, Task};
use iced_aw::{TabLabel, Tabs};
use ui::proxy_logs::{ProxyLogMessage, ProxyLogs};
use ui::request_editor::{EditorMessage, RequestEditor};
use ui::settings::{SettingsMessage, SettingsTabs};
use ui::start_menu::{StartMenu, StartMenuMessage};

mod db;
mod proxy;
mod ui;

enum AppState {
    Menu,
    ProjectLoaded(String),
}

#[derive(Debug)]
enum Message {
    TabSelected(TabId),
    SettingsMessage(SettingsMessage),
    ProxyLogMessage(ProxyLogMessage),
    ProxyEvent(ProxyEvent),
    StartMenuEvent(StartMenuMessage),
    RequestEditor(EditorMessage),
}

struct App {
    state: AppState,
    selected_tab: TabId,
    settings_tab: ui::settings::SettingsTabs,
    proxy_logs: ui::proxy_logs::ProxyLogs,
    start_menu: StartMenu,
    request_editor: RequestEditor,
}

impl Default for App {
    fn default() -> Self {
        //  Certificate store should be loaded afterwards in order to display potential errors
        let certificate_store = CertificateStore::generate().unwrap();

        Self {
            state: AppState::Menu,
            selected_tab: TabId::ProxyLogs,
            settings_tab: SettingsTabs::new(certificate_store),
            proxy_logs: ProxyLogs::new(),
            start_menu: StartMenu::new(),
            request_editor: RequestEditor::new(),
        }
    }
}

impl App {
    fn title(&self) -> String {
        let mut base_title = "Yatangaki".to_owned();
        if let AppState::ProjectLoaded(project_name) = &self.state {
            base_title.push_str(&format!(" - [{project_name}]"));
        }
        base_title
    }

    fn update(&mut self, message: Message) -> Task<Message> {
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
                return Task::batch(commands);
            }
            Message::StartMenuEvent(event) => match event {
                StartMenuMessage::LoadSelectedProject(project_name) => {
                    if let Err(e) = db::logs::create_project_db(&project_name) {
                        println!("failed to create net log db: {e}");
                    }

                    self.state = AppState::ProjectLoaded(project_name);
                }
                _ => return self.start_menu.update(event).map(Message::StartMenuEvent),
            },
            Message::RequestEditor(event) => {
                return self
                    .request_editor
                    .update(event)
                    .map(Message::RequestEditor);
            }
        }

        Task::none()
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        self.settings_tab.subscription()
    }

    fn view(&self) -> iced::Element<'_, Message> {
        match &self.state {
            AppState::Menu => self.start_menu.view(),
            AppState::ProjectLoaded(_project_name) => Tabs::new(Message::TabSelected)
                .text_size(12.0)
                .tab_bar_width(Length::Fixed(700.0))
                .push(
                    TabId::StartMenu,
                    TabLabel::Text("Load".into()),
                    self.start_menu.view(),
                )
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
                .push(
                    TabId::RequestEditor,
                    TabLabel::Text("Request editor".into()),
                    self.request_editor.view(),
                )
                .set_active_tab(&self.selected_tab)
                .into(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
enum TabId {
    StartMenu,
    Settings,
    ProxyLogs,
    RequestEditor,
}

fn main() -> iced::Result {
    iced::application(App::title, App::update, App::view)
        .subscription(App::subscription)
        .run()
}
