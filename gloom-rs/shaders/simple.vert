#version 430 core

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec4 color;
layout(location = 2) in vec3 inNormal;

uniform mat4 mvp_matrix; // MVP matrix
uniform mat4 model_matrix; // Model matrix (used for normals)     

out vec4 fragColor;
out vec3 fragNormal;

void main() {
    fragColor = color;

    // Extract the top-left 3x3 part of the model matrix for normal transformation
    mat3 normal_matrix = mat3(model_matrix); 

    // Transform the normal and normalize it
    fragNormal = normalize(normal_matrix * inNormal);
    
    vec4 vertex_pre_tf = vec4(inPosition, 1.0);
    gl_Position = mvp_matrix * vertex_pre_tf;
}
