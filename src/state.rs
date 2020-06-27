use super::camera::*;
use super::instance::*;
use super::model::*;
use super::point_cloud::*;
use super::texture::*;
use super::uniforms::*;
use super::boids::*;
use winit::{event::*, window::Window};

pub struct State {
    pub surface: wgpu::Surface,
    pub adapter: wgpu::Adapter,
    pub size: winit::dpi::PhysicalSize<u32>,

    pub device: wgpu::Device,
    pub queue: wgpu::Queue,

    pub swap_chain: wgpu::SwapChain,
    pub sc_desc: wgpu::SwapChainDescriptor,

    pub depth_texture: Texture,

    obj_model: Model,

    pub instances: Vec<Instance>,
    pub instance_buffer: wgpu::Buffer,

    pub camera: Camera,
    pub camera_controller: CameraController,

    pub uniforms: Uniforms,
    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,

    pub point_cloud: PointCloud,
    pub point_cloud_pipeline: wgpu::RenderPipeline,

    pub model_render_pipeline: wgpu::RenderPipeline,
    pub default: bool,
    pub clear_color: wgpu::Color,

    pub boids: Boids,
    pub boids_render_pipeline: wgpu::RenderPipeline,
}

impl State {
    pub async fn new(window: &Window) -> State {
        let (size, surface, adapter) = Self::setup_adapter(window).await;

        let (device, queue) = Self::setup_device(&adapter).await;

        let (swap_chain, sc_desc) = Self::setup_swapchain(&device, &size, &surface);

        let depth_texture = Texture::create_depth_texture(&device, &sc_desc, "depth_texture");

        let (instances, instance_buffer) = Instance::setup_instances(&device);

        let texture_layout = Texture::setup_bing_group_layout(&device);
        let uniform_layout = Uniforms::setup_bing_group_layout(&device);
        let obj_model = Self::setup_obj_model(&device, &queue, &texture_layout);

        let (camera, camera_controller) = Self::setup_camera(&sc_desc);

        let (uniforms, uniform_buffer) = Self::setup_uniforms(&device, &camera);
        let uniform_bind_group =
            Uniforms::create_bind_group(&device, &uniform_buffer, Some(&uniform_layout));

        let point_cloud = PointCloud::new_sphere(&device, 1000);
        let point_cloud_pipeline = PointCloud::setup_default_render_pipeline(
            &device,
            Some(&[&uniform_layout]),
            Some(sc_desc.format),
            None,
        );

        let model_render_pipeline = Model::setup_default_render_pipeline(
            &device,
            Some(&[&texture_layout, &uniform_layout]),
            Some(sc_desc.format),
            None,
        );
        let clear_color = wgpu::Color {
            r: 0.1,
            g: 0.2,
            b: 0.3,
            a: 1.0,
        };

        let indices = &obj_model.meshes[0].index_buffer;
        let vertices = &obj_model.meshes[0].vertex_buffer;
        let num_elements = obj_model.meshes[0].num_elements;
        let sample_points = &point_cloud.vertex_buffer;
        let sample_count = point_cloud.num_vertices;

        let boids = Boids::create_boids(&device, 1024, indices, vertices, num_elements, sample_points, sample_count);
        let boids_render_pipeline = Boids::setup_default_render_pipeline(&device, Some(&[&uniform_layout]), Some(sc_desc.format), None);

        Self {
            surface,
            adapter,
            size,
            device,
            queue,
            swap_chain,
            sc_desc,
            depth_texture,
            obj_model,
            instances,
            instance_buffer,
            camera,
            camera_controller,
            uniforms,
            uniform_buffer,
            uniform_bind_group,
            model_render_pipeline,
            default: true,
            clear_color,
            point_cloud,
            point_cloud_pipeline,
            boids,
            boids_render_pipeline,
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        self.sc_desc.width = new_size.width;
        self.sc_desc.height = new_size.height;
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
        self.camera.aspect = self.sc_desc.width as f32 / self.sc_desc.height as f32;

        self.depth_texture =
            Texture::create_depth_texture(&self.device, &self.sc_desc, "depth_texture");
    }

    pub fn input(&mut self, event: &Event<()>) -> bool {
        self.camera_controller.process_events(event)
    }

    pub fn update(&mut self) {
        self.camera_controller.update_camera(&mut self.camera);
        self.uniforms.update_view_proj(&self.camera);

        let cmd = self.boids.update(&self.device, 0.01);

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

        self.queue.submit(vec![encoder.finish(), cmd]);
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
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachmentDescriptor {
                    attachment: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });
            // render_pass.set_pipeline(&self.model_render_pipeline);
            // render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            // let mesh = &self.obj_model.meshes[0];
            // let material = &self.obj_model.materials[mesh.material];
            // render_pass.draw_mesh_instanced(
            //     mesh,
            //     material,
            //     0..self.instances.len() as u32,
            //     &self.uniform_bind_group,
            // );
            render_pass.set_pipeline(&self.boids_render_pipeline);
            render_pass.draw_boids_instanced(&self.boids, 0..10, &self.uniform_bind_group);

            // render_pass.set_pipeline(&self.point_cloud_pipeline);
            // render_pass.draw_point_cloud_instanced(
            //     &self.point_cloud,
            //     &self.instance_buffer,
            //     0..self.instances.len() as u32,
            //     &self.uniform_bind_group,
            // );
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
            eye: (0.0, 25.0, 30.0).into(),
            target: (0.0, 0.0, 0.0).into(),
            up: cgmath::Vector3::unit_y(),
            aspect: sc_desc.width as f32 / sc_desc.height as f32,
            fovy: 45.0,
            znear: 0.01,
            zfar: 2000.0,
        };

        let camera_controller = CameraController::new(0.2, 10.0);

        (camera, camera_controller)
    }

    fn setup_uniforms(device: &wgpu::Device, camera: &Camera) -> (Uniforms, wgpu::Buffer) {
        let mut uniforms = Uniforms::new();
        uniforms.update_view_proj(&camera);

        let uniform_buffer = device.create_buffer_with_data(
            bytemuck::cast_slice(&[uniforms]),
            wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        );
        (uniforms, uniform_buffer)
    }

    fn setup_obj_model(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Model {
        let (obj_model, cmds) =
            Model::load(&device, &bind_group_layout, "assets/models/cube.obj").unwrap();
        queue.submit(cmds);
        obj_model
    }
}

pub trait Renderable {
    fn create_shader_module(device: & wgpu::Device, code: &[u32]) -> wgpu::ShaderModule {
        return device.create_shader_module(wgpu::util::make_spirv(bytemuck::cast_slice(code)));
    }

    fn create_render_pipeline_descriptor<'a>(
        pipeline_layout: &'a wgpu::PipelineLayout,
        vs: &'a wgpu::ShaderModule,
        fs: Option<&'a wgpu::ShaderModule>,
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
            fragment_stage: if let Some(fs) = fs {
                Some(wgpu::ProgrammableStageDescriptor {
                    module: fs,
                    entry_point: "main",
                })
            } else {
                None
            },
            rasterization_state: Some(wgpu::RasterizationStateDescriptor {
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: wgpu::CullMode::None,
                depth_bias: 0,
                depth_bias_slope_scale: 0.0,
                depth_bias_clamp: 0.0,
            }),
            primitive_topology: topology,
            color_states: color_states,
            depth_stencil_state: Some(wgpu::DepthStencilStateDescriptor {
                format: Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil_front: wgpu::StencilStateFaceDescriptor::IGNORE,
                stencil_back: wgpu::StencilStateFaceDescriptor::IGNORE,
                stencil_read_mask: 0,
                stencil_write_mask: 0,
            }),
            vertex_state: wgpu::VertexStateDescriptor {
                index_format: wgpu::IndexFormat::Uint32,
                vertex_buffers: vertex_buffers,
            },
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        }
    }

    fn create_color_state_descriptor(format: wgpu::TextureFormat) -> wgpu::ColorStateDescriptor {
        wgpu::ColorStateDescriptor {
            format: format,
            alpha_blend: wgpu::BlendDescriptor::REPLACE,
            color_blend: wgpu::BlendDescriptor::REPLACE,
            write_mask: wgpu::ColorWrite::ALL,
        }
    }

    fn create_pipeline_layout(
        device: & wgpu::Device,
        bind_group_layouts: & [& wgpu::BindGroupLayout],
    ) -> wgpu::PipelineLayout {
        device.create_pipeline_layout(& wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: bind_group_layouts,
        })
    }

    fn create_render_pipeline(
        device: & wgpu::Device,
        bind_group_layouts: & [& wgpu::BindGroupLayout],
        format: wgpu::TextureFormat,
        (vs, fs): (& wgpu::ShaderModule, Option<& wgpu::ShaderModule>),
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

    fn setup_shader(device: & wgpu::Device) -> (wgpu::ShaderModule, Option<wgpu::ShaderModule>);

    fn setup_bind_group_layouts(device: & wgpu::Device) -> Vec<wgpu::BindGroupLayout>;

    fn setup_vertex_input<'a>() -> Vec<wgpu::VertexBufferDescriptor<'a>>;

    fn setup_default_render_pipeline(
        device: & wgpu::Device,
        layouts: Option<& [& wgpu::BindGroupLayout]>,
        format: Option<wgpu::TextureFormat>,
        shaders: Option<(& wgpu::ShaderModule, Option<& wgpu::ShaderModule>)>,
    ) -> wgpu::RenderPipeline;
}
