use cgmath::{SquareMatrix, InnerSpace};
use futures::executor::block_on;
use image::GenericImageView;
use include_glsl::include_glsl;
use std::mem;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

mod texture;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct Vertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
}

impl Vertex {
    const VERTEX_ATTRS: [wgpu::VertexAttributeDescriptor; 2] =
        wgpu::vertex_attr_array![0 => Float3, 1 => Float2];
    const VERTEX_BUFF_DESC: wgpu::VertexBufferDescriptor<'static> = wgpu::VertexBufferDescriptor {
        stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
        step_mode: wgpu::InputStepMode::Vertex,
        attributes: &Self::VERTEX_ATTRS,
    };
}

unsafe impl bytemuck::Pod for Vertex {}
unsafe impl bytemuck::Zeroable for Vertex {}

const VERTICES: &[Vertex] = &[
    Vertex {
        position: [-0.0868241, 0.49240386, 0.0],
        tex_coords: [0.4131759, 0.00759614],
    }, // A
    Vertex {
        position: [-0.49513406, 0.06958647, 0.0],
        tex_coords: [0.0048659444, 0.43041354],
    }, // B
    Vertex {
        position: [-0.21918549, -0.44939706, 0.0],
        tex_coords: [0.28081453, 0.949397057],
    }, // C
    Vertex {
        position: [0.35966998, -0.3473291, 0.0],
        tex_coords: [0.85967, 0.84732911],
    }, // D
    Vertex {
        position: [0.44147372, 0.2347359, 0.0],
        tex_coords: [0.9414737, 0.2652641],
    }, // E
];

const INDICES: &[u16] = &[0, 1, 4, 1, 2, 4, 2, 3, 4];

#[cfg_attr(rustfmt, rustfmt_skip)]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

struct Camera {
    eye: cgmath::Point3<f32>,
    target: cgmath::Point3<f32>,
    up: cgmath::Vector3<f32>,
    aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32,
}

impl Camera {
    fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
        let view = cgmath::Matrix4::look_at(self.eye, self.target, self.up);
        let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);
        return OPENGL_TO_WGPU_MATRIX * proj * view;
    }
}

struct CameraController {
    speed: f32,
    is_up_pressed: bool,
    is_down_pressed: bool,
    is_forward_pressed: bool,
    is_backward_pressed: bool,
    is_left_pressed: bool,
    is_right_pressend: bool,
    is_mouse_activated: bool,
    mouse_can_be_activated: bool,
    x_delta: f64,
    y_delta: f64,
}

impl CameraController {
    fn new(speed: f32) -> Self {
        Self {
            speed,
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
        }
    }

    fn process_events(&mut self, event: &Event<()>) -> bool {
        match event {
            Event::WindowEvent{
                event,
                ..
            } => match event {
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
                            if self.mouse_can_be_activated && is_pressed{
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
                },
                _ => false,
            },
            Event::DeviceEvent{
                event,
                ..
            } => match event {
                DeviceEvent::MouseMotion{
                    delta: (x, y)
                } => {
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
            _=> false,
        }
    }

    fn update_camera(&mut self, camera: &mut Camera) {
        let forward = (camera.target-camera.eye).normalize();

        if self.is_forward_pressed {
            camera.eye +=forward * self.speed;
            camera.target +=forward * self.speed;
        }
        if self.is_backward_pressed {
            camera.eye -= forward * self.speed;
            camera.target -= forward * self.speed;
        }

        let right = forward.cross(camera.up);
        
        if self.is_right_pressend {
            camera.eye +=right * self.speed;
            camera.target +=right * self.speed;
        }
        if self.is_left_pressed {
            camera.eye -=right * self.speed;
            camera.target -=right * self.speed;
        }

        if self.is_up_pressed {
            camera.eye +=camera.up * self.speed;
            camera.target +=camera.up * self.speed;
        }
        if self.is_down_pressed {
            camera.eye -=camera.up * self.speed;
            camera.target -=camera.up * self.speed;
        }


        let pitch = cgmath::Matrix3::from_axis_angle(right, cgmath::Deg(-self.y_delta as f32 /  5.0));
        let yaw = cgmath::Matrix3::from_axis_angle(camera.up, cgmath::Deg(-self.x_delta as f32 / 5.0));
        camera.target = camera.eye + pitch * yaw * (camera.target-camera.eye);
        camera.up = pitch* yaw * camera.up;
        //println!("{:#?}", (self.x_delta, self.y_delta));
        self.x_delta = 0.0;
        self.y_delta = 0.0;

    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct Uniforms {
    view_proj: cgmath::Matrix4<f32>,
}

unsafe impl bytemuck::Pod for Uniforms {}
unsafe impl bytemuck::Zeroable for Uniforms {}

impl Uniforms {
    fn new() -> Uniforms {
        Self {
            view_proj: cgmath::Matrix4::identity(),
        }
    }

    fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.build_view_projection_matrix();
    }
}

struct State {
    surface: wgpu::Surface,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,

    default_render_pipeline: wgpu::RenderPipeline,
    default: bool,

    vertex_buffer: wgpu::Buffer,
    num_vertices: u32,
    index_buffer: wgpu::Buffer,
    num_indices: u32,

    texture: texture::Texture,
    texture_bind_group: wgpu::BindGroup,

    uniforms: Uniforms,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,

    camera: Camera,
    camera_controller: CameraController,

    size: winit::dpi::PhysicalSize<u32>,

    clear_color: wgpu::Color,
}

impl State {
    async fn new(window: &Window) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance
            .request_adapter(
                &wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::Default,
                    compatible_surface: Some(&surface),
                },
                wgpu::UnsafeExtensions::disallow(),
            )
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    extensions: wgpu::Extensions::default(),
                    limits: Default::default(),
                    shader_validation: true,
                },
                None,
            )
            .await
            .unwrap();

        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        let swap_chain = device.create_swap_chain(&surface, &sc_desc);

        let clear_color = wgpu::Color {
            r: 0.1,
            g: 0.2,
            b: 0.3,
            a: 1.0,
        };

        let camera = Camera {
            eye: (0.0, 1.0, 2.0).into(),
            target: (0.0, 0.0, 0.0).into(),
            up: cgmath::Vector3::unit_y(),
            aspect: sc_desc.width as f32 / sc_desc.height as f32,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
        };

        let camera_controller = CameraController::new(0.2);

        let mut uniforms = Uniforms::new();
        uniforms.update_view_proj(&camera);

        let uniform_buffer = device.create_buffer_with_data(
            bytemuck::cast_slice(&[uniforms]),
            wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        );

        let vs = Self::create_shader_module(&device, include_glsl!("../shaders/shader.vert"));
        let fs = Self::create_shader_module(&device, include_glsl!("../shaders/shader.frag"));

        let vertex_buffer = device
            .create_buffer_with_data(bytemuck::cast_slice(VERTICES), wgpu::BufferUsage::VERTEX);
        let num_vertices = VERTICES.len() as u32;

        let index_buffer =
            device.create_buffer_with_data(bytemuck::cast_slice(INDICES), wgpu::BufferUsage::INDEX);
        let num_indices = INDICES.len() as u32;

        let (texture, cmd_buffer) = texture::Texture::from_bytes(
            &device,
            include_bytes!("../assets/textures/happy-tree.png"),
            "happy-tree.png",
        )
        .unwrap();

        queue.submit(Some(cmd_buffer));

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("texture_bind_group_layout"),
                bindings: &[
                    wgpu::BindGroupLayoutEntry::new(
                        0,
                        wgpu::ShaderStage::FRAGMENT,
                        wgpu::BindingType::SampledTexture {
                            dimension: wgpu::TextureViewDimension::D2,
                            component_type: wgpu::TextureComponentType::Uint,
                            multisampled: false,
                        },
                    ),
                    wgpu::BindGroupLayoutEntry::new(
                        1,
                        wgpu::ShaderStage::FRAGMENT,
                        wgpu::BindingType::Sampler { comparison: false },
                    ),
                ],
            });

        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            bindings: &[
                wgpu::Binding {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::Binding {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
            ],
            label: Some("texture_bind_group"),
        });

        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("uniform_bind_group_layout"),
                bindings: &[wgpu::BindGroupLayoutEntry::new(
                    0,
                    wgpu::ShaderStage::VERTEX,
                    wgpu::BindingType::UniformBuffer {
                        dynamic: false,
                        min_binding_size: None,
                    },
                )],
            });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            bindings: &[wgpu::Binding {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(uniform_buffer.slice(..)),
            }],
            label: Some("uniform_bind_group"),
        });

        let default_render_pipeline = Self::create_default_render_pipeline(
            &device,
            &[&texture_bind_group_layout, &uniform_bind_group_layout],
            sc_desc.format,
            &vs,
            &fs,
            wgpu::PrimitiveTopology::TriangleList,
            &[Vertex::VERTEX_BUFF_DESC],
        );

        Self {
            surface,
            adapter,
            device,
            queue,
            sc_desc,
            swap_chain,
            size,
            clear_color,
            default_render_pipeline,
            default: true,
            vertex_buffer,
            num_vertices,
            index_buffer,
            num_indices,
            texture,
            texture_bind_group,
            camera,
            camera_controller,
            uniforms,
            uniform_buffer,
            uniform_bind_group,
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        self.sc_desc.width = new_size.width;
        self.sc_desc.height = new_size.height;
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
    }

    fn input(&mut self, event: &Event<()>) -> bool {
        self.camera_controller.process_events(event)
    }

    fn update(&mut self) {
        self.camera_controller.update_camera(&mut self.camera);
        self.uniforms.update_view_proj(&self.camera);

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("update encoder")
        });

        let staging_buffer = self.device.create_buffer_with_data(bytemuck::cast_slice(&[self.uniforms]), wgpu::BufferUsage::COPY_SRC);

        encoder.copy_buffer_to_buffer(&staging_buffer, 0, &self.uniform_buffer, 0, std::mem::size_of::<Uniforms>() as wgpu::BufferAddress);

        self.queue.submit(Some(encoder.finish()));
    }

    fn render(&mut self) {
        let frame = self
            .swap_chain
            .get_next_frame()
            .expect("Timeout getting texture")
            .output;

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: &frame.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear_color),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });
            render_pass.set_pipeline(if self.default {
                &self.default_render_pipeline
            } else {
                &self.default_render_pipeline
            });
            render_pass.set_bind_group(0, &self.texture_bind_group, &[]);
            render_pass.set_bind_group(1, &self.uniform_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..));
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        }

        self.queue.submit(Some(encoder.finish()));
    }

    fn create_vertex_buffer_descriptor<'a>(
        stride: wgpu::BufferAddress,
        instanced: bool,
        attributes: &'a [wgpu::VertexAttributeDescriptor],
    ) -> wgpu::VertexBufferDescriptor<'a> {
        wgpu::VertexBufferDescriptor {
            stride: stride,
            step_mode: if instanced {
                wgpu::InputStepMode::Instance
            } else {
                wgpu::InputStepMode::Vertex
            },
            attributes: attributes,
        }
    }

    fn create_shader_module(device: &wgpu::Device, code: &[u32]) -> wgpu::ShaderModule {
        return device.create_shader_module(wgpu::util::make_spirv(bytemuck::cast_slice(code)));
    }

    fn create_pipeline_layout(
        device: &wgpu::Device,
        bind_group_layouts: &[&wgpu::BindGroupLayout],
    ) -> wgpu::PipelineLayout {
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: bind_group_layouts,
        })
    }

    fn create_color_state_descriptor(format: wgpu::TextureFormat) -> wgpu::ColorStateDescriptor {
        wgpu::ColorStateDescriptor {
            format: format,
            alpha_blend: wgpu::BlendDescriptor::REPLACE,
            color_blend: wgpu::BlendDescriptor::REPLACE,
            write_mask: wgpu::ColorWrite::ALL,
        }
    }

    fn create_render_pipeline_descriptor<'a>(
        pipeline_layout: &'a wgpu::PipelineLayout,
        vs: &'a wgpu::ShaderModule,
        fs: &'a wgpu::ShaderModule,
        topology: wgpu::PrimitiveTopology,
        color_states: &'a [wgpu::ColorStateDescriptor],
        vertex_buffers: &'a [wgpu::VertexBufferDescriptor],
    ) -> wgpu::RenderPipelineDescriptor<'a> {
        wgpu::RenderPipelineDescriptor {
            layout: pipeline_layout,
            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: vs,
                entry_point: "main",
            },
            fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                module: fs,
                entry_point: "main",
            }),
            rasterization_state: Some(wgpu::RasterizationStateDescriptor {
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: wgpu::CullMode::None,
                depth_bias: 0,
                depth_bias_slope_scale: 0.0,
                depth_bias_clamp: 0.0,
            }),
            primitive_topology: topology,
            color_states: color_states,
            depth_stencil_state: None,
            vertex_state: wgpu::VertexStateDescriptor {
                index_format: wgpu::IndexFormat::Uint16,
                vertex_buffers: vertex_buffers,
            },
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        }
    }

    fn create_default_render_pipeline(
        device: &wgpu::Device,
        bind_group_layouts: &[&wgpu::BindGroupLayout],
        format: wgpu::TextureFormat,
        vs: &wgpu::ShaderModule,
        fs: &wgpu::ShaderModule,
        topology: wgpu::PrimitiveTopology,
        vertex_buffers: &[wgpu::VertexBufferDescriptor],
    ) -> wgpu::RenderPipeline {
        let layout = Self::create_pipeline_layout(device, bind_group_layouts);
        let color_states = [Self::create_color_state_descriptor(format)];
        let descriptor = Self::create_render_pipeline_descriptor(
            &layout,
            vs,
            fs,
            topology,
            &color_states,
            vertex_buffers,
        );
        device.create_render_pipeline(&descriptor)
    }
}

fn main() {
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
            window.set_cursor_grab(state.camera_controller.is_mouse_activated);
            window.set_cursor_visible(!state.camera_controller.is_mouse_activated);

        },
        Event::DeviceEvent{..} => {state.input(&event_o);},
        _ => {}
    });
}
