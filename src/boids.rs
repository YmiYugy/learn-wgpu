#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct Boid {
    pos: cgmath::Vector4<f32>,
    vel: cgmath::Vector4<f32>,
}

unsafe impl bytemuck::Pod for Boid {}
unsafe impl bytemuck::Zeroable for Boid {}