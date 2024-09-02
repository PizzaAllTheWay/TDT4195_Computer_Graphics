#version 430 core

out vec4 color;

void main()
{
    // Scale factor to control the size of the checkerboard squares
    float scale = 50.0;

    // Get the x and y coordinates in pixels from gl_FragCoord
    float x = gl_FragCoord.x / scale;
    float y = gl_FragCoord.y / scale;

    // Compute the color directly using the modulo result
    // Passing this directly to the output variable
    // avoids the use of if-else statements in shader which
    // are not optimal to run on the GPU
    float checker = mod(floor(x) + floor(y), 2.0);

    // Set color based on checker value
    color = vec4(vec3(checker), 1.0);
}
