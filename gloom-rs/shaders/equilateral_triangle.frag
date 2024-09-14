#version 460 core

in vec4 fragColor; // Receive the color from the vertex shader
out vec4 finalColor;

void main() {
    finalColor = fragColor; // Use the interpolated color
}
