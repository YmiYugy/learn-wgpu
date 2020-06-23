pub struct Instance {
    pub position: cgmath::Vector3<f32>,
    pub rotation: cgmath::Quaternion<f32>,
}

impl Instance {
    pub fn to_matrix(&self) -> cgmath::Matrix4<f32> {
        cgmath::Matrix4::from_translation(self.position) * cgmath::Matrix4::from(self.rotation)
    }

    pub const NUM_INSTANCES_PER_ROW: u32 = 10;
    pub const NUM_INSTANCES: u32 =
        Instance::NUM_INSTANCES_PER_ROW * Instance::NUM_INSTANCES_PER_ROW;
    pub const INSTANCE_DISPLACEMENT: cgmath::Vector3<f32> = cgmath::Vector3::new(
        Instance::NUM_INSTANCES_PER_ROW as f32 * 0.5,
        0.0,
        Instance::NUM_INSTANCES_PER_ROW as f32 * 0.5,
    );
}
