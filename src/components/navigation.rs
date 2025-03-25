use crate::action::Action;
use crate::app::{Mode, ModeHolderLock};
use crate::components::{area_util, Component};
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::palette::tailwind;
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Tabs};
use ratatui::Frame;
use strum::{Display, EnumIter, FromRepr, IntoEnumIterator};

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
}

#[derive(Eq, PartialEq, Clone, Copy, Display, FromRepr, EnumIter)]
enum NavigationItem {
    #[strum(to_string = "最近聊天")]
    RecentChat,
    #[strum(to_string = "我的好友")]
    Contact,
    #[strum(to_string = "设置")]
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
                    self.item = self.item.circle()
                }
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        let area = area_util::total_area(area);
        match self.mode_holder.get_mode() {
            Mode::RecentChat | Mode::Contact | Mode::Setting => {
                let [navigation_area, _] =
                    Layout::vertical([Constraint::Length(3), Constraint::Fill(1)]).areas(area);
                let navigation_area = area_util::centered_rect(50, 100, navigation_area);
                let titles = NavigationItem::iter().map(NavigationItem::title);
                let highlight_style = (Color::default(), self.item.palette().c700);
                let selected_tab_index = self.item as usize;
                let tabs = Tabs::new(titles)
                    .block(
                        Block::default()
                            .borders(Borders::BOTTOM)
                            .border_style(Style::default().fg(Color::Yellow))
                            .title("Chat-Tui")
                            .title_style(Style::default().fg(Color::Green))
                            .title_alignment(Alignment::Center),
                    )
                    .highlight_style(highlight_style)
                    .select(selected_tab_index)
                    .padding("     ", "     ")
                    .divider("-");
                frame.render_widget(tabs, navigation_area);
            }
            _ => {}
        };
        Ok(())
    }
}
