use super::camera::*;
use super::instance::*;
use super::texture::*;
use super::uniforms::Uniforms;
use super::vertex::*;
use cgmath::InnerSpace;
use cgmath::Rotation3;
use cgmath::Zero;
use include_glsl::include_glsl;
use winit::{event::*, window::Window};

pub struct State {
    pub surface: wgpu::Surface,
    pub adapter: wgpu::Adapter,
    pub size: winit::dpi::PhysicalSize<u32>,

    pub device: wgpu::Device,
    pub queue: wgpu::Queue,

    pub swap_chain: wgpu::SwapChain,
    pub sc_desc: wgpu::SwapChainDescriptor,

    pub vertex_buffer: wgpu::Buffer,
    pub num_vertices: u32,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,

    pub instances: Vec<Instance>,
    pub instance_buffer: wgpu::Buffer,

    pub texture: Texture,
    pub texture_bind_group: wgpu::BindGroup,

    pub camera: Camera,
    pub camera_controller: CameraController,

    pub uniforms: Uniforms,
    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,

    pub default_render_pipeline: wgpu::RenderPipeline,
    pub default: bool,
    pub clear_color: wgpu::Color,
}

impl State {
    pub async fn new(window: &Window) -> Self {
        let (size, surface, adapter) = Self::setup_adapter(window).await;

        let (device, queue) = Self::setup_device(&adapter).await;

        let (swap_chain, sc_desc) = Self::setup_swapchain(&device, &size, &surface);

        let (vertex_buffer, num_vertices, index_buffer, num_indices) =
            Self::setup_vertex_input(&device);

        let (instances, instance_buffer) = Self::setup_instances(&device);

        let (texture, texture_bind_group_layout, texture_bind_group) =
            Self::setup_texture(&device, &queue);

        let (camera, camera_controller) = Self::setup_camera(&sc_desc);

        let (uniforms, uniform_buffer, uniform_bind_group_layout, uniform_bind_group) =
            Self::setup_uniforms(&device, &camera);
        let (vs, fs) = Self::setup_shader(&device);

        let default_render_pipeline = Self::setup_default_render_pipeline(
            &device,
            &[&texture_bind_group_layout, &uniform_bind_group_layout],
            sc_desc.format,
            &vs,
            &fs,
            wgpu::PrimitiveTopology::TriangleList,
            &[Vertex::desc(), InstanceRaw::desc()],
        );
        let clear_color = wgpu::Color {
            r: 0.1,
            g: 0.2,
            b: 0.3,
            a: 1.0,
        };

        Self {
            surface,
            adapter,
            size,
            device,
            queue,
            swap_chain,
            sc_desc,
            vertex_buffer,
            num_vertices,
            index_buffer,
            num_indices,
            instances,
            instance_buffer,
            texture,
            texture_bind_group,
            camera,
            camera_controller,
            uniforms,
            uniform_buffer,
            uniform_bind_group,
            default_render_pipeline,
            default: true,
            clear_color,
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        self.sc_desc.width = new_size.width;
        self.sc_desc.height = new_size.height;
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
    }

    pub fn input(&mut self, event: &Event<()>) -> bool {
        self.camera_controller.process_events(event)
    }

    pub fn update(&mut self) {
        self.camera_controller.update_camera(&mut self.camera);
        self.uniforms.update_view_proj(&self.camera);

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("update encoder"),
            });

        let staging_buffer = self.device.create_buffer_with_data(
            bytemuck::cast_slice(&[self.uniforms]),
            wgpu::BufferUsage::COPY_SRC,
        );

        encoder.copy_buffer_to_buffer(
            &staging_buffer,
            0,
            &self.uniform_buffer,
            0,
            std::mem::size_of::<Uniforms>() as wgpu::BufferAddress,
        );

        self.queue.submit(Some(encoder.finish()));
    }

    pub fn render(&mut self) {
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
            render_pass.set_pipeline(&self.default_render_pipeline);
            render_pass.set_bind_group(0, &self.texture_bind_group, &[]);
            render_pass.set_bind_group(1, &self.uniform_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..));
            render_pass.draw_indexed(0..self.num_indices, 0, 0..Instance::NUM_INSTANCES);
        }

        self.queue.submit(Some(encoder.finish()));
    }

    async fn setup_adapter(
        window: &Window,
    ) -> (winit::dpi::PhysicalSize<u32>, wgpu::Surface, wgpu::Adapter) {
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

        (size, surface, adapter)
    }

    async fn setup_device(adapter: &wgpu::Adapter) -> (wgpu::Device, wgpu::Queue) {
        adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    extensions: wgpu::Extensions::default(),
                    limits: Default::default(),
                    shader_validation: true,
                },
                None,
            )
            .await
            .unwrap()
    }

    fn setup_swapchain(
        device: &wgpu::Device,
        size: &winit::dpi::PhysicalSize<u32>,
        surface: &wgpu::Surface,
    ) -> (wgpu::SwapChain, wgpu::SwapChainDescriptor) {
        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        let swap_chain = device.create_swap_chain(&surface, &sc_desc);

        (swap_chain, sc_desc)
    }

    fn setup_camera(sc_desc: &wgpu::SwapChainDescriptor) -> (Camera, CameraController) {
        let camera = Camera {
            eye: (0.0, 1.0, 2.0).into(),
            target: (0.0, 0.0, 0.0).into(),
            up: cgmath::Vector3::unit_y(),
            aspect: sc_desc.width as f32 / sc_desc.height as f32,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
        };

        let camera_controller = CameraController::new(0.05, 10.0);

        (camera, camera_controller)
    }

    fn setup_uniforms(
        device: &wgpu::Device,
        camera: &Camera,
    ) -> (
        Uniforms,
        wgpu::Buffer,
        wgpu::BindGroupLayout,
        wgpu::BindGroup,
    ) {
        let mut uniforms = Uniforms::new();
        uniforms.update_view_proj(&camera);

        let uniform_buffer = device.create_buffer_with_data(
            bytemuck::cast_slice(&[uniforms]),
            wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        );

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

        (
            uniforms,
            uniform_buffer,
            uniform_bind_group_layout,
            uniform_bind_group,
        )
    }

    fn setup_shader(device: &wgpu::Device) -> (wgpu::ShaderModule, wgpu::ShaderModule) {
        let vs = Self::create_shader_module(device, include_glsl!("../shaders/shader.vert"));
        let fs = Self::create_shader_module(device, include_glsl!("../shaders/shader.frag"));

        (vs, fs)
    }

    fn setup_vertex_input(device: &wgpu::Device) -> (wgpu::Buffer, u32, wgpu::Buffer, u32) {
        let vertex_buffer = device
            .create_buffer_with_data(bytemuck::cast_slice(VERTICES), wgpu::BufferUsage::VERTEX);
        let num_vertices = VERTICES.len() as u32;

        let index_buffer =
            device.create_buffer_with_data(bytemuck::cast_slice(INDICES), wgpu::BufferUsage::INDEX);
        let num_indices = INDICES.len() as u32;

        (vertex_buffer, num_vertices, index_buffer, num_indices)
    }

    fn setup_texture(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> (Texture, wgpu::BindGroupLayout, wgpu::BindGroup) {
        let (texture, cmd_buffer) = Texture::from_bytes(
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

        (texture, texture_bind_group_layout, texture_bind_group)
    }

    fn setup_default_render_pipeline(
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

    fn setup_instances(device: &wgpu::Device) -> (Vec<Instance>, wgpu::Buffer) {
        let instances: Vec<Instance> = (0..Instance::NUM_INSTANCES_PER_ROW)
            .flat_map(|z| {
                (0..Instance::NUM_INSTANCES_PER_ROW).map(move |x| {
                    let position = cgmath::Vector3 {
                        x: x as f32,
                        y: 0.0,
                        z: z as f32,
                    } - Instance::INSTANCE_DISPLACEMENT;

                    let rotation = if position.is_zero() {
                        cgmath::Quaternion::from_axis_angle(
                            cgmath::Vector3::unit_z(),
                            cgmath::Deg(0.0),
                        )
                    } else {
                        cgmath::Quaternion::from_axis_angle(
                            position.clone().normalize(),
                            cgmath::Deg(45.0),
                        )
                    };

                    Instance { position, rotation }
                })
            })
            .collect();

        let instance_data: Vec<InstanceRaw> = instances.iter().map(Instance::to_raw).collect();
        let instance_buffer = device.create_buffer_with_data(
            bytemuck::cast_slice(&instance_data),
            wgpu::BufferUsage::VERTEX,
        );
        (instances, instance_buffer)
    }

    #[allow(dead_code)]
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
}
