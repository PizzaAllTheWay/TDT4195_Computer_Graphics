#version 460 core

uniform vec3 ChangingColor;

out vec4 FragColor;

void main()
{
    FragColor = vec4(ChangingColor, 1.0);
}
