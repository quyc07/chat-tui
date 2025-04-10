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
    centered_rect(60, 50, area)
}

pub(crate) fn contact_area(rect: Rect) -> Rect {
    dynamic_area(rect)
}

pub(crate) fn group_manager_area(rect: Rect) -> Rect {
    chat(rect)
}

pub(crate) fn setting_area(rect: Rect) -> Rect {
    dynamic_area(rect)
}

// 简单估算：按宽度估算换行后的行数
fn estimate_line_count(text: &str, width: u16) -> usize {
    text.lines()
        .map(|line| {
            let len = unicode_width::UnicodeWidthStr::width(line);
            (len as f32 / width.max(1) as f32).ceil() as usize
        })
        .sum()
}

pub(crate) fn cal_center_area(area: Rect, text: &str) -> Rect {
    // 1. 估算需要多少行（考虑终端宽度）
    let text_width = area.width.saturating_sub(2); // 留出 Block 的边框
    let estimated_lines = estimate_line_count(text, text_width);

    // 2. 计算垂直居中的 y 坐标
    let paragraph_height = estimated_lines as u16 + 2; // +2 for border
    let y_offset = area.y + (area.height.saturating_sub(paragraph_height)) / 2;
    Rect {
        x: area.x,
        y: y_offset,
        width: area.width,
        height: paragraph_height.min(area.height),
    }
}
