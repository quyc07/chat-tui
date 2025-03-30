use ratatui::layout::{Constraint, Direction, Layout, Rect};

// ANCHOR: centered_rect
/// helper function to create a centered rect using up certain percentage of the available rect `r`
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    // Cut the given rectangle into three vertical pieces
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    // Then cut the middle vertical piece into three width-wise pieces
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1] // Return the middle chunk
}
// ANCHOR_END: centered_rect

pub(crate) fn total_area(rect: Rect) -> Rect {
    centered_rect(70, 100, rect)
}

pub(crate) fn navigation_area(rect: Rect) -> Rect {
    let area = total_area(rect);
    let [navigation_area, _] =
        Layout::vertical([Constraint::Length(3), Constraint::Fill(1)]).areas(area);
    navigation_area
}

pub(crate) fn dynamic_area(rect: Rect) -> Rect {
    let area = total_area(rect);
    let [_, dynamic_area] =
        Layout::vertical([Constraint::Length(3), Constraint::Fill(1)]).areas(area);
    dynamic_area
}

pub(crate) fn recent_chat(rect: Rect) -> Rect {
    let dynamic_area = dynamic_area(rect);
    let [recent_chat_area, _] =
        Layout::horizontal([Constraint::Percentage(35), Constraint::Fill(1)]).areas(dynamic_area);
    recent_chat_area
}

pub(crate) fn chat(rect: Rect) -> Rect {
    let dynamic_area = dynamic_area(rect);
    let [_, chat_area] =
        Layout::horizontal([Constraint::Percentage(35), Constraint::Fill(1)]).areas(dynamic_area);
    chat_area
}

pub(crate) fn alert_area(rect: Rect) -> Rect {
    let area = total_area(rect);
    centered_rect(80, 50, area)
}
