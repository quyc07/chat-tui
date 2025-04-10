use crate::action::Action;
use crate::app::{Mode, ModeHolderLock};
use crate::components::{area_util, Component};
use crate::token::CURRENT_USER;
use crossterm::event::KeyEvent;
use ratatui::layout::{Alignment, Rect};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::{symbols, Frame};

pub(crate) struct Setting {
    mode_holder: ModeHolderLock,
}

impl Setting {
    pub(crate) fn new(mode_holder: ModeHolderLock) -> Self {
        Self { mode_holder }
    }
}

impl Component for Setting {
    fn handle_key_event(&mut self, _key: KeyEvent) -> color_eyre::Result<Option<Action>> {
        Ok(None)
    }

    fn update(&mut self, _action: Action) -> color_eyre::Result<Option<Action>> {
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        if self.mode_holder.get_mode() == Mode::Setting {
            let area = area_util::setting_area(area);
            let area = area_util::centered_rect(60, 10, area);
            let block = Block::new()
                .borders(Borders::ALL)
                .border_set(symbols::border::ROUNDED);
            let user = CURRENT_USER.get_user().user.unwrap();
            let text = format!(
                "你好{}，欢迎使用Chat-Tui！\n这里还没有实现哟～～～",
                user.name
            );
            let area = area_util::cal_center_area(area, text.as_str());
            let user_input = Paragraph::new(text)
                .alignment(Alignment::Center)
                .block(block);
            frame.render_widget(user_input, area);
        }
        Ok(())
    }
}
