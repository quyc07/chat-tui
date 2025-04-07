use crate::action::{Action, ConfirmEvent};
use crate::app::{Mode, ModeHolderLock};
use crate::components::{Component, area_util};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;
use ratatui::widgets::{Block, Clear};
use ratatui::{Frame, symbols};

pub struct Alert {
    /// alert message
    msg: String,
    /// 全局状态
    mode_holder: ModeHolderLock,
    /// 确认事件
    confirm_event: Option<ConfirmEvent>,
    /// 上一个状态
    last_mode: Option<Mode>,
}

impl Component for Alert {
    fn handle_key_event(&mut self, key: KeyEvent) -> color_eyre::Result<Option<Action>> {
        match self.mode_holder.get_mode() {
            Mode::Alert => match key.code {
                KeyCode::Enter if self.confirm_event.is_some() => {
                    Ok(Some(Action::Confirm(self.confirm_event.clone().unwrap())))
                }
                KeyCode::Esc => {
                    self.close();
                    Ok(None)
                }
                _ => Ok(None),
            },
            _ => Ok(None),
        }
    }

    fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
        if let Action::Alert(msg, confirm_event) = action {
            self.msg = msg;
            self.confirm_event = confirm_event;
            self.last_mode = Some(self.mode_holder.get_mode());
            self.mode_holder.set_mode(Mode::Alert);
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        if self.mode_holder.get_mode() == Mode::Alert {
            let area = area_util::alert_area(area);
            let [_, alert_area, _] = Layout::vertical([
                Constraint::Fill(1),
                Constraint::Length(3),
                Constraint::Fill(1),
            ])
            .areas(area);
            frame.render_widget(Clear, alert_area);
            let msg = match self.confirm_event {
                None => "Esc to quit.",
                Some(_) => "Esc to quit, Enter to submit.",
            };
            let block = Block::new()
                .title(msg)
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_set(symbols::border::ROUNDED);
            let msg = Paragraph::new(self.msg.as_str())
                .style(Style::default().fg(Color::Yellow))
                .alignment(Alignment::Center)
                .block(block);
            frame.render_widget(msg, alert_area);
        }
        Ok(())
    }
}
const DEFAULT_ALERT_MSG: &str = "欢迎使用Chat-Tui！";

impl Alert {
    pub fn new(mode_holder: ModeHolderLock) -> Alert {
        Self {
            msg: DEFAULT_ALERT_MSG.to_string(),
            mode_holder,
            confirm_event: None,
            last_mode: None,
        }
    }

    fn close(&mut self) {
        self.msg = DEFAULT_ALERT_MSG.to_string();
        self.confirm_event.take();
        self.mode_holder.set_mode(self.last_mode.unwrap());
    }
}
