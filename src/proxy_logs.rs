use crate::proxy::{ProxyEvent, ProxyLogRow};
use crate::Message;
use iced::widget::{button, row, Column, Container, Text};
use iced::{Command, Element, Length};
use std::collections::HashMap;

type PacketId = usize;

#[derive(Default)]
pub struct ProxyLogs {
    last_id: usize,
    packets: HashMap<PacketId, ProxyLogRow>,
    selected_packet: PacketId,
}

#[derive(Debug, Clone)]
pub enum ProxyLogMessage {
    ProxyEvent(ProxyEvent),
    SelectPacket(PacketId),
}

impl ProxyLogs {
    fn insert_packet(&mut self, row: ProxyLogRow) {
        let _ = self.packets.insert(self.last_id, row);
        self.last_id += 1;
    }

    pub fn view(&self) -> Element<'_, Message> {
        let mut content = Column::new();

        let header = row!(
            Text::new("Id").width(Length::Fixed(50.0)),
            Text::new("Pxy id").width(Length::Fixed(50.0)),
            Text::new("Url").width(Length::Fill)
        );
        content = content.push(header);

        for (id, packet) in &self.packets {
            let row = row!(
                Text::new(id.to_string()).width(Length::Fixed(50.0)),
                Text::new(packet.proxy_id.to_string()).width(Length::Fixed(50.0)),
                Text::new(&packet.url).width(Length::Fill)
            );
            content = content.push(button(row).on_press(ProxyLogMessage::SelectPacket(*id)));
        }
        let content: Element<'_, ProxyLogMessage> = Container::new(content).padding(20.0).into();
        content.map(Message::ProxyLogMessage)
    }

    pub fn update(&mut self, message: ProxyLogMessage) -> Command<ProxyLogMessage> {
        match message {
            ProxyLogMessage::ProxyEvent(event) => match event {
                ProxyEvent::NewLogRow(row) => {
                    self.insert_packet(row);
                }
                _ => {}
            },
            ProxyLogMessage::SelectPacket(packet_id) => {
                self.selected_packet = packet_id;
            }
        }
        Command::none()
    }
}
