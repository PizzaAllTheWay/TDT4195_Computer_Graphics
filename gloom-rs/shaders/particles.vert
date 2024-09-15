#version 460 core

layout(location = 0) in vec3 position;   // Vertex position
layout(location = 1) in vec4 vertexColor; // Vertex color (from VAO)

out vec4 fragColor;  // Pass the color to the fragment shader

uniform mat4 viewProjectionMatrix;

void main() {
    gl_Position = viewProjectionMatrix * vec4(position, 1.0);
    fragColor = vertexColor;  // Pass color to fragment shader
}
