use winit::event::*;

use cgmath::InnerSpace;

#[cfg_attr(rustfmt, rustfmt_skip)]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

pub struct Camera {
    pub eye: cgmath::Point3<f32>,
    pub target: cgmath::Point3<f32>,
    pub up: cgmath::Vector3<f32>,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl Camera {
    pub fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
        let view = cgmath::Matrix4::look_at(self.eye, self.target, self.up);
        let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);
        return OPENGL_TO_WGPU_MATRIX * proj * view;
    }
}

pub struct CameraController {
    pub movement_speed: f32,
    pub mouse_speed: f32,
    pub is_up_pressed: bool,
    pub is_down_pressed: bool,
    pub is_forward_pressed: bool,
    pub is_backward_pressed: bool,
    pub is_left_pressed: bool,
    pub is_right_pressend: bool,
    pub is_mouse_activated: bool,
    pub mouse_can_be_activated: bool,
    pub x_delta: f64,
    pub y_delta: f64,
    pub alt: bool,
}

impl CameraController {
    pub fn new(movement_speed: f32, mouse_speed: f32) -> Self {
        Self {
            movement_speed,
            mouse_speed,
            is_up_pressed: false,
            is_down_pressed: false,
            is_forward_pressed: false,
            is_backward_pressed: false,
            is_left_pressed: false,
            is_right_pressend: false,
            is_mouse_activated: false,
            mouse_can_be_activated: true,
            x_delta: 0.0,
            y_delta: 0.0,
            alt: false,
        }
    }

    pub fn process_events(&mut self, event: &Event<()>) -> bool {
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            state,
                            virtual_keycode: Some(keycode),
                            ..
                        },
                    ..
                } => {
                    let is_pressed = *state == ElementState::Pressed;
                    match keycode {
                        VirtualKeyCode::Space => {
                            self.is_up_pressed = is_pressed;
                            true
                        }
                        VirtualKeyCode::LShift => {
                            self.is_down_pressed = is_pressed;
                            true
                        }
                        VirtualKeyCode::W => {
                            self.is_forward_pressed = is_pressed;
                            true
                        }
                        VirtualKeyCode::S => {
                            self.is_backward_pressed = is_pressed;
                            true
                        }
                        VirtualKeyCode::A => {
                            self.is_left_pressed = is_pressed;
                            true
                        }
                        VirtualKeyCode::D => {
                            self.is_right_pressend = is_pressed;
                            true
                        }
                        VirtualKeyCode::G => {
                            if self.mouse_can_be_activated && is_pressed {
                                self.is_mouse_activated = !self.is_mouse_activated;
                                self.mouse_can_be_activated = false;
                                true
                            } else if !is_pressed {
                                self.mouse_can_be_activated = true;
                                true
                            } else {
                                false
                            }
                        }
                        _ => false,
                    }
                }
                WindowEvent::ModifiersChanged(m) => {
                    if self.alt != m.alt() {
                        self.alt = m.alt();
                        true
                    } else {
                        false
                    }
                }
                _ => false,
            },
            Event::DeviceEvent { event, .. } => match event {
                DeviceEvent::MouseMotion { delta: (x, y) } => {
                    if self.is_mouse_activated {
                        self.x_delta += x;
                        self.y_delta += y;
                        true
                    } else {
                        false
                    }
                }
                _ => false,
            },
            _ => false,
        }
    }

    pub fn update_camera(&mut self, camera: &mut Camera) {
        let forward = (camera.target - camera.eye).normalize();
        let speed = if self.alt {
            self.movement_speed * 5.0
        } else {
            self.movement_speed
        };

        if self.is_forward_pressed {
            camera.eye += forward * speed;
            camera.target += forward * speed;
        }
        if self.is_backward_pressed {
            camera.eye -= forward * speed;
            camera.target -= forward * speed;
        }

        let right = forward.cross(camera.up);

        if self.is_right_pressend {
            camera.eye += right * speed;
            camera.target += right * speed;
        }
        if self.is_left_pressed {
            camera.eye -= right * speed;
            camera.target -= right * speed;
        }

        if self.is_up_pressed {
            camera.eye += camera.up * speed;
            camera.target += camera.up * speed;
        }
        if self.is_down_pressed {
            camera.eye -= camera.up * speed;
            camera.target -= camera.up * speed;
        }

        let pitch = cgmath::Matrix3::from_axis_angle(
            right,
            cgmath::Deg(-self.y_delta as f32 * 0.01 * self.mouse_speed),
        );
        let yaw = cgmath::Matrix3::from_axis_angle(
            camera.up,
            cgmath::Deg(-self.x_delta as f32 * 0.01 * self.mouse_speed),
        );
        camera.target = camera.eye + pitch * yaw * (camera.target - camera.eye);
        //camera.up = pitch * yaw * camera.up;
        //println!("{:#?}", (self.x_delta, self.y_delta));
        self.x_delta = 0.0;
        self.y_delta = 0.0;
    }
}