use crate::Message;
use iced::widget::{
    text,
    text_editor::{Action, Content},
};
use iced::{Element, Task};

#[derive(Debug, Clone)]
pub enum EditorMessage {
    EditorResized(u16),
    EditorAction(Action),
}

pub struct RequestEditor {
    content: Content,
}

impl RequestEditor {
    pub fn new() -> Self {
        Self {
            content: Content::with_text("issou"),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        super::commons::bordered_view(text("request editor").into())
    }

    pub fn update(&mut self, _message: EditorMessage) -> Task<EditorMessage> {
        Task::none()
    }
}
