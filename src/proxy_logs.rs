use crate::db::Db;
use crate::proxy::{ProxyEvent, ProxyLogRow};
use crate::Message;
use iced::widget::pane_grid::Pane;
use iced::widget::{
    button, column, container, pane_grid, row, scrollable, text, Column, Container, PaneGrid, Text,
};
use iced::{Command, Element, Length};

type PacketId = usize;

pub struct ProxyLogs {
    last_id: usize,
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
            focused_row: None,
            panes,
            main_pane: pane,
            request_viewer_displayed: false,
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let pane_grid = PaneGrid::new(&self.panes, |_id, pane, _is_maximized| {
            pane_grid::Content::new(match pane {
                Panes::Logs => self.proxy_table_view(),
                Panes::RequestViewer => match self.focused_row {
                    Some(row_id) => text("request"),
                    None => text("no row selected"),
                }
                .into(),
                Panes::ResponseViewer => text("response content").into(),
            })
        });

        container(pane_grid).padding(10).into()
    }

    fn proxy_table_view(&self) -> Element<'_, Message> {
        let header = row!(
            Text::new("Id").width(Length::Fixed(75.0)),
            Text::new("Pxy id").width(Length::Fixed(75.0)),
            Text::new("Authority").width(Length::Fixed(400.0)),
            Text::new("Path").width(Length::Fill)
        );

        let mut rows = Column::new();
        for summary in Db::get_packets_summary().unwrap() {
            let row = row!(
                Text::new(summary.packet_id.to_string()).width(Length::Fixed(75.0)),
                Text::new(summary.proxy_id.to_string()).width(Length::Fixed(75.0)),
                Text::new(summary.authority).width(Length::Fixed(400.0)),
                Text::new(summary.path).width(Length::Fill)
            );

            let mut row_button =
                button(row).on_press(ProxyLogMessage::SelectPacket(summary.packet_id));
            if self.focused_row != Some(summary.packet_id) {
                row_button = row_button.style(iced::theme::Button::Secondary);
            }

            rows = rows.push(row_button);
        }
        let rows = scrollable(rows);
        let content = column![header, rows];

        let content: Element<'_, ProxyLogMessage> = Container::new(content).padding(20.0).into();
        content.map(Message::ProxyLogMessage)
    }

    pub fn update(&mut self, message: ProxyLogMessage) -> Command<ProxyLogMessage> {
        match message {
            ProxyLogMessage::ProxyEvent(event) => match event {
                ProxyEvent::NewRequestLogRow => {}
                ProxyEvent::NewResponseLogRow => {}
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
