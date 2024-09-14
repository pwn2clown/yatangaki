use crate::db::logs;
use crate::proxy::types::ProxyEvent;
use crate::Message;
use iced::widget::pane_grid::Pane;
use iced::widget::{
    button, column, container, horizontal_space, pane_grid, row, scrollable, text, text_input,
    Column, Container, PaneGrid,
};
use iced::{Element, Length, Task};

type PacketId = usize;

pub struct ProxyLogs {
    raw_search_query: String,
    focused_row: Option<PacketId>,
    panes: pane_grid::State<Panes>,
    main_pane: Pane,
    selected_request_content: Option<String>,
    selected_response_content: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ProxyLogMessage {
    ProxyEvent(ProxyEvent),
    SelectPacket(PacketId),
    UpdateQuery(String),
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
            raw_search_query: String::default(),
            focused_row: None,
            panes,
            main_pane: pane,
            selected_request_content: None,
            selected_response_content: None,
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        PaneGrid::new(&self.panes, |_id, pane, _is_maximized| {
            pane_grid::Content::new(match pane {
                Panes::Logs => self.proxy_table_view(),
                Panes::RequestViewer => self.request_viewer_view(),
                Panes::ResponseViewer => self.response_viewer_view(),
            })
        })
        .into()
    }

    fn response_viewer_view(&self) -> Element<'_, Message> {
        container(scrollable(match &self.selected_response_content {
            Some(raw_response) => text(raw_response),
            _ => text("error: response not found"),
        }))
        .padding(10)
        .into()
    }

    fn request_viewer_view(&self) -> Element<'_, Message> {
        container(scrollable(match &self.selected_request_content {
            Some(raw_request) => text(raw_request),
            _ => text("error: request not found"),
        }))
        .padding(10)
        .into()
    }

    fn proxy_table_view(&self) -> Element<'_, Message> {
        let mut rows = Column::new();

        for summary in logs::get_row_metadata().unwrap() {
            let mut row_button = button(row!(
                text(summary.packet_id).width(Length::Fixed(75.0)),
                text(summary.proxy_id).width(Length::Fixed(75.0)),
                text(summary.authority).width(Length::Fixed(400.0)),
                text(summary.path).width(Length::Fill)
            ))
            .on_press(ProxyLogMessage::SelectPacket(summary.packet_id));

            if self.focused_row != Some(summary.packet_id) {
                row_button = row_button.style(button::secondary);
            }

            rows = rows.push(row_button);
        }

        let content: Element<'_, ProxyLogMessage> = Container::new(column![
            text_input("Search", &self.raw_search_query)
                .width(Length::Fill)
                .on_input(ProxyLogMessage::UpdateQuery),
            horizontal_space().height(15),
            row!(
                text("Id").width(Length::Fixed(75.0)),
                text("Pxy id").width(Length::Fixed(75.0)),
                text("Authority").width(Length::Fixed(400.0)),
                text("Path").width(Length::Fill)
            ),
            scrollable(rows)
        ])
        .padding(20.0)
        .into();

        content.map(Message::ProxyLogMessage)
    }

    pub fn update(&mut self, message: ProxyLogMessage) -> Task<ProxyLogMessage> {
        match message {
            ProxyLogMessage::ProxyEvent(_event) => {}
            ProxyLogMessage::SelectPacket(packet_id) => {
                if self.selected_request_content.is_none() {
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
                }

                if self.focused_row != Some(packet_id) {
                    if let Ok(Some(row)) = logs::get_full_row(packet_id) {
                        let _ = self.selected_request_content.insert(row.request_as_str());
                        let _ = self
                            .selected_response_content
                            .insert(row.response_as_str().unwrap_or_default());
                    }

                    let _ = self.focused_row.insert(packet_id);
                }
            }
            ProxyLogMessage::UpdateQuery(query) => {
                self.raw_search_query = query;
            }
        }
        Task::none()
    }
}
