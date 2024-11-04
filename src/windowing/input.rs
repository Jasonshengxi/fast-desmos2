use glam::Vec2;
use std::collections::{HashMap, HashSet};
use winit::event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

#[derive(Debug, Default)]
pub struct InputTracker {
    mouse_pos: Vec2,
    wheel_delta: Vec2,

    last_polled_mouse: Vec2,

    held_keys: HashSet<KeyCode>,
    held_buttons: HashSet<MouseButton>,

    key_changes: HashMap<KeyCode, ElementState>,
    button_changes: HashMap<MouseButton, ElementState>,

    polled: bool,
}

impl InputTracker {
    pub fn poll(&mut self) {
        self.polled = true;
    }

    pub fn mouse_pos(&self) -> Vec2 {
        assert!(self.polled);
        self.mouse_pos
    }

    pub fn mouse_delta(&self) -> Vec2 {
        assert!(self.polled);
        self.mouse_pos - self.last_polled_mouse
    }

    pub fn gpu_mouse_delta(&self) -> Vec2 {
        let x = self.mouse_delta();
        Vec2::new(x.x, -x.y) * 2.0
    }

    pub fn wheel_delta(&self) -> Vec2 {
        assert!(self.polled);
        self.wheel_delta
    }

    pub fn is_key_down(&self, key_code: KeyCode) -> bool {
        assert!(self.polled);
        self.held_keys.contains(&key_code)
    }
    pub fn is_button_down(&self, mouse_button: MouseButton) -> bool {
        assert!(self.polled);
        self.held_buttons.contains(&mouse_button)
    }

    pub fn key_changes(&self, key_code: KeyCode) -> Option<ElementState> {
        assert!(self.polled);
        self.key_changes.get(&key_code).copied()
    }
    pub fn button_changes(&self, mouse_button: MouseButton) -> Option<ElementState> {
        assert!(self.polled);
        self.button_changes.get(&mouse_button).copied()
    }
    pub fn is_key_pressed(&self, key_code: KeyCode) -> bool {
        self.key_changes(key_code) == Some(ElementState::Pressed)
    }
    pub fn is_button_pressed(&self, mouse_button: MouseButton) -> bool {
        self.button_changes(mouse_button) == Some(ElementState::Pressed)
    }
    pub fn is_key_released(&self, key_code: KeyCode) -> bool {
        self.key_changes(key_code) == Some(ElementState::Released)
    }
    pub fn is_button_released(&self, mouse_button: MouseButton) -> bool {
        self.button_changes(mouse_button) == Some(ElementState::Released)
    }

    pub fn process_event(&mut self, event: &WindowEvent) {
        if self.polled {
            self.polled = false;
            self.key_changes.clear();
            self.button_changes.clear();
            self.last_polled_mouse = self.mouse_pos;
            self.wheel_delta = Vec2::ZERO;
        }

        match *event {
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_pos = Vec2::new(position.x as f32, position.y as f32);
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state,
                        ..
                    },
                ..
            } => {
                match state {
                    ElementState::Pressed => self.held_keys.insert(code),
                    ElementState::Released => self.held_keys.remove(&code),
                };
                self.key_changes.insert(code, state);
            }
            WindowEvent::MouseInput { button, state, .. } => {
                match state {
                    ElementState::Pressed => self.held_buttons.insert(button),
                    ElementState::Released => self.held_buttons.remove(&button),
                };
                self.button_changes.insert(button, state);
            }
            WindowEvent::MouseWheel { delta, .. } => match delta {
                MouseScrollDelta::LineDelta(x, y) => {
                    self.wheel_delta += Vec2::new(x, y) * 14.0;
                }
                MouseScrollDelta::PixelDelta(by) => {
                    self.wheel_delta += Vec2::new(by.x as f32, by.y as f32)
                }
            },
            _ => {}
        }
    }
}
