#version 430 core

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec4 color;
layout(location = 2) in vec3 inNormal;

uniform mat4 transformation_matrix;

out vec4 fragColor;
out vec3 fragNormal;

void main() {
    fragColor = color;
    fragNormal = inNormal;
    
    vec4 vertex_pre_tf = vec4(inPosition, 1.0);
    gl_Position = transformation_matrix * vertex_pre_tf;
}
