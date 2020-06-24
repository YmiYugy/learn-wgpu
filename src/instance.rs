use super::model::Vertex;
use cgmath::{
    Rotation3,
    Zero,
    InnerSpace,
};
pub struct Instance {
    pub position: cgmath::Vector3<f32>,
    pub rotation: cgmath::Quaternion<f32>,
}

impl Instance {
    pub fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: cgmath::Matrix4::from_translation(self.position)
                * cgmath::Matrix4::from(self.rotation),
        }
    }

    pub const SPACE_BETWEEN: f32 = 3.0;
    pub const NUM_INSTANCES_PER_ROW: u32 = 100;
    pub const NUM_INSTANCES: u32 =
        Instance::NUM_INSTANCES_PER_ROW * Instance::NUM_INSTANCES_PER_ROW;
    pub const INSTANCE_DISPLACEMENT: cgmath::Vector3<f32> = cgmath::Vector3::new(
        Instance::NUM_INSTANCES_PER_ROW as f32 * 0.5,
        0.0,
        Instance::NUM_INSTANCES_PER_ROW as f32 * 0.5,
    );

    pub fn setup_instances(device: &wgpu::Device) -> (Vec<Instance>, wgpu::Buffer) {
        let instances: Vec<Instance> = (0..Instance::NUM_INSTANCES_PER_ROW)
            .flat_map(|z| {
                (0..Instance::NUM_INSTANCES_PER_ROW).map(move |x| {
                    let x = Instance::SPACE_BETWEEN
                        * (x as f32 - Instance::NUM_INSTANCES_PER_ROW as f32 / 2.0);
                    let z = Instance::SPACE_BETWEEN
                        * (z as f32 - Instance::NUM_INSTANCES_PER_ROW as f32 / 2.0);
                    let position = cgmath::Vector3::new(x, 0.0, z);

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
            wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::STORAGE,
        );
        (instances, instance_buffer)
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct InstanceRaw {
    pub model: cgmath::Matrix4<f32>,
}

unsafe impl bytemuck::Pod for InstanceRaw {}
unsafe impl bytemuck::Zeroable for InstanceRaw {}

impl Vertex for InstanceRaw {
    fn desc<'a>() -> wgpu::VertexBufferDescriptor<'a> {
        wgpu::VertexBufferDescriptor {
            stride: std::mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Instance,
            attributes: &wgpu::vertex_attr_array![3 => Float4, 4 => Float4, 5 => Float4, 6 => Float4],
        }
    }
}
