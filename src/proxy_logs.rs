use crate::proxy::{ProxyEvent, ProxyLogRow};
use crate::Message;
use iced::widget::pane_grid::Pane;
use iced::widget::{button, container, pane_grid, row, text, Column, Container, PaneGrid, Text};
use iced::{Command, Element, Length};
use std::collections::HashMap;

type PacketId = usize;

pub struct ProxyLogs {
    last_id: usize,
    packets: HashMap<PacketId, ProxyLogRow>,
    focused_row: Option<PacketId>,
    panes: pane_grid::State<Panes>,
    main_pane: Pane,
    request_viewer_displayed: bool,
}

#[derive(Debug, Clone)]
pub enum ProxyLogMessage {
    ProxyEvent(ProxyEvent),
    SelectPacket(PacketId),
}

enum Panes {
    RequestViewer,
    ResponseViewer,
    Logs,
}

impl ProxyLogs {
    pub fn new() -> Self {
        let (panes, pane) = pane_grid::State::new(Panes::Logs);

        Self {
            last_id: 0,
            packets: HashMap::default(),
            focused_row: None,
            panes,
            main_pane: pane,
            request_viewer_displayed: false,
        }
    }

    fn insert_packet(&mut self, row: ProxyLogRow) {
        let _ = self.packets.insert(self.last_id, row);
        self.last_id += 1;
    }

    pub fn view(&self) -> Element<'_, Message> {
        let pane_grid = PaneGrid::new(&self.panes, |_id, pane, _is_maximized| {
            pane_grid::Content::new(match pane {
                Panes::Logs => self.proxy_table_view(),
                Panes::RequestViewer => match self.focused_row {
                    Some(row_id) => {
                        let request = self.packets.get(&row_id).unwrap().request.clone();
                        //let body = String::from_utf8_lossy(request.body());
                        let method = request.method().as_str();
                        text(method).into()
                    }
                    None => text("no row selected").into(),
                },
                Panes::ResponseViewer => text("response content").into(),
            })
        });

        container(pane_grid).padding(10).into()
    }

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
                if !self.request_viewer_displayed {
                    let (pane, _) = self
                        .panes
                        .split(
                            pane_grid::Axis::Horizontal,
                            self.main_pane,
                            Panes::RequestViewer,
                        )
                        .unwrap();

                    let _ =
                        self.panes
                            .split(pane_grid::Axis::Vertical, pane, Panes::ResponseViewer);

                    self.request_viewer_displayed = true;
                }

                let _ = self.focused_row.insert(packet_id);
            }
        }
        Command::none()
    }
}
