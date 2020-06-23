use super::camera::Camera;
use cgmath::SquareMatrix;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Uniforms {
    pub view_proj: cgmath::Matrix4<f32>,
    pub model: cgmath::Matrix4<f32>,
}

unsafe impl bytemuck::Pod for Uniforms {}
unsafe impl bytemuck::Zeroable for Uniforms {}

impl Uniforms {
    pub fn new() -> Uniforms {
        Self {
            view_proj: cgmath::Matrix4::identity(),
            model: cgmath::Matrix4::identity(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.build_view_projection_matrix();
    }
}
