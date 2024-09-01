#version 460 core

// Input vertex attributes
// The 'location = 0' specifies that this variable will receive data from the first attribute (position) in the VAO.
layout(location = 0) in vec3 position; // Vertex position (x, y, z coordinates)

// The 'location = 1' specifies that this variable will receive data from the second attribute (color) in the VAO.
layout(location = 1) in vec3 vertexColor; // Vertex color (r, g, b values)

layout(location = 2) in vec2 texCoords; // Texture coordinates

// Output to the fragment shader
// This variable will carry the color data to the fragment shader.
out vec3 fragColor; // Output color to the fragment shader

out vec2 TexCoords;     // Output texture coordinates to the fragment shader

// Uniform variables
// These are matrices that will be provided by the application, typically used to transform the vertex positions.
uniform mat4 model; // Model matrix: Transforms object space coordinates to world space
uniform mat4 view;  // View matrix: Transforms world space coordinates to camera (view) space
uniform mat4 projection; // Projection matrix: Transforms camera space coordinates to clip space

void main()
{
    // The main function is called for each vertex processed by the GPU.

    // Transform the vertex position using the model, view, and projection matrices.
    // The 'vec4(position, 1.0)' converts the 3D position to a 4D vector (homogeneous coordinates) with w = 1.0.
    // This is necessary because the transformation matrices (model, view, projection) are 4x4 matrices.
    // gl_Position = projection * view * model * vec4(position, 1.0);

    // Directly passing the position to gl_Position for now
    gl_Position = vec4(position, 1.0);
    // The result, gl_Position, is a special built-in variable that determines where the vertex will appear on the screen.

    // Pass the vertex color to the fragment shader.
    // 'fragColor' is the output variable that will be passed to the fragment shader.
    fragColor = vertexColor; // The vertex color is passed along without modification.

    TexCoords = texCoords;
}
