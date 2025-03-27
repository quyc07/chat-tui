use crate::action::Action;
use crate::components::Component;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::prelude::{Color, Style};
use ratatui::widgets::Widget;

pub(crate) struct UserInput {
    /// 当前文本框内容
    pub(crate) input: Option<String>,
    /// 用户输入类型和预加载的数据
    pub(crate) input_data: InputData,
    /// 光标在文本框的索引
    character_index: usize,
    /// 输入框光标位置
    cursor_position: Option<Position>,
    /// 是否正在编辑该文本框
    pub(crate) is_editing: bool,
}

pub(crate) enum InputData {
    UserName {
        label: Option<String>,
        data: Option<String>,
    },
    Password {
        label: Option<String>,
        data: Option<String>,
    },
    ChatMsg {
        label: Option<String>,
        data: Option<String>,
    },
}

impl InputData {
    fn set_input(&mut self, input_data: Option<String>) {
        match self {
            InputData::UserName { label: _, data } => {
                *data = input_data;
            }
            InputData::Password { label: _, data } => {
                *data = input_data;
            }
            InputData::ChatMsg { label: _, data } => {
                *data = input_data;
            }
        }
    }

    fn reset_input(&mut self) {
        match self {
            InputData::UserName { label: _, data } => {
                *data = None;
            }
            InputData::Password { label: _, data } => {
                *data = None;
            }
            InputData::ChatMsg { label: _, data } => {
                *data = None;
            }
        }
    }

    fn get_input_data(&self) -> Option<String> {
        match self {
            InputData::UserName { label: _, data } => data.clone(),
            InputData::Password { label: _, data } => data.clone(),
            InputData::ChatMsg { label: _, data } => data.clone(),
        }
    }

    pub(crate) fn label(&self) -> String {
        match self {
            InputData::UserName { label, data: _ } => label.clone().unwrap_or_default(),
            InputData::Password { label, data: _ } => label.clone().unwrap_or_default(),
            InputData::ChatMsg { label, data: _ } => label.clone().unwrap_or_default(),
        }
    }
}

impl Widget for &mut UserInput {
    fn render(self, _area: Rect, _buf: &mut Buffer)
    where
        Self: Sized,
    {
    }
}

impl Component for UserInput {
    fn handle_key_event(&mut self, key: KeyEvent) -> color_eyre::Result<Option<Action>> {
        if self.is_editing {
            match key.code {
                KeyCode::Enter => self.submit_message(),
                KeyCode::Char(to_insert) => self.enter_char(to_insert),
                KeyCode::Backspace => self.delete_char(),
                KeyCode::Left => self.move_cursor_left(),
                KeyCode::Right => self.move_cursor_right(),
                KeyCode::Esc => self.is_editing = false,
                _ => {}
            }
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, _area: Rect) -> color_eyre::Result<()> {
        if let Some(position) = self.cursor_position {
            frame.set_cursor_position(position)
        }
        Ok(())
    }
}

impl UserInput {
    pub(crate) fn data(&self) -> Option<String> {
        self.input_data.get_input_data()
    }
    pub fn new(input_data: InputData) -> Self {
        Self {
            input: None,
            input_data,
            character_index: 0,
            cursor_position: None,
            is_editing: false,
        }
    }

    pub(crate) fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.character_index.saturating_sub(1);
        self.character_index = self.clamp_cursor(cursor_moved_left);
    }

    pub(crate) fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.character_index.saturating_add(1);
        self.character_index = self.clamp_cursor(cursor_moved_right);
    }

    pub(crate) fn enter_char(&mut self, new_char: char) {
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

    pub(crate) fn delete_char(&mut self) {
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

    pub(crate) fn submit_message(&mut self) {
        self.input_data.set_input(self.input.clone());
    }

    fn reset(&mut self) {
        self.input.take();
        self.input_data.reset_input();
        self.reset_cursor();
    }

    fn cal_high(input_size: usize, area: Rect) -> u16 {
        let total_length = (input_size * 3 + 2) as u16;
        if area.height / 5 > total_length {
            area.height / 5
        } else {
            total_length
        }
    }

    pub(crate) fn set_cursor_position(&mut self, input_area: Rect) {
        self.cursor_position = Some(Position::new(
            // Draw the cursor at the current position in the input field.
            // This position is can be controlled via the left and right arrow key
            input_area.x + self.character_index as u16 + 1,
            // Move one line down, from the border to the input line
            input_area.y + 1,
        ))
    }

    pub(crate) fn select_style(&self) -> Style {
        if self.is_editing {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        }
    }
}

#[cfg(test)]
mod test {}
