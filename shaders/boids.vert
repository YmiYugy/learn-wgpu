#version 450

layout(location=0) in vec4 a_position;

layout(location = 1) in vec4 a_translation;
layout(location = 2) in vec4 a_rotation;

layout(set=0, binding=0) 
uniform Uniforms {
    mat4 u_view_proj;
};

#define eps 0.005

vec3 orthogonal(vec3 v) {
    float x = abs(v.x);
    float y = abs(v.y);
    float z = abs(v.z);

    vec3 other = x < y ? (x < z ? vec3(1, 0, 0) : vec3(0, 0, 1)) : (y < z ? vec3(0, 1, 0) : vec3(0, 0, 1));
    return cross(v, other);
}

vec4 get_rotation_between(vec3 u, vec3 v) {
    vec4 q;
    vec3 v0 = normalize(u);
    vec3 v1 = normalize(v);

    float d = dot(v0, v1);

    if (d < (eps - 1.0)) {
        q = vec4(normalize(orthogonal(u)), 0);
    } else {
        float s = sqrt((1+d)*2);
        float invs = 1/s;

        vec3 c = cross(v0, v1);

        q.x = c.x * invs;
        q.y = c.y * invs;
        q.z = c.z * invs;
        q.w = s * 0.5;
        q = normalize(q);
    }
    return q;
}

vec3 rotate(vec4 quaternion, vec3 vec) {
    return vec + 2.0 * cross(quaternion.xyz, cross(quaternion.xyz, vec) + quaternion.w * vec);
}


void main() {
    gl_Position = u_view_proj *  (vec4(0.05 * rotate(get_rotation_between(vec3(1,0,0), a_rotation.xyz),a_position.xyz), 1.0)+a_translation);
}