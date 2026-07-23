use crate::app_state::{App, Message};
use iced::Task;
use iced::keyboard::{self, Key, key};

impl App {
    pub fn handle_keyboard_event(&mut self, event: keyboard::Event) -> Task<Message> {
        match event {
            keyboard::Event::KeyPressed { key, modifiers, .. } => match &key {
                Key::Named(key::Named::Space) => self.handle_toggle_pause(),
                Key::Named(key::Named::ArrowLeft) => self.handle_skip_back(5),
                Key::Named(key::Named::ArrowRight) => self.handle_skip_forward(5),
                Key::Named(key::Named::ArrowUp) => {
                    self.handle_arrow_key(true, modifiers.control())
                }
                Key::Named(key::Named::ArrowDown) => {
                    self.handle_arrow_key(false, modifiers.control())
                }
                Key::Named(key::Named::Enter) => self.handle_toggle_fullscreen(),
                Key::Character(c) => self.handle_character_key(c.as_str()),
                Key::Named(key::Named::Escape) => {
                    if self.fullscreen {
                        self.handle_toggle_fullscreen()
                    } else if !self.dict_word.is_empty() {
                        self.handle_close_dictionary()
                    } else {
                        Task::none()
                    }
                }
                _ => Task::none(),
            },
            _ => Task::none(),
        }
    }

    fn handle_arrow_key(&mut self, is_up: bool, ctrl_pressed: bool) -> Task<Message> {
        if ctrl_pressed {
            let s = if is_up {
                (self.speed + 0.25).min(4.0)
            } else {
                (self.speed - 0.25).max(0.25)
            };
            self.handle_set_speed(s)
        } else {
            let v = if is_up {
                (self.volume + 0.05).min(2.0)
            } else {
                (self.volume - 0.05).max(0.0)
            };
            self.handle_set_volume(v)
        }
    }

    fn handle_character_key(&mut self, c: &str) -> Task<Message> {
        match c {
            "f" | "F" => self.handle_toggle_fullscreen(),
            "m" | "M" => self.handle_toggle_mute(),
            "l" | "L" => self.handle_toggle_loop(),
            "[" => {
                let s = (self.speed - 0.25).max(0.25);
                self.handle_set_speed(s)
            }
            "]" => {
                let s = (self.speed + 0.25).min(4.0);
                self.handle_set_speed(s)
            }
            "," => self.handle_frame_step_backward(),
            "." => self.handle_frame_step_forward(),
            "o" | "O" => self.handle_open_file(),
            "s" | "S" => self.handle_load_subtitle(),
            "c" | "C" => self.handle_cycle_content_fit(),
            _ => Task::none(),
        }
    }
}
