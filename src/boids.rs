use super::instance::*;
use super::model::Vertex;
use super::point_cloud::*;
use super::state::*;
use super::uniforms::*;
use cgmath::InnerSpace;
use include_glsl::include_glsl;
use rand::Rng;
use std::ops::Range;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Boid {
    pos: [f32; 4],
    vel: [f32; 4],
}

unsafe impl bytemuck::Pod for Boid {}
unsafe impl bytemuck::Zeroable for Boid {}

impl Vertex for Boid {
    fn desc<'a>() -> wgpu::VertexBufferDescriptor<'a> {
        wgpu::VertexBufferDescriptor {
            stride: std::mem::size_of::<Boid>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float4],
        }
    }
}

pub struct Boids {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,

    instance_buffer: wgpu::Buffer,
    num_instances: u32,

    boid_buffer1: wgpu::Buffer,
    boid_buffer2: wgpu::Buffer,
    boid_buffer_index: bool,
    boid_bind_group1: wgpu::BindGroup,
    boid_bind_group2: wgpu::BindGroup,

    compute_scene_bind_group: wgpu::BindGroup,

    compute_uniforms: ComputeUniforms,
    compute_uniform_buffer: wgpu::Buffer,
    compute_uniform_bind_group: wgpu::BindGroup,

    compute_shader: wgpu::ShaderModule,
    compute_pipeline: wgpu::ComputePipeline,
}

impl Boids {
    pub fn create_boids(
        device: &wgpu::Device,
        num_instances: u32,
        scene_indices: & wgpu::Buffer,
        scene_vertices: & wgpu::Buffer,
        scene_index_count: u32,
        sample_points: & wgpu::Buffer,
        sample_count: u32,
    ) -> Self {
        let (obj_models, _) = tobj::load_obj("assets/models/boid.obj", true).unwrap();
        assert_eq!(obj_models.len(), 1);
        for m in obj_models {
            let mut vertices = Vec::new();
            let m = m.mesh;
            for i in 0..m.positions.len() / 3 {
                vertices.push(Point {
                    pos: [
                        m.positions[i * 3],
                        m.positions[i * 3 + 1],
                        m.positions[i * 3 + 2],
                        1.0,
                    ],
                });
            }
            let vertex_buffer = device.create_buffer_with_data(
                bytemuck::cast_slice(&vertices),
                wgpu::BufferUsage::VERTEX,
            );
            let index_buffer = device.create_buffer_with_data(
                bytemuck::cast_slice(&m.indices),
                wgpu::BufferUsage::INDEX,
            );
            let num_indices = m.indices.len() as u32;

            let mut rng = rand::thread_rng();

            let boids: Vec<_> = std::iter::repeat_with(|| Boid {
                pos: [
                    (rng.gen::<f32>() - 0.5) * 5.0,
                    (rng.gen::<f32>() - 0.5) * 5.0,
                    (rng.gen::<f32>() - 0.5) * 5.0,
                    1.0,
                ],
                vel: [
                    (rng.gen::<f32>() - 0.5) * 5.0,
                    (rng.gen::<f32>() - 0.5) * 5.0,
                    (rng.gen::<f32>() - 0.5) * 5.0,
                    0.0,
                ],
            })
            .take(num_instances as usize)
            .collect();
            let boid_buffer1 = device
                .create_buffer_with_data(bytemuck::cast_slice(&boids), wgpu::BufferUsage::STORAGE);
            let boid_buffer2 = device
                .create_buffer_with_data(bytemuck::cast_slice(&boids), wgpu::BufferUsage::STORAGE);

            let instances: Vec<InstanceRaw> = boids
                .iter()
                .map(|boid| {
                    Instance {
                        position: cgmath::vec3(boid.pos[0], boid.pos[1], boid.pos[2]),
                        rotation: cgmath::Quaternion::from_arc(
                            cgmath::Vector3::unit_x(),
                            cgmath::vec3(boid.vel[0], boid.vel[1], boid.vel[2]).normalize(),
                            None,
                        ),
                    }
                    .to_raw()
                })
                .collect();
            let instance_buffer = device.create_buffer_with_data(
                bytemuck::cast_slice(&instances),
                wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::STORAGE,
            );

            let boid_bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("boid_bind_group_layout"),
                    bindings: &[
                        wgpu::BindGroupLayoutEntry::new(
                            0,
                            wgpu::ShaderStage::COMPUTE,
                            wgpu::BindingType::StorageBuffer {
                                dynamic: false,
                                min_binding_size: None,
                                readonly: false,
                            },
                        ),
                        wgpu::BindGroupLayoutEntry::new(
                            1,
                            wgpu::ShaderStage::COMPUTE,
                            wgpu::BindingType::StorageBuffer {
                                dynamic: false,
                                min_binding_size: None,
                                readonly: false,
                            },
                        ),
                    ],
                });
            let boid_bind_group1 = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &boid_bind_group_layout,
                bindings: &[
                    wgpu::Binding {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(boid_buffer1.slice(..)),
                    },
                    wgpu::Binding {
                        binding: 1,
                        resource: wgpu::BindingResource::Buffer(boid_buffer2.slice(..)),
                    },
                ],
                label: Some("boid_bind_group1"),
            });
            let boid_bind_group2 = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &boid_bind_group_layout,
                bindings: &[
                    wgpu::Binding {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(boid_buffer2.slice(..)),
                    },
                    wgpu::Binding {
                        binding: 1,
                        resource: wgpu::BindingResource::Buffer(boid_buffer1.slice(..)),
                    },
                ],
                label: Some("boid_bind_group2"),
            });

            let compute_scene_bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("compute_scene_bind_group_layout"),
                    bindings: &[
                        wgpu::BindGroupLayoutEntry::new(
                            0,
                            wgpu::ShaderStage::COMPUTE,
                            wgpu::BindingType::StorageBuffer {
                                dynamic: false,
                                min_binding_size: None,
                                readonly: false,
                            },
                        ),
                        wgpu::BindGroupLayoutEntry::new(
                            1,
                            wgpu::ShaderStage::COMPUTE,
                            wgpu::BindingType::StorageBuffer {
                                dynamic: false,
                                min_binding_size: None,
                                readonly: false,
                            },
                        ),
                        wgpu::BindGroupLayoutEntry::new(
                            2,
                            wgpu::ShaderStage::COMPUTE,
                            wgpu::BindingType::StorageBuffer {
                                dynamic: false,
                                min_binding_size: None,
                                readonly: false,
                            },
                        ),
                        wgpu::BindGroupLayoutEntry::new(
                            3,
                            wgpu::ShaderStage::COMPUTE,
                            wgpu::BindingType::StorageBuffer {
                                dynamic: false,
                                min_binding_size: None,
                                readonly: false,
                            },
                        ),
                    ],
                });
            let compute_scene_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &compute_scene_bind_group_layout,
                bindings: &[
                    wgpu::Binding {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(instance_buffer.slice(..)),
                    },
                    wgpu::Binding {
                        binding: 1,
                        resource: wgpu::BindingResource::Buffer(scene_indices.slice(..)),
                    },
                    wgpu::Binding {
                        binding: 2,
                        resource: wgpu::BindingResource::Buffer(scene_vertices.slice(..)),
                    },
                    wgpu::Binding {
                        binding: 3,
                        resource: wgpu::BindingResource::Buffer(sample_points.slice(..)),
                    },
                ],
                label: Some("compute_scene_bind_group"),
            });

            let compute_uniforms = ComputeUniforms {
                triangle_count: scene_index_count / 3,
                sample_cout: sample_count,
                delta: 0.0,
            };
            let compute_uniform_buffer = device.create_buffer_with_data(
                bytemuck::cast_slice(&[compute_uniforms]),
                wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            );
            let compute_uniform_bind_group_layout =
                ComputeUniforms::setup_bing_group_layout(device);
            let compute_uniform_bind_group = ComputeUniforms::create_bind_group(
                device,
                &compute_uniform_buffer,
                Some(&compute_uniform_bind_group_layout),
            );

            let compute_pipline_layout =
                device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    bind_group_layouts: &[
                        &boid_bind_group_layout,
                        &compute_scene_bind_group_layout,
                        &compute_uniform_bind_group_layout,
                    ],
                });

            let compute_shader =
                Self::create_shader_module(device, include_glsl!("../shaders/boids.comp"));

            let compute_pipeline =
                device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    layout: &compute_pipline_layout,
                    compute_stage: wgpu::ProgrammableStageDescriptor {
                        module: &compute_shader,
                        entry_point: "main",
                    },
                });

            return Self {
                vertex_buffer,
                index_buffer,
                num_indices,
                instance_buffer,
                num_instances,
                boid_buffer1,
                boid_buffer2,
                boid_buffer_index: false,
                boid_bind_group1,
                boid_bind_group2,
                compute_scene_bind_group,
                compute_uniforms,
                compute_uniform_buffer,
                compute_uniform_bind_group,
                compute_shader,
                compute_pipeline,
            };
        }

        panic!("no model found")
    }

    pub fn update(& mut self, device: &wgpu::Device, delta: f32) -> wgpu::CommandBuffer {
        self.compute_uniforms.delta = delta;
        let staging_buffer = device.create_buffer_with_data(
            bytemuck::cast_slice(&[self.compute_uniforms]),
            wgpu::BufferUsage::COPY_SRC,
        );

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("compute_encoder"),
        });

        encoder.copy_buffer_to_buffer(
            &staging_buffer,
            0,
            &self.compute_uniform_buffer,
            0,
            std::mem::size_of::<ComputeUniforms>() as wgpu::BufferAddress,
        );
        {
            let mut compute_pass = encoder.begin_compute_pass();
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, if self.boid_buffer_index {&self.boid_bind_group2} else {&self.boid_bind_group1}, &[]);
            self.boid_buffer_index = !self.boid_buffer_index;
            compute_pass.set_bind_group(1, &self.compute_scene_bind_group, &[]);
            compute_pass.set_bind_group(2, &self.compute_uniform_bind_group, &[]);
            compute_pass.dispatch(self.num_instances, 1, 1);
        }

        encoder.finish()
    }
}

impl Renderable for Boids {
    fn setup_shader(device: &wgpu::Device) -> (wgpu::ShaderModule, Option<wgpu::ShaderModule>) {
        (
            Self::create_shader_module(device, include_glsl!("../shaders/boids.vert")),
            Some(Self::create_shader_module(
                device,
                include_glsl!("../shaders/boids.frag"),
            )),
        )
    }
    fn setup_bind_group_layouts(device: &wgpu::Device) -> Vec<wgpu::BindGroupLayout> {
        vec![Uniforms::setup_bing_group_layout(device)]
    }
    fn setup_vertex_input<'a>() -> Vec<wgpu::VertexBufferDescriptor<'a>> {
        vec![Point::desc(), InstanceRaw::desc()]
    }
    fn setup_default_render_pipeline(
        device: &wgpu::Device,
        layouts: Option<&[&wgpu::BindGroupLayout]>,
        format: Option<wgpu::TextureFormat>,
        shaders: Option<(&wgpu::ShaderModule, Option<&wgpu::ShaderModule>)>,
    ) -> wgpu::RenderPipeline {
        if shaders.is_some() {
            Self::create_render_pipeline(
                device,
                layouts.unwrap_or(
                    &Self::setup_bind_group_layouts(device)
                        .iter()
                        .collect::<Vec<&wgpu::BindGroupLayout>>(),
                ),
                format.unwrap(),
                shaders.unwrap(),
                wgpu::PrimitiveTopology::TriangleList,
                Self::setup_vertex_input().as_ref(),
            )
        } else {
            let (vs, fs) = Self::setup_shader(device);
            Self::create_render_pipeline(
                device,
                layouts.unwrap_or(
                    &Self::setup_bind_group_layouts(device)
                        .iter()
                        .collect::<Vec<&wgpu::BindGroupLayout>>(),
                ),
                format.unwrap(),
                (&vs, fs.as_ref()),
                wgpu::PrimitiveTopology::TriangleList,
                Self::setup_vertex_input().as_ref(),
            )
        }
    }
}

pub trait DrawBoids<'a, 'b>
where
    'b: 'a,
{
    fn draw_boids(&mut self, boids: &'b Boids, uniforms: &'b wgpu::BindGroup);
    fn draw_boids_instanced(
        &mut self,
        boids: &'b Boids,
        instances: Range<u32>,
        uniforms: &'b wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawBoids<'a, 'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_boids(&mut self, boids: &'b Boids, uniforms: &'b wgpu::BindGroup) {
        self.draw_boids_instanced(boids, 0..1, uniforms);
    }
    fn draw_boids_instanced(
        &mut self,
        boids: &'b Boids,
        instances: Range<u32>,
        uniforms: &'b wgpu::BindGroup,
    ) {
        self.set_index_buffer(boids.index_buffer.slice(..));
        self.set_vertex_buffer(0, boids.vertex_buffer.slice(..));
        self.set_vertex_buffer(1, boids.instance_buffer.slice(..));
        self.set_bind_group(0, &uniforms, &[]);
        self.draw_indexed(0..boids.num_indices, 0, instances);
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ComputeUniforms {
    triangle_count: u32,
    sample_cout: u32,
    delta: f32,
}

unsafe impl bytemuck::Pod for ComputeUniforms {}
unsafe impl bytemuck::Zeroable for ComputeUniforms {}

impl ComputeUniforms {
    pub fn setup_bing_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("compute_uniform_bind_group_layout"),
            bindings: &[wgpu::BindGroupLayoutEntry::new(
                0,
                wgpu::ShaderStage::COMPUTE,
                wgpu::BindingType::UniformBuffer {
                    dynamic: false,
                    min_binding_size: None,
                },
            )],
        })
    }

    pub fn create_bind_group(
        device: &wgpu::Device,
        buffer: &wgpu::Buffer,
        layout: Option<&wgpu::BindGroupLayout>,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: layout.unwrap_or(&Self::setup_bing_group_layout(device)),
            bindings: &[wgpu::Binding {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(buffer.slice(..)),
            }],
            label: Some("compute_uniform_bind_group"),
        })
    }
}
