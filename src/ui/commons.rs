use iced::widget::container;
use iced::{Border, Color, Element, Length};

pub fn bordered_view<'a, T: 'a>(content: Element<'a, T>) -> Element<'a, T> {
    let content = container(content)
        .style(|_theme| container::Style {
            border: Border {
                color: Color::from_rgb8(195, 195, 195),
                radius: iced::border::Radius::new(6),
                width: 1.0,
            },
            ..Default::default()
        })
        .padding(20)
        .height(Length::Fill)
        .width(Length::Fill);

    container(content).padding(5.0).into()
}
