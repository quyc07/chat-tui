use crate::action::{Action, ConfirmEvent};
use crate::app::{Mode, ModeHolderLock};
use crate::components::Component;
use crossterm::event::KeyEvent;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;
use ratatui::Frame;

pub struct Alert {
    /// alert message
    msg: String,
    /// 全局状态
    mode_holder: ModeHolderLock,
    /// 确认事件
    confirm_event: ConfirmEvent,
    /// 上一个状态
    last_mode: Mode,
}

impl Widget for &mut Alert {
    fn render(self, _area: Rect, _buf: &mut Buffer)
    where
        Self: Sized,
    {
        // match self.mode_holder.get_mode() {
        //     Mode::Login => {}
        //     Mode::RecentChat => {}
        //     Mode::Contact => {
        //         let area = centered_rect(50, 100, area);
        //         let [_, alert_area, _] = Layout::vertical([
        //             Constraint::Fill(1),
        //             Constraint::Length(4),
        //             Constraint::Fill(1),
        //         ])
        //         .areas(area);
        //         Clear.render(alert_area, buf);
        //         let [help_area, msg_area] =
        //             Layout::vertical([Constraint::Length(1), Constraint::Length(3)])
        //                 .areas(alert_area);
        //         let (msg, style) = (
        //             vec!["Esc to quit, Enter to submit.".into()],
        //             Style::default(),
        //         );
        //         let text = Text::from(Line::from(msg)).patch_style(style);
        //         let help_message = Paragraph::new(text);
        //         help_message.render(help_area, buf);
        //         let msg = Paragraph::new(self.msg.as_str())
        //             .style(Style::default().fg(Color::Yellow))
        //             .block(Block::default().borders(Borders::ALL));
        //         msg.render(msg_area, buf);
        //     }
        // }
    }
}

impl Component for Alert {
    fn handle_key_event(&mut self, key: KeyEvent) -> color_eyre::Result<Option<Action>> {
        // match self.mode_holder.get_mode() {
        //     Mode::Alert => match key.code {
        //         KeyCode::Enter => Ok(Some(Action::Confirm(self.confirm_event.clone()))),
        //         KeyCode::Esc => {
        //             self.close();
        //             Ok(None)
        //         }
        //         _ => Ok(None),
        //     },
        //     _ => Ok(None),
        // }
        Ok(None)
    }

    fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
        // if let Action::Alert(msg, confirm_event) = action {
        //     self.msg = msg;
        //     self.confirm_event = confirm_event;
        //     self.last_mode = self.mode_holder.get_mode();
        //     self.mode_holder.set_mode(Mode::Alert);
        // }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        frame.render_widget(&mut *self, area);
        Ok(())
    }
}

impl Alert {
    fn close(&mut self) {
        self.mode_holder.set_mode(self.last_mode);
    }
}
