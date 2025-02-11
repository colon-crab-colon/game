use crate::engine::core::engine::Engine;
use cgmath::{Deg, InnerSpace, Matrix3, Vector3, Zero};
use core::f32;
use crossbeam::{atomic::AtomicCell, channel::Sender};
use std::{
    collections::HashMap,
    thread::sleep,
    time::{Duration, Instant},
};
use winit::{
    event::{DeviceEvent, Event, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};

const MOVEMENT_SPEED: f32 = 0.95;
const MOVEMENT_CONTROL_MULTIPLIER: f32 = 4.0;
const X_SENSITIVITY: f32 = -0.01;
const Y_SENSITIVITY: f32 = -0.01;
const TICKS: f64 = 64.0;

#[derive(Clone, Copy)]
pub enum InputHandler {
    Game,
    Gui,
}

pub struct EventHandler {
    engine: &'static Engine<'static>,
    handler: &'static AtomicCell<InputHandler>,
    keymap: HashMap<KeyCode, bool>,
    events: Sender<WindowEvent>,
}

impl EventHandler {
    pub fn new(
        engine: &'static Engine,
        handler: &'static AtomicCell<InputHandler>,
        sender: Sender<WindowEvent>,
    ) -> Self {
        let keys = &[
            KeyCode::KeyW,
            KeyCode::KeyA,
            KeyCode::KeyS,
            KeyCode::KeyD,
            KeyCode::Space,
            KeyCode::ShiftLeft,
            KeyCode::ControlLeft,
        ];

        Self {
            engine,
            handler,
            keymap: HashMap::from_iter(keys.iter().map(|k| (*k, false))),
            events: sender,
        }
    }

    pub fn handle(&mut self) {
        let events = self.engine.events();

        let id = self.engine.window().id();

        let duration = Duration::from_secs_f64(1.0 / TICKS);

        loop {
            let start = Instant::now();

            if self.engine.exited() {
                break;
            }

            while let Ok(event) = events.try_recv() {
                match event {
                    Event::WindowEvent { window_id, event } => {
                        if window_id == id {
                            self.handle_window_event(event);
                        }
                    }
                    Event::DeviceEvent {
                        device_id: _,
                        event,
                    } => {
                        self.handle_device_event(event);
                    }
                    _ => {}
                }
            }

            let mut offset: Vector3<f32> = Vector3::zero();

            let multiplier = match self.keymap.get(&KeyCode::ControlLeft).unwrap() {
                true => MOVEMENT_CONTROL_MULTIPLIER,
                false => 1.0,
            };

            if *self.keymap.get(&KeyCode::KeyW).unwrap() {
                offset.z += 1.0 * multiplier;
            }

            if *self.keymap.get(&KeyCode::KeyS).unwrap() {
                offset.z -= 1.0 * multiplier;
            }

            if *self.keymap.get(&KeyCode::KeyA).unwrap() {
                offset.x -= 1.0 * multiplier;
            }

            if *self.keymap.get(&KeyCode::KeyD).unwrap() {
                offset.x += 1.0 * multiplier;
            }

            if *self.keymap.get(&KeyCode::Space).unwrap() {
                offset.y += 1.0 * multiplier;
            }

            if *self.keymap.get(&KeyCode::ShiftLeft).unwrap() {
                offset.y -= 1.0 * multiplier;
            }

            let eye = self.engine.camera().get_eye();
            let look_at = self.engine.camera().get_look_at();

            let direction = (look_at - eye).normalize();

            let right = direction.cross(self.engine.camera().up()).normalize();

            self.engine.camera().set_eye_no_update(
                eye + (direction * offset.z * MOVEMENT_SPEED)
                    + (right * offset.x * MOVEMENT_SPEED)
                    + (self.engine.camera().up() * offset.y * MOVEMENT_SPEED),
            );
            self.engine.camera().set_look_at_no_update(
                look_at
                    + (direction * offset.z * MOVEMENT_SPEED)
                    + (right * offset.x * MOVEMENT_SPEED)
                    + (self.engine.camera().up() * offset.y * MOVEMENT_SPEED),
            );
            self.engine.camera().update();

            let elapsed = start.elapsed();

            if elapsed < duration {
                sleep(duration - elapsed);
            }
        }
    }

    pub fn handle_device_event(&mut self, event: DeviceEvent) {
        if let DeviceEvent::MouseMotion {
            delta: (delta_x, delta_y),
        } = event
        {
            let eye = self.engine.camera().get_eye();
            let look_at = self.engine.camera().get_look_at();

            let mut relative = look_at - eye;

            let rotation = Matrix3::from_angle_y(Deg(delta_x as f32 * X_SENSITIVITY));

            relative = rotation * relative;

            self.engine.camera().set_look_at_no_update(eye + relative);

            let eye = self.engine.camera().get_eye();
            let look_at = self.engine.camera().get_look_at();

            let mut relative = (look_at - eye).normalize();

            let right = relative.cross(Vector3::unit_y()).normalize();

            let rotation = Matrix3::from_axis_angle(right, Deg(delta_y as f32 * Y_SENSITIVITY));

            relative = rotation * relative;

            self.engine.camera().set_look_at_no_update(eye + relative);

            self.engine.camera().update();
        };
    }

    pub fn handle_window_event(&mut self, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                self.engine.exit();
            }
            WindowEvent::Resized(size) => {
                self.engine.renderer().resize(size.width, size.height);
            }
            _ => {}
        }

        self.events.send(event.clone()).unwrap();

        if matches!(self.handler.load(), InputHandler::Gui) {
            self.engine
                .ui_renderer()
                .handle_window_event(self.engine.window().window(), &event);
            return;
        }

        if let WindowEvent::KeyboardInput {
            device_id: _,
            event,
            is_synthetic: _,
        } = event
        {
            /*
            if let PhysicalKey::Code(KeyCode::Escape) = event.physical_key {
                if !event.state.is_pressed() {
                    self.on_escape();
                }
                return;
            }


            // option menu is open
            if self.options_ui_guard.is_some() {
                return;
            }
            */

            if let PhysicalKey::Code(code) = event.physical_key {
                if let Some(state) = self.keymap.get_mut(&code) {
                    *state = event.state.is_pressed();
                }
            }
        }
    }

    /*
    fn on_escape(&mut self) {
        let window = self.engine.window().window();

        if let Some(guard) = self.options_ui_guard.take() {
            drop(guard);

            window.set_cursor_visible(false);
            window
                .set_cursor_grab(CursorGrabMode::Confined)
                .or_else(|_| window.set_cursor_grab(CursorGrabMode::Locked))
                .unwrap();

            return;
        }

        window.set_cursor_visible(true);
        window.set_cursor_grab(CursorGrabMode::None).unwrap();

        self.options_ui_guard = Some(self.engine.ui_renderer().add_static_ui(|ctx| {
            let screen_rect = ctx.screen_rect();

            egui::Area::new("pause_menu".into())
                .fixed_pos(screen_rect.center())
                .sense(Sense::click())
                .show(ctx, |ui| {
                    egui::Frame::new()
                        .fill(Color32::from_rgba_unmultiplied(0, 0, 0, 230))
                        .show(ui, |ui| {
                            ui.set_min_size(screen_rect.size());

                            ui.add_space(screen_rect.height() / 2. - 60. / 2.);

                            ui.vertical_centered(|ui| {
                                let button = ui.add_sized(
                                    [140., 60.],
                                    egui::Button::new(RichText::new("Exit").size(24.)),
                                );
                                if button.clicked() {
                                    self.engine.exit();
                                }
                            });
                        })
                });
        }))
    }
    */
}
