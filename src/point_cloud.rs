use super::model::*;
use super::state::*;
use super::uniforms::*;
use super::instance::*;
use include_glsl::include_glsl;
use std::ops::Range;

pub struct PointCloud {
    vertex_buffer: wgpu::Buffer,
    num_vertices: u32,
}

impl PointCloud {
    pub fn new_sphere(device: &wgpu::Device, samples: u32) -> Self {
        let indices = 0..samples;

        let phi = indices
            .clone()
            .map(|x| (1.0 as f32 - 2.0 * x as f32 / samples as f32).acos());
        let sqrt5 = (5 as f32).sqrt();
        let theta = indices
            .clone()
            .map(|x| std::f32::consts::PI * (1.0 + sqrt5) * x as f32);

        let vertices: Vec<Point> = phi
            .zip(theta)
            .map(|(phi, theta)| Point {
                pos: [
                    phi.cos(),
                    theta.sin() * phi.sin(),
                    theta.cos() * phi.sin(),
                    1.0,
                ],
            })
            .collect();

        let vertex_buffer = device.create_buffer_with_data(
            bytemuck::cast_slice(&vertices.as_slice()),
            wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::STORAGE,
        );

        Self {
            vertex_buffer,
            num_vertices: samples,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Point {
    pub pos: [f32; 4],
}

unsafe impl bytemuck::Pod for Point {}
unsafe impl bytemuck::Zeroable for Point {}

impl Vertex for Point {
    fn desc<'a>() -> wgpu::VertexBufferDescriptor<'a> {
        wgpu::VertexBufferDescriptor {
            stride: std::mem::size_of::<Point>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float4],
        }
    }
}

impl Renderable for PointCloud {
    fn setup_shader(device: &wgpu::Device) -> (wgpu::ShaderModule, Option<wgpu::ShaderModule>) {
        (
            Self::create_shader_module(device, include_glsl!("../shaders/point_cloud.vert")),
            Some(Self::create_shader_module(
                device,
                include_glsl!("../shaders/point_cloud.frag"),
            )),
        )
    }
    fn setup_bind_group_layouts(device: &wgpu::Device) -> Vec<wgpu::BindGroupLayout> {
        vec![Uniforms::setup_bing_group_layout(device)]
    }
    fn setup_vertex_input<'a>() -> Vec<wgpu::VertexBufferDescriptor<'a>> {
        let desc1= Point::desc();
        let desc2 = InstanceRaw::desc();
        vec![desc1, desc2]
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
                wgpu::PrimitiveTopology::PointList,
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
                wgpu::PrimitiveTopology::PointList,
                Self::setup_vertex_input().as_ref(),
            )
        }
    }
}
pub trait DrawPointCloud<'a, 'b>
where
    'b: 'a,
{
    fn draw_point_cloud(&mut self, point_cloud: &'b PointCloud, instance_buffer: &'b wgpu::Buffer, uniforms: &'b wgpu::BindGroup);
    fn draw_point_cloud_instanced(
        &mut self,
        point_cloud: &'b PointCloud,
        instance_buffer: &'b wgpu::Buffer,
        instances: Range<u32>,
        uniforms: &'b wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawPointCloud<'a, 'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_point_cloud(&mut self, point_cloud: &'b PointCloud, instance_buffer: &'b wgpu::Buffer, uniforms: &'b wgpu::BindGroup) {
        self.draw_point_cloud_instanced(point_cloud, instance_buffer, 0..1, uniforms);
    }
    fn draw_point_cloud_instanced(
        &mut self,
        point_cloud: &'b PointCloud,
        instance_buffer: &'b wgpu::Buffer,
        instances: Range<u32>,
        uniforms: &'b wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, point_cloud.vertex_buffer.slice(..));
        self.set_vertex_buffer(1, instance_buffer.slice(..));
        self.set_bind_group(0, &uniforms, &[]);
        self.draw(0..point_cloud.num_vertices, instances);
    }
}