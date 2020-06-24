use super::model::*;

pub struct PointCloud {
    vertex_buffer: wgpu::Buffer,
    num_vertices: u32,
}

impl PointCloud {
    pub fn new_sphere(device: &wgpu::Device, samples: u32) -> Self {
        let indices = 0..samples;

        let phi = indices.clone().map(|x| (1.0 as f32 - 2.0*x as f32/samples as f32).acos());
        let sqrt5 = (5 as f32).sqrt();
        let theta = indices.clone().map(|x| std::f32::consts::PI * (1.0 + sqrt5) * x as f32);

        let vertices: Vec<Point> = phi.zip(theta).map(|(phi, theta)| Point {pos: [phi.cos(), theta.sin() * phi.sin(), theta.cos() * phi.sin(),  1.0]}).collect();

        let vertex_buffer = device.create_buffer_with_data(bytemuck::cast_slice(&vertices.as_slice()), wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::STORAGE);

        Self {
            vertex_buffer,
            num_vertices: samples,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct Point {
    pos: [f32;4]
}

unsafe impl bytemuck::Pod for Point {}
unsafe impl bytemuck::Zeroable for Point {}

impl Vertex for PointCloud {
    fn desc<'a>() -> wgpu::VertexBufferDescriptor<'a> {
        wgpu::VertexBufferDescriptor {
            stride: std::mem::size_of::<Point>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float4], 
        }
    }
}




