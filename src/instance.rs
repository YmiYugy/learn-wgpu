use super::vertex::VBDesc;
pub struct Instance {
    pub position: cgmath::Vector3<f32>,
    pub rotation: cgmath::Quaternion<f32>,
}

impl Instance {
    pub fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: cgmath::Matrix4::from_translation(self.position) * cgmath::Matrix4::from(self.rotation),
        }
    }

    pub const NUM_INSTANCES_PER_ROW: u32 = 100;
    pub const NUM_INSTANCES: u32 =
        Instance::NUM_INSTANCES_PER_ROW * Instance::NUM_INSTANCES_PER_ROW;
    pub const INSTANCE_DISPLACEMENT: cgmath::Vector3<f32> = cgmath::Vector3::new(
        Instance::NUM_INSTANCES_PER_ROW as f32 * 0.5,
        0.0,
        Instance::NUM_INSTANCES_PER_ROW as f32 * 0.5,
    );
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct InstanceRaw {
    pub model: cgmath::Matrix4<f32>
}

unsafe impl bytemuck::Pod for InstanceRaw {}
unsafe impl bytemuck::Zeroable for InstanceRaw {}

impl VBDesc for InstanceRaw {
    fn desc<'a>() -> wgpu::VertexBufferDescriptor<'a> {
        wgpu::VertexBufferDescriptor {
            stride: std::mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Instance,
            attributes: &wgpu::vertex_attr_array![2 => Float4, 3 => Float4, 4 => Float4, 5 => Float4],
            
        }
    }
    
}