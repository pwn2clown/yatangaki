use crate::db::config;
use crate::Message;
use iced::widget::{button, text, Button, Column, Container, Row, TextInput};
use iced::{Command, Element, Length};
use iced_aw::SelectionList;

#[derive(Debug, Clone)]
pub enum StartMenuMessage {
    SelectedProject(usize, String),
    CreateProject(String),
    UpdateProjectName(String),
    LoadSelectedProject(Option<String>),
}

pub struct StartMenu {
    selected_project_index: Option<usize>,
    input_project_name: String,
    projects: Vec<String>,
    err: Option<String>,
}

impl StartMenu {
    pub fn new() -> Self {
        let (projects, err) = match config::project_list() {
            Ok(projects) => (projects, None),
            Err(e) => (vec![], Some(e.to_string())),
        };

        Self {
            selected_project_index: None,
            input_project_name: String::default(),
            projects,
            err,
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let select_list_width = Length::Fixed(400.0);

        let project_list = SelectionList::new(&self.projects, StartMenuMessage::SelectedProject)
            .height(Length::Fixed(200.0))
            .width(select_list_width);

        let add_project_button: Button<'_, StartMenuMessage> = button("Add").on_press(
            StartMenuMessage::CreateProject(self.input_project_name.clone()),
        );

        let maybe_selected_project = match self.selected_project_index {
            Some(index) => self.projects.get(index).cloned(),
            None => None,
        };

        let load_project_button: Button<'_, StartMenuMessage> = button("Load").on_press(
            StartMenuMessage::LoadSelectedProject(maybe_selected_project),
        );

        let project_name_input = Row::new()
            .push(
                TextInput::new("New project", &self.input_project_name)
                    .on_input(StartMenuMessage::UpdateProjectName),
            )
            .push(add_project_button)
            .width(select_list_width);

        let content = Column::new()
            .push(text("Select project:"))
            .push(project_list)
            .push(project_name_input)
            .push(load_project_button);

        let content: Element<'_, StartMenuMessage> = Container::new(content).padding(20.0).into();

        content.map(Message::StartMenuEvent)
    }

    pub fn update(&mut self, message: StartMenuMessage) -> Command<StartMenuMessage> {
        match message {
            StartMenuMessage::CreateProject(project_name) => {
                let _ = config::create_project(&project_name);

                self.projects.push(project_name);
                self.input_project_name = String::default();
            }
            StartMenuMessage::SelectedProject(row_index, _project_name) => {
                let _ = self.selected_project_index.insert(row_index);
            }
            StartMenuMessage::UpdateProjectName(project_name) => {
                self.input_project_name = project_name;
            }
            StartMenuMessage::LoadSelectedProject(_project_name) => {}
        }
        Command::none()
    }
}
