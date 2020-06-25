#version 450

layout(location=0) in vec4 a_position;

layout(location = 3) in mat4 a_model;

layout(set=0, binding=0) 
uniform Uniforms {
    mat4 u_view_proj;
};

void main() {
    gl_Position = u_view_proj * a_model * a_position;
}