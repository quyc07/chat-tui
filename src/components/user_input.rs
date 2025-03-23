use crate::action::Action;
use crate::app::{Mode, ModeHolderLock};
use crate::components::Component;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::widgets::Widget;
use ratatui::Frame;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

pub struct UserInput {
    /// Current value of the input box
    input: Option<String>,
    /// 加载组件时带入的用户输入 TODO 用户输入类型和数据
    input_data: Option<InputData>,
    /// Position of cursor in the editor area.
    character_index: usize,
    /// 
    input_rx: UnboundedReceiver<InputData>,
    /// 答案
    input_tx: UnboundedSender<InputData>,
    /// 全局状态
    mode_holder: ModeHolderLock,
    /// 输入框光标位置
    cursor_position: Option<Position>,
}

enum InputData {
    UserName(Option<String>),
    Password(Option<String>),
    ChatMsg(Option<String>),
}

impl InputData {
    pub fn set_input(&mut self, data: Option<String>) {
        match self {
            InputData::UserName(_) => {
                *self = InputData::UserName(data);
            }
            InputData::Password(_) => {
                *self = InputData::Password(data);
            }
            InputData::ChatMsg(_) => {
                *self = InputData::ChatMsg(data);
            }
        }
    }
}

impl Widget for &mut UserInput {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        match self.mode_holder.get_mode() {
            Mode::Login => {}

            Mode::Input => {}
            _ => {}
        }
    }
}

impl Component for UserInput {
    fn handle_key_event(&mut self, key: KeyEvent) -> color_eyre::Result<Option<Action>> {
        if self.mode_holder.get_mode() == Mode::Input {
            match key.code {
                KeyCode::Enter => self.submit_message(),
                KeyCode::Char(to_insert) => self.enter_char(to_insert),
                KeyCode::Backspace => self.delete_char(),
                KeyCode::Left => self.move_cursor_left(),
                KeyCode::Right => self.move_cursor_right(),
                KeyCode::Esc => self.close(),
                _ => {}
            }
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        frame.render_widget(&mut *self, area);
        if let Some(position) = self.cursor_position {
            frame.set_cursor_position(position)
        }
        Ok(())
    }
}

impl UserInput {
    pub fn new() -> Self {
        todo!()
    }

    fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.character_index.saturating_sub(1);
        self.character_index = self.clamp_cursor(cursor_moved_left);
    }

    fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.character_index.saturating_add(1);
        self.character_index = self.clamp_cursor(cursor_moved_right);
    }

    fn enter_char(&mut self, new_char: char) {
        let index = self.byte_index();
        let mut input = self.current_input();
        input.insert(index, new_char);
        self.set_current_input(input);
        self.move_cursor_right();
    }

    /// Returns the byte index based on the character position.
    ///
    /// Since each character in a string can be contain multiple bytes, it's necessary to calculate
    /// the byte index based on the index of the character.
    fn byte_index(&self) -> usize {
        self.current_input()
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.character_index)
            .unwrap_or(self.current_input().len())
    }

    fn delete_char(&mut self) {
        let is_not_cursor_leftmost = self.character_index != 0;
        if is_not_cursor_leftmost {
            // Method "remove" is not used on the saved text for deleting the selected char.
            // Reason: Using remove on String works on bytes instead of the chars.
            // Using remove would require special care because of char boundaries.

            let current_index = self.character_index;
            let from_left_to_current_index = current_index - 1;

            // Getting all characters before the selected character.
            let input = self.current_input();
            let before_char_to_delete = input.chars().take(from_left_to_current_index);
            // Getting all characters after selected character.
            let after_char_to_delete = input.chars().skip(current_index);

            // Put all characters together except the selected one.
            // By leaving the selected one out, it is forgotten and therefore deleted.
            let input = before_char_to_delete.chain(after_char_to_delete).collect();
            self.set_current_input(input);
            self.move_cursor_left();
        }
    }

    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.current_input().chars().count())
    }

    fn reset_cursor(&mut self) {
        self.character_index = 0;
        self.cursor_position = None;
    }

    fn current_input(&self) -> String {
        self.input.clone().unwrap_or_default()
    }

    fn set_current_input(&mut self, input: String) {
        self.input = Some(input);
    }

    fn submit_message(&mut self) {
        let mut input_data = self.input_data.take().unwrap();
        input_data.set_input(self.input.clone());
        self.input_tx.send(input_data).unwrap();
        self.reset()
    }

    fn reset(&mut self) {
        self.input.take();
        self.input_data.take();
        self.reset_cursor();
    }

    fn close(&mut self) {
        self.input_tx.send(self.input_data.take().unwrap()).unwrap();
        self.reset()
    }

    fn cal_high(input_size: usize, area: Rect) -> u16 {
        let total_length = (input_size * 3 + 2) as u16;
        if area.height / 5 > total_length {
            area.height / 5
        } else {
            total_length
        }
    }

    fn set_cursor_position(&mut self, input_area: Rect) {
        self.cursor_position = Some(Position::new(
            // Draw the cursor at the current position in the input field.
            // This position is can be controlled via the left and right arrow key
            input_area.x + self.character_index as u16 + 1,
            // Move one line down, from the border to the input line
            input_area.y + 1,
        ))
    }
}

#[cfg(test)]
mod test {}
