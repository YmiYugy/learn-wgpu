#version 450

layout(location=0) in vec4 a_position;
layout(location=1) in vec4 a_normal;
layout(location=2) in vec2 a_tex_coords;

layout(location = 3) in mat4 a_model;

layout(location=0) out vec2 v_tex_coords;

layout(set=1, binding=0) 
uniform Uniforms {
    mat4 u_view_proj;
};

void main() {
    v_tex_coords = a_tex_coords;
    gl_Position = u_view_proj * a_model * a_position;
}