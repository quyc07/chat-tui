use crate::action::{Action, ConfirmEvent};
use crate::app::{Mode, ModeHolderLock};
use crate::components::group_manager::ManageAction;
use crate::components::recent_chat::SELECTED_STYLE;
use crate::components::{area_util, Component};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Alignment, Constraint, Layout, Margin, Rect};
use ratatui::prelude::Text;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::widgets::Paragraph;
use ratatui::widgets::{Block, Clear};
use ratatui::widgets::{Borders, HighlightSpacing, List, ListItem};
use ratatui::{symbols, Frame};
use strum::IntoEnumIterator;

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
            if let Some(ConfirmEvent::GroupManage(None)) = self.confirm_event {
                self.draw_manage_action(frame, area);
            } else {
                self.draw_common(frame, area);
            }
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

    fn draw_common(&mut self, frame: &mut Frame, area: Rect) {
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

    fn draw_manage_action(&mut self, frame: &mut Frame, area: Rect) {
        let area = area_util::alert_area(area);
        let [_, alert_area, _] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(6),
            Constraint::Fill(1),
        ])
        .areas(area);
        frame.render_widget(Clear, alert_area);
        let msg = match self.confirm_event {
            None => "Esc to quit.",
            Some(_) => "Esc to quit, ↑↓ To Select, Enter to submit.",
        };
        let block = Block::new()
            .title(msg)
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_set(symbols::border::ROUNDED);
        frame.render_widget(block, alert_area);
        let [_, alert_msg_area, items_area, _] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .areas(alert_area);
        let alert_msg = Paragraph::new(self.msg.as_str())
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center);
        frame.render_widget(
            alert_msg,
            alert_msg_area.inner(Margin {
                horizontal: 1,
                vertical: 0,
            }),
        );
        let items: Vec<ListItem> = ManageAction::iter()
            .map(|action| ListItem::new(Text::from(action)))
            .collect();
        // Create a List from all list items and highlight the currently selected one
        let list = List::new(items)
            .highlight_style(SELECTED_STYLE)
            .highlight_spacing(HighlightSpacing::Always);
        frame.render_widget(
            list,
            items_area.inner(Margin {
                horizontal: 2,
                vertical: 0,
            }),
        );
    }
}
