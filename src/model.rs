use super::instance::*;
use super::state::*;
use super::texture::*;
use super::uniforms::*;
use include_glsl::include_glsl;
use std::{ops::Range, path::Path, path::PathBuf};

pub trait Vertex {
    fn desc<'a>() -> wgpu::VertexBufferDescriptor<'a>;
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ModelVertex {
    pub position: [f32; 4],
    pub normal: [f32; 4],
    pub tex_coords: [f32; 2],
}

impl Vertex for ModelVertex {
    fn desc<'a>() -> wgpu::VertexBufferDescriptor<'a> {
        wgpu::VertexBufferDescriptor {
            stride: std::mem::size_of::<ModelVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float4, 1 => Float4, 2 => Float2],
        }
    }
}

unsafe impl bytemuck::Pod for ModelVertex {}
unsafe impl bytemuck::Zeroable for ModelVertex {}

pub struct Model {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
}

pub struct Material {
    pub name: String,
    pub diffuse_texture: Texture,
    pub bind_group: wgpu::BindGroup,
}

pub struct Mesh {
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
    pub material: usize,
}

impl Model {
    pub fn load<P: AsRef<Path> + std::fmt::Debug>(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        path: P,
    ) -> Result<(Self, Vec<wgpu::CommandBuffer>), failure::Error> {
        let (obj_models, obj_materials) = tobj::load_obj(path.as_ref(), true)?;

        // We're assuming that the texture files are stored with the obj file
        let mut containing_folder = PathBuf::from(path.as_ref());
        containing_folder.pop();
        containing_folder.pop();
        containing_folder.push("textures");

        // Our `Texure` struct currently returns a `CommandBuffer` when it's created so we need to collect those and return them.
        let mut command_buffers = Vec::new();

        let mut materials = Vec::new();
        for mat in obj_materials {
            let diffuse_path = mat.diffuse_texture;
            let (diffuse_texture, cmds) =
                Texture::load(&device, containing_folder.join(diffuse_path))?;

            let bind_group = diffuse_texture.create_bind_group(device, Some(layout));

            materials.push(Material {
                name: mat.name,
                diffuse_texture,
                bind_group,
            });
            command_buffers.push(cmds);
        }

        let mut meshes = Vec::new();
        for m in obj_models {
            let mut vertices = Vec::new();
            for i in 0..m.mesh.positions.len() / 3 {
                vertices.push(ModelVertex {
                    position: [
                        m.mesh.positions[i * 3],
                        m.mesh.positions[i * 3 + 1],
                        m.mesh.positions[i * 3 + 2],
                        1.0,
                    ],
                    tex_coords: [m.mesh.texcoords[i * 2], m.mesh.texcoords[i * 2 + 1]],
                    normal: [
                        m.mesh.normals[i * 3],
                        m.mesh.normals[i * 3 + 1],
                        m.mesh.normals[i * 3 + 2],
                        0.0,
                    ],
                });
            }

            let vertex_buffer = device.create_buffer_with_data(
                bytemuck::cast_slice(&vertices),
                wgpu::BufferUsage::VERTEX,
            );
            let index_buffer = device.create_buffer_with_data(
                bytemuck::cast_slice(&m.mesh.indices),
                wgpu::BufferUsage::INDEX,
            );

            meshes.push(Mesh {
                name: m.name,
                vertex_buffer,
                index_buffer,
                num_elements: m.mesh.indices.len() as u32,
                material: m.mesh.material_id.unwrap_or(0),
            });
        }

        Ok((Self { meshes, materials }, command_buffers))
    }
}

pub trait DrawModel<'a, 'b>
where
    'b: 'a,
{
    fn draw_mesh(&mut self, mesh: &'b Mesh, material: &'b Material, uniforms: &'b wgpu::BindGroup);
    fn draw_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        material: &'b Material,
        instances: Range<u32>,
        uniforms: &'b wgpu::BindGroup,
    );

    fn draw_model(&mut self, model: &'b Model, uniforms: &'b wgpu::BindGroup);
    fn draw_model_instanced(
        &mut self,
        model: &'b Model,
        instances: Range<u32>,
        uniforms: &'b wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawModel<'a, 'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_mesh(&mut self, mesh: &'b Mesh, material: &'b Material, uniforms: &'b wgpu::BindGroup) {
        self.draw_mesh_instanced(mesh, material, 0..1, uniforms);
    }
    fn draw_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        material: &'b Material,
        instances: Range<u32>,
        uniforms: &'b wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..));
        self.set_bind_group(0, &material.bind_group, &[]);
        self.set_bind_group(1, &uniforms, &[]);
        self.draw_indexed(0..mesh.num_elements, 0, instances);
    }

    fn draw_model(&mut self, model: &'b Model, uniforms: &'b wgpu::BindGroup) {
        self.draw_model_instanced(model, 0..1, uniforms);
    }

    fn draw_model_instanced(
        &mut self,
        model: &'b Model,
        instances: Range<u32>,
        uniforms: &'b wgpu::BindGroup,
    ) {
        for mesh in &model.meshes {
            let material = &model.materials[mesh.material];
            self.draw_mesh_instanced(mesh, material, instances.clone(), uniforms);
        }
    }
}

impl Renderable for Model {
    fn setup_shader(device: &wgpu::Device) -> (wgpu::ShaderModule, Option<wgpu::ShaderModule>) {
        let vs = Self::create_shader_module(device, include_glsl!("../shaders/textured.vert"));
        let fs = Self::create_shader_module(device, include_glsl!("../shaders/textured.frag"));

        (vs, Some(fs))
    }
    fn setup_bind_group_layouts(device: &wgpu::Device) -> Vec<wgpu::BindGroupLayout> {
        let texture_layout = Texture::setup_bing_group_layout(device);
        let uniform_layout = Uniforms::setup_bing_group_layout(device);
        vec![texture_layout, uniform_layout]
    }
    fn setup_vertex_input<'a>() -> Vec<wgpu::VertexBufferDescriptor<'a>> {
        vec![ModelVertex::desc(), InstanceRaw::desc()]
    }
    fn setup_default_render_pipeline(
        device: &wgpu::Device,
        layouts: Option<&[&wgpu::BindGroupLayout]>,
        format: Option<wgpu::TextureFormat>,
        shaders: Option<(&wgpu::ShaderModule, Option<&wgpu::ShaderModule>)>,
    ) -> wgpu::RenderPipeline {
        if layouts.is_some() {
            if shaders.is_some() {
                Self::create_render_pipeline(
                    device,
                    layouts.unwrap(),
                    format.unwrap(),
                    shaders.unwrap().0,
                    shaders.unwrap().1,
                    wgpu::PrimitiveTopology::TriangleList,
                    Self::setup_vertex_input().as_ref(),
                )
            } else {
                let (vs, fs) = Self::setup_shader(device);
                Self::create_render_pipeline(
                    device,
                    layouts.unwrap(),
                    format.unwrap(),
                    &vs,
                    fs.as_ref(),
                    wgpu::PrimitiveTopology::TriangleList,
                    Self::setup_vertex_input().as_ref(),
                )
            }
        } else {
            let layouts_v = Self::setup_bind_group_layouts(device);
            let layouts_v: Vec<&wgpu::BindGroupLayout> = layouts_v.iter().collect();
            let layouts = layouts_v.as_slice();
            if shaders.is_some() {
                Self::create_render_pipeline(
                    device,
                    layouts,
                    format.unwrap(),
                    shaders.unwrap().0,
                    shaders.unwrap().1,
                    wgpu::PrimitiveTopology::TriangleList,
                    Self::setup_vertex_input().as_ref(),
                )
            } else {
                let (vs, fs) = Self::setup_shader(device);
                Self::create_render_pipeline(
                    device,
                    layouts,
                    format.unwrap(),
                    &vs,
                    fs.as_ref(),
                    wgpu::PrimitiveTopology::TriangleList,
                    Self::setup_vertex_input().as_ref(),
                )
            }
        }
    }
}
