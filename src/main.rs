pub mod camera;
pub mod instance;
pub mod model;
pub mod state;
pub mod texture;
pub mod uniforms;
pub mod boids;
pub mod point_cloud;

use state::*;

use futures::executor::block_on;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

fn main() {
    println!("{}", wgpu::DeviceDescriptor::default().limits.max_bind_groups);

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut state = block_on(State::new(&window));

    event_loop.run(move |event_o, _, control_flow| match event_o {
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == window.id() => {
            if !state.input(&event_o) {
                match event {
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    WindowEvent::KeyboardInput { input, .. } => match input {
                        KeyboardInput {
                            state: ElementState::Pressed,
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            ..
                        } => *control_flow = ControlFlow::Exit,
                        _ => {}
                    },
                    WindowEvent::Resized(physical_size) => {
                        state.resize(*physical_size);
                    }
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        state.resize(**new_inner_size);
                    }
                    _ => {}
                }
            }
        }
        Event::RedrawRequested(_) => {
            state.update();
            state.render();
        }
        Event::MainEventsCleared => {
            window.request_redraw();
            window
                .set_cursor_grab(state.camera_controller.is_mouse_activated)
                .unwrap();
            window.set_cursor_visible(!state.camera_controller.is_mouse_activated);
        }
        Event::DeviceEvent { .. } => {
            state.input(&event_o);
        }
        _ => {}
    });
}
