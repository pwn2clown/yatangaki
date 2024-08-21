use crate::db::config;
use crate::Message;
use iced::widget::{button, row, text, Button, Column, Container, Row, TextInput};
use iced::{Element, Length, Task};
use iced_aw::SelectionList;

#[derive(Debug, Clone)]
pub enum StartMenuMessage {
    SelectedProject(usize, String),
    CreateProject(String),
    UpdateProjectName(String),
    LoadSelectedProject(String),
    DeleteProject(String, usize),
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

    fn project_list_view(&self) -> Column<'_, StartMenuMessage> {
        let select_list_width = Length::Fixed(400.0);

        let project_list = SelectionList::new(&self.projects, StartMenuMessage::SelectedProject)
            .height(Length::Fixed(200.0))
            .width(select_list_width);

        let add_project_button: Button<'_, StartMenuMessage> = button("Add").on_press(
            StartMenuMessage::CreateProject(self.input_project_name.clone()),
        );

        let project_name_input = Row::new()
            .push(
                TextInput::new("New project", &self.input_project_name)
                    .on_input(StartMenuMessage::UpdateProjectName),
            )
            .push(add_project_button)
            .width(select_list_width);

        Column::new()
            .push(text("Select project:"))
            .push(project_list)
            .push(project_name_input)
    }

    fn selected_project_menu(&self) -> Element<'_, StartMenuMessage> {
        match self.selected_project_index {
            Some(index) => {
                let selected_project_name = self.projects.get(index).unwrap();

                let load_project_button: Button<'_, StartMenuMessage> = button("Load").on_press(
                    StartMenuMessage::LoadSelectedProject(selected_project_name.clone()),
                );

                let delete_project_button: Button<'_, StartMenuMessage> = button("Delete")
                    .on_press(StartMenuMessage::DeleteProject(
                        selected_project_name.clone(),
                        index,
                    ));

                Column::new()
                    .push(load_project_button)
                    .push(delete_project_button)
                    .into()
            }
            None => text("no project selected").into(),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let content = row!(self.project_list_view(), self.selected_project_menu()).spacing(50);

        let content: Element<'_, StartMenuMessage> = Container::new(content).padding(20.0).into();
        content.map(Message::StartMenuEvent)
    }

    pub fn update(&mut self, message: StartMenuMessage) -> Task<StartMenuMessage> {
        match message {
            StartMenuMessage::CreateProject(project_name) => {
                if !project_name.is_empty() {
                    let _ = config::create_project(&project_name);
                    self.projects.push(project_name);
                    self.input_project_name = String::default();
                }
            }
            StartMenuMessage::SelectedProject(row_index, _project_name) => {
                let _ = self.selected_project_index.insert(row_index);
            }
            StartMenuMessage::UpdateProjectName(project_name) => {
                self.input_project_name = project_name;
            }
            StartMenuMessage::DeleteProject(name, index) => {
                if let Err(e) = config::delete_project(&name) {
                    println!("failed to delete project: {e}");
                    self.projects.remove(index);
                    self.selected_project_index = None;
                }
            }
            StartMenuMessage::LoadSelectedProject(_project_name) => {}
        }
        Task::none()
    }
}
