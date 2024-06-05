use crate::proxy::{ProxyEvent, ProxyLogRow};
use crate::Message;
use iced::widget::text_editor::Content;
use iced::widget::{button, row, Column, Container, Text, TextEditor};
use iced::{Command, Element, Length};
use iced_aw::{split::Axis, Split};
use std::collections::HashMap;

type PacketId = usize;

#[derive(Default)]
pub struct ProxyLogs {
    last_id: usize,
    packets: HashMap<PacketId, ProxyLogRow>,
    focused_row: Option<PacketId>,
    horizontal_divider_position: Option<u16>,
    editor_divider_position: Option<u16>,
}

#[derive(Debug, Clone)]
pub enum ProxyLogMessage {
    ProxyEvent(ProxyEvent),
    SelectPacket(PacketId),
    SplitResize(u16),
    EditorSplitResize(u16),
}

impl ProxyLogs {
    fn insert_packet(&mut self, row: ProxyLogRow) {
        let _ = self.packets.insert(self.last_id, row);
        self.last_id += 1;
    }

    pub fn view(&self) -> Element<'_, Message> {
        if self.focused_row.is_some() {
            Split::new(
                self.proxy_table_view(),
                Text::new("Request editor"),
                self.horizontal_divider_position,
                Axis::Horizontal,
                |position| Message::ProxyLogMessage(ProxyLogMessage::SplitResize(position)),
            )
            .into()
        } else {
            self.proxy_table_view()
        }
    }

    /*
    fn request_editor_view(&self) -> Element<'_, Message> {
        Split::new(
            TextEditor::new(&Content::with_text("issou la chancla")),
            TextEditor::new(&Content::with_text("risitas")),
            self.editor_divider_position,
            Axis::Vertical,
            |position| Message::ProxyLogMessage(ProxyLogMessage::EditorSplitResize(position)),
        )
        .into()
    }
    */

    fn proxy_table_view(&self) -> Element<'_, Message> {
        let mut content = Column::new();

        let header = row!(
            Text::new("Id").width(Length::Fixed(75.0)),
            Text::new("Pxy id").width(Length::Fixed(75.0)),
            Text::new("Authority").width(Length::Fixed(400.0)),
            Text::new("Path").width(Length::Fill)
        );
        content = content.push(header);

        for (id, packet) in &self.packets {
            let row = row!(
                Text::new(id.to_string()).width(Length::Fixed(75.0)),
                Text::new(packet.proxy_id.to_string()).width(Length::Fixed(75.0)),
                Text::new(packet.request.uri().authority().unwrap().as_str())
                    .width(Length::Fixed(400.0)),
                Text::new(packet.request.uri().path_and_query().map_or_else(
                    || String::from("/"),
                    |path_and_query| path_and_query.to_string()
                ))
                .width(Length::Fill)
            );

            let mut row_button = button(row).on_press(ProxyLogMessage::SelectPacket(*id));
            if self.focused_row != Some(*id) {
                row_button = row_button.style(iced::theme::Button::Secondary);
            }

            content = content.push(row_button);
        }
        let content: Element<'_, ProxyLogMessage> = Container::new(content).padding(20.0).into();
        content.map(Message::ProxyLogMessage)
    }

    pub fn update(&mut self, message: ProxyLogMessage) -> Command<ProxyLogMessage> {
        match message {
            ProxyLogMessage::ProxyEvent(event) => match event {
                ProxyEvent::PushLogRow(row) => {
                    self.insert_packet(row);
                }
                _ => {}
            },
            //  TODO: rename message to ToggleLowRow
            ProxyLogMessage::SelectPacket(packet_id) => {
                println!("selecting proxy log row {packet_id}");
                let _ = self.focused_row.insert(packet_id);
            }
            ProxyLogMessage::SplitResize(position) => {
                self.horizontal_divider_position = Some(position);
            }
            ProxyLogMessage::EditorSplitResize(position) => {
                self.editor_divider_position = Some(position);
            }
        }
        Command::none()
    }
}
