use crate::action::Action;
use crate::app::{Mode, ModeHolderLock};
use crate::components::{Component, area_util};
use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::palette::tailwind;
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Tabs};
use strum::{Display, EnumIter, FromRepr, IntoEnumIterator};
use unicode_width::UnicodeWidthStr;

pub(crate) struct Navigation {
    mode_holder: ModeHolderLock,
    item: NavigationItem,
}

impl Navigation {
    pub(crate) fn new(mode_holder: ModeHolderLock) -> Self {
        Self {
            mode_holder,
            item: NavigationItem::RecentChat,
        }
    }

    fn next(&mut self) {
        self.item.circle();
        let next_mode = match self.mode_holder.get_mode() {
            Mode::RecentChat => Mode::Contact,
            Mode::Contact => Mode::Setting,
            Mode::Setting => Mode::RecentChat,
            _ => self.mode_holder.get_mode(),
        };
        self.mode_holder.set_mode(next_mode);
    }
}

#[derive(Eq, PartialEq, Clone, Copy, Display, FromRepr, EnumIter)]
enum NavigationItem {
    #[strum(to_string = "最近聊天")]
    RecentChat,
    #[strum(to_string = "我的好友")]
    Contact,
    #[strum(to_string = "我的设置")]
    Setting,
}

impl NavigationItem {
    pub(crate) fn circle(self) -> Self {
        match self {
            NavigationItem::RecentChat => NavigationItem::Contact,
            NavigationItem::Contact => NavigationItem::Setting,
            NavigationItem::Setting => NavigationItem::RecentChat,
        }
    }

    /// Get the previous tab, if there is no previous tab return the current tab.
    pub(crate) fn previous(self) -> Self {
        let current_index: usize = self as usize;
        let previous_index = current_index.saturating_sub(1);
        Self::from_repr(previous_index).unwrap_or(self)
    }

    /// Get the next tab, if there is no next tab return the current tab.
    pub(crate) fn next(self) -> Self {
        let current_index = self as usize;
        let next_index = current_index.saturating_add(1);
        Self::from_repr(next_index).unwrap_or(self)
    }

    /// Return tab's name as a styled `Line`
    pub(crate) fn title(self) -> Line<'static> {
        format!("  {self}  ")
            .fg(tailwind::SLATE.c200)
            .bg(self.palette().c900)
            .into()
    }

    pub(crate) const fn palette(self) -> tailwind::Palette {
        match self {
            Self::RecentChat => tailwind::BLUE,
            Self::Contact => tailwind::EMERALD,
            Self::Setting => tailwind::INDIGO,
        }
    }
}

impl Component for Navigation {
    fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
        match self.mode_holder.get_mode() {
            Mode::RecentChat | Mode::Contact | Mode::Setting => {
                if let Action::NextTab = action {
                    self.next();
                }
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        match self.mode_holder.get_mode() {
            Mode::RecentChat
            | Mode::Chat
            | Mode::Contact
            | Mode::GroupManager
            | Mode::Setting
            | Mode::Alert => {
                let navigation_area = area_util::navigation_area(area);
                let titles = NavigationItem::iter().map(NavigationItem::title);
                let highlight_style = (Color::default(), self.item.palette().c700);
                let selected_tab_index = self.item as usize;
                let padding = cal_padding(&navigation_area);
                let tabs = Tabs::new(titles)
                    .block(
                        Block::default()
                            .title("Chat-Tui")
                            .title_style(Style::default().fg(Color::Green))
                            .borders(Borders::BOTTOM)
                            .border_style(Style::default().fg(Color::Green))
                            .title_alignment(Alignment::Center),
                    )
                    .highlight_style(highlight_style)
                    .select(selected_tab_index)
                    .padding(" ".repeat(padding), " ".repeat(padding))
                    .divider("-");
                frame.render_widget(tabs, navigation_area);
            }
            _ => {}
        };
        Ok(())
    }
}

fn cal_padding(area: &Rect) -> usize {
    let width = area.width as usize;
    let navi_width = NavigationItem::iter()
        .map(|item| item.to_string().as_str().width_cjk() + 4)
        .sum::<usize>()
        + 2;
    let len = NavigationItem::iter().len();
    if width < navi_width {
        0
    } else {
        (width - navi_width) / (len * 2)
    }
}
#[cfg(test)]
mod tests {
    use strum::IntoEnumIterator;
    use unicode_width::UnicodeWidthStr;
    use crate::components::navigation::NavigationItem;

    #[test]
    fn test_string_width() {
        let string = String::from("我的好友");
        let width = UnicodeWidthStr::width(string.as_str());
        println!("width: {width}, string: {string}");
        let width = UnicodeWidthStr::width_cjk(string.as_str());
        println!("width_cjk: {width}, string: {string}");
    }

    #[test]
    fn test_len() {
        let i = NavigationItem::iter().count();
        println!("{}", i);
    }
}
