use crate::db::logs;
use crate::proxy::ProxyEvent;
use crate::Message;
use iced::widget::pane_grid::Pane;
use iced::widget::{
    button, column, container, pane_grid, row, scrollable, text, Column, Container, PaneGrid, Text,
};
use iced::{Element, Length, Task};

type PacketId = usize;

pub struct ProxyLogs {
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
                    Some(packet_id) => self.request_viewer_view(packet_id),
                    None => text("no row selected").into(),
                },
                Panes::ResponseViewer => match self.focused_row {
                    Some(packet_id) => self.response_viewer_view(packet_id),
                    None => text("no row selected").into(),
                },
            })
        });

        container(pane_grid).padding(10).into()
    }

    fn response_viewer_view(&self, packet_id: PacketId) -> Element<'_, Message> {
        let content = match logs::get_full_response_row(packet_id) {
            Ok(Some(response)) => {
                let mut raw_request = format!("HTTP/1.1 {}\n", response.status_code);

                for (key, value) in response.headers {
                    raw_request.push_str(&format!("{key}: {value}\n"));
                }

                raw_request.push('\n');
                raw_request.push_str(&String::from_utf8_lossy(&response.body));

                text(raw_request)
            }
            Err(e) => text(format!("failed to get request: {e:#?}")),
            _ => text("request not found"),
        };

        container(scrollable(content)).padding(10).into()
    }

    fn request_viewer_view(&self, packet_id: PacketId) -> Element<'_, Message> {
        let content = match logs::get_full_request_row(packet_id) {
            Ok(Some(log_row)) => {
                let mut raw_request = format!(
                    "{} {} HTTP/1.1\n",
                    log_row.request_summary.method, log_row.request_summary.path
                );

                for (key, value) in log_row.request_headers {
                    raw_request.push_str(&format!("{key}: {value}\n"));
                }

                raw_request.push('\n');
                raw_request.push_str(&String::from_utf8_lossy(&log_row.request_body));

                text(raw_request)
            }
            Err(e) => text(format!("failed to get request: {e:#?}")),
            _ => text("request not found"),
        };

        container(scrollable(content)).padding(10).into()
    }

    fn proxy_table_view(&self) -> Element<'_, Message> {
        let header = row!(
            Text::new("Id").width(Length::Fixed(75.0)),
            Text::new("Pxy id").width(Length::Fixed(75.0)),
            Text::new("Authority").width(Length::Fixed(400.0)),
            Text::new("Path").width(Length::Fill)
        );

        let mut rows = Column::new();
        for summary in logs::get_packets_summary().unwrap() {
            let row = row!(
                Text::new(summary.packet_id.to_string()).width(Length::Fixed(75.0)),
                Text::new(summary.proxy_id.to_string()).width(Length::Fixed(75.0)),
                Text::new(summary.authority).width(Length::Fixed(400.0)),
                Text::new(summary.path).width(Length::Fill)
            );

            let mut row_button =
                button(row).on_press(ProxyLogMessage::SelectPacket(summary.packet_id));
            if self.focused_row != Some(summary.packet_id) {
                row_button = row_button.style(button::primary);
            }

            rows = rows.push(row_button);
        }
        let rows = scrollable(rows);
        let content = column![header, rows];

        let content: Element<'_, ProxyLogMessage> = Container::new(content).padding(20.0).into();
        content.map(Message::ProxyLogMessage)
    }

    pub fn update(&mut self, message: ProxyLogMessage) -> Task<ProxyLogMessage> {
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
        Task::none()
    }
}
