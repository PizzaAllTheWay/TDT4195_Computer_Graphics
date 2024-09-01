#version 460 core

// Uniform variable for changing the color dynamically
uniform vec3 ChangingColor; // Uniform for dynamic color modulation
uniform float scale;        // Scale of the checkerboard squares

// Input from the vertex shader
in vec2 TexCoords;  // Texture coordinates from the vertex shader

// Output to the framebuffer
// This variable represents the final color that will be written to the screen (or to the framebuffer).
out vec4 color; // Final output color (r, g, b, a)

void main()
{
    // Scale the texture coordinates to control the size of the checkerboard squares
    vec2 scaledCoords = TexCoords * scale;

    // Determine the checkerboard color by using a step function on the sine of the scaled coordinates
    float checkerPattern = mod(floor(scaledCoords.x) + floor(scaledCoords.y), 2.0);

    // Define the two colors for the checkerboard pattern
    vec3 color1 = vec3(1.0, 1.0, 1.0); // White color
    vec3 color2 = vec3(0.3, 0.3, 0.3); // Inverse color

    // Mix the two colors based on the checkerPattern value (0.0 or 1.0)
    vec3 checkerboardColor = mix(color1, color2, checkerPattern);

    // Combine the checkerboard pattern with the dynamic color modulation
    vec3 finalColor = checkerboardColor * ChangingColor;

    // Apply the final color with an alpha value of 1.0
    color = vec4(finalColor, 0.3f);
}
