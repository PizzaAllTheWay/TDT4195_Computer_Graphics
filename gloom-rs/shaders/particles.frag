#version 460 core

in vec4 fragColor; // Receive the color from the vertex shader

out vec4 finalColor;

uniform mat4 changingColorMatrix;

void main() {
    finalColor = changingColorMatrix * fragColor; // Use the interpolated color
}
