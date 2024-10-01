#version 430 core

in vec4 fragColor;  // Color passed from the vertex shader
in vec3 fragNormal; // Normal passed from the vertex shader

out vec4 outColor;  // Final color to be written to the screen

void main() {
    // The defined light direction
    vec3 lightDirection = normalize(vec3(0.8, -0.5, 0.6));
    
    // Normalize the fragment normal vector
    vec3 normal = normalize(fragNormal);
    
    // Light intensity using Lambertian model
    float lightIntensity = max(dot(normal, -lightDirection), 0.0);
    
    outColor = vec4(fragColor.rgb * lightIntensity, 1.0);
}
