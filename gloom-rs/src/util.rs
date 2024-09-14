use std::{ffi::CString, mem, os::raw::c_void, path::Path};
use libc;

pub unsafe fn get_gl_string(name: gl::types::GLenum) -> String {
    std::ffi::CStr::from_ptr(gl::GetString(name) as *mut libc::c_char).to_string_lossy().to_string()
}

// Debug callback to panic upon encountering any OpenGL error
pub extern "system" fn debug_callback(
    source: u32, e_type: u32, id: u32,
    severity: u32, _length: i32,
    msg: *const libc::c_char, _data: *mut std::ffi::c_void
) {
    if e_type != gl::DEBUG_TYPE_ERROR { return }
    if severity == gl::DEBUG_SEVERITY_HIGH ||
       severity == gl::DEBUG_SEVERITY_MEDIUM ||
       severity == gl::DEBUG_SEVERITY_LOW
    {
        let severity_string = match severity {
            gl::DEBUG_SEVERITY_HIGH => "high",
            gl::DEBUG_SEVERITY_MEDIUM => "medium",
            gl::DEBUG_SEVERITY_LOW => "low",
            _ => "unknown",
        };
        unsafe {
            let string = CString::from_raw(msg as *mut libc::c_char);
            let error_message = String::from_utf8_lossy(string.as_bytes()).to_string();
            panic!("{}: Error of severity {} raised from {}: {}\n",
                id, severity_string, source, error_message);
        }
    }
}

// Get the size of an arbitrary array of numbers measured in bytes
// Example usage:  byte_size_of_array(my_array)
pub fn byte_size_of_array<T>(val: &[T]) -> isize {
    std::mem::size_of_val(&val[..]) as isize
}

// Get the OpenGL-compatible pointer to an arbitrary array of numbers
// Example usage:  pointer_to_array(my_array)
pub fn pointer_to_array<T>(val: &[T]) -> *const c_void {
    &val[0] as *const T as *const c_void
}

// Get the size of the given type in bytes
// Example usage:  size_of::<u64>()
pub fn size_of<T>() -> i32 {
    mem::size_of::<T>() as i32
}

// Get an offset in bytes for n units of type T, represented as a relative pointer
// Example usage:  offset::<u64>(4)
pub fn offset<T>(n: u32) -> *const c_void {
    (n * mem::size_of::<T>() as u32) as *const T as *const c_void
}

// Get a null pointer (equivalent to an offset of 0)
pub fn null() -> *const c_void {
    std::ptr::null()
}

// * Load .obj files to normalized vertices and correct indices
pub fn load_obj(filename: &str) -> (Vec<f32>, Vec<f32>, Vec<f32>, Vec<u32>) {
    // Set load options to triangulate the mesh
    let load_options = tobj::LoadOptions {
        triangulate: true,
        ..Default::default() // Use the default settings for other options
    };

    // Load the OBJ file with the specified options
    let obj = tobj::load_obj(&Path::new(filename), &load_options);
    
    let (models, _) = obj.expect("Failed to load OBJ file");

    // Initialize vectors to store vertices, normals, texture coordinates, and indices
    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut texcoords = Vec::new();
    let mut indices = Vec::new();

    // Iterate through the models and store the data
    for m in models.iter() {
        let mesh = &m.mesh;

        // Store vertices
        for v in mesh.positions.chunks(3) {
            let vertex = glm::vec3(v[0], v[1], v[2]);

            vertices.push(vertex.x);
            vertices.push(vertex.y);
            vertices.push(vertex.z);
        }

        // Store normals
        if !mesh.normals.is_empty() {
            normals.extend_from_slice(&mesh.normals);
        }

        // Store texture coordinates
        if !mesh.texcoords.is_empty() {
            texcoords.extend_from_slice(&mesh.texcoords);
        }

        // Store indices
        indices.extend_from_slice(&mesh.indices);
    }

    (vertices, normals, texcoords, indices)
}




// * Generate VAO (Vertex Array Object)
pub unsafe fn create_vao(
    vertices: &Vec<f32>, 
    indices: &Vec<u32>, 
    colors: &Vec<f32>,
    texcoords: &Vec<f32>
) -> (u32, u32) {
    // * Generate a VAO and bind it (Vertex Array Object)
    /*
     Specify VAO ID
     This is how we will interact with our VAO, not directly, but through the ID
     This is just how OpenGL pipeline is built to be interacted with

     We also make sure to use 32 bit data structures as this is the most common for OpenGL pipeline
     I don't want to break and debug stuff so we keep everything like that
     */
    let mut vao_id: u32 = 0; 
    /*
     Generate VAO, 
     This is where we generate the IDs as well, so it needs to be pointed to in memory
     */
    gl::GenVertexArrays(1, &mut vao_id); 
    /*
     Bind VAO
     Here we just specify where our VAO ID is located at 
     This will allow us later to link VBO to shaders using VAO, as VAO will be bound
     */
    gl::BindVertexArray(vao_id);

    // * Generate a VBO and bind it (Vertex Buffer Object) for vertices
    /*
     This step is very similar to the VAO generation, only with VBO ID instead and binding that
     
     Where it differs is the Binding process
     As the VBO is a buffer that will hold all the data that will go to VAO, we need to specify target type of buffer
     We are going to be using very basic ARRAY type buffer for all our data storage
     There are other but I have no idea what they do, supposedly better performance and space usage for different data buffer types
     */
    let mut vbo_id: u32 = 0;
    gl::GenBuffers(1, &mut vbo_id);
    gl::BindBuffer(gl::ARRAY_BUFFER, vbo_id);

    // * Fill it with data for vertices
    /*
     Here we fill the VBO with data by calling a function
     Then we specify data we want to fill the VBO with

     NOTE: Only 1 VBO ID can be filled at the time, so to fill multiple VBO we need to bind different VBO IDs

     We must specify what kind of data the VBO should have, it should be the same as the VBO itself obviously lol
     Then we specify size of the data we fill VBO with, we specify in BYTES as memory is stored in BYTES (Must remember to say)
     Then we point to the data we want to fill VBO with, using pointers to point to the memory location
     Then finally we will update usage of this VBO data, in our case STATIC_DRAW as we don't change the triangle often if at all 
     
     (Many other complex usages here for better performance when rendering, however we stick with basics cuz this is getting confusing for me lol)
     */
    gl::BufferData(
        gl::ARRAY_BUFFER,
        byte_size_of_array(vertices),
        pointer_to_array(vertices),
        gl::STATIC_DRAW
    );

    // * Configure a VAP for the data and enable it (Vertex Attribute Pointer)
    /*
     Here we configure VAP and enabling it by calling a function

     VAP Will specify what type of data in what type of data structure we passed down to VBO
     VAP Will then specify how shaders should interpret VBO and which vertex shaders it should be associated with 
     Since we only have triangles this specification should be straight forward 
     
     Specify position/index of the vertex attribute in shader program corresponds to the data passing through VBO. Since we only have triangles, this is very generic as there is only 1 Vertex Shader located at "in layout(location=0) vec3 vertex;", ie location = 0
     Specify number of components per Vertex. Each vertex consists of 3 floats (32 bits) (x, y, z). So that is why 3 
     Specify data type of each component, generic 32 bit floats as OpenGL likes it jesjes
     Specify if we want to normalize the data, we don't, that is just cursed unless you are working with very big and large values at the same time, which we don't
     Specify Stride: number of bytes between each new vertex (3 32-bit floats per vertex)
     Specify offset of the first component (should always be 0, otherwise what kind of data structure are we even handling X-X) 
     
     Lastly We enable VAP :)
     */
    let position_attribute_index: u32 = 0;
    let number_of_vertexes_per_triangle: i32 = 3;
    let stride: i32 = number_of_vertexes_per_triangle * size_of::<f32>();
    gl::VertexAttribPointer(
        position_attribute_index,
        number_of_vertexes_per_triangle,
        gl::FLOAT,
        gl::FALSE,
        stride,
        std::ptr::null()
    );
    gl::EnableVertexAttribArray(position_attribute_index); // Array/Pointer, same stuff at the end of the day, just some renaming, still enables VAP

    // * Generate a VBO and bind it (Vertex Buffer Object) for colors
    /*
     Here we generate a second VBO, this time for the vertex colors.
     The process is identical to generating the VBO for vertices.
     */
    if !colors.is_empty() {
        let mut vbo_id_color: u32 = 0;
        gl::GenBuffers(1, &mut vbo_id_color);
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo_id_color);
        gl::BufferData(
            gl::ARRAY_BUFFER,
            byte_size_of_array(colors),
            pointer_to_array(colors),
            gl::STATIC_DRAW,
        );
    }

    // * Configure a VAP for the color data and enable it
    /*
     Same as for the vertex data, but now for colors.
     This VAP will be linked to the color attribute in the shader.
     
     Specify position/index of the color attribute in shader program corresponds to the data passing through VBO. Since we only have triangles this specification should be straight forward 
     Since we have colors, we assume there are 3 components per color (R, G, B). So that is why 3 
     Specify data type of each component, generic 32 bit floats as OpenGL likes it
     Specify if we want to normalize the data, we don't, that is just cursed unless you are working with very big and large values at the same time, which we don't
     Specify Stride: number of bytes between each new color (3 32-bit floats per color)
     Specify offset of the first component (should always be 0, otherwise what kind of data structure are we even handling X-X) 
     
     Lastly We enable VAP :)
     */
    if !colors.is_empty() {
        let color_attribute_index: u32 = 1;
        let color_components_per_vertex = if colors.len() % 4 == 0 { 4 } else { 3 }; // Directly check if RGBA or RGB

        gl::VertexAttribPointer(
            color_attribute_index,
            color_components_per_vertex,
            gl::FLOAT,
            gl::FALSE,
            color_components_per_vertex * size_of::<f32>(),
            std::ptr::null(),
        );
        gl::EnableVertexAttribArray(color_attribute_index);
    }

    // * Generate a VBO and bind it (Vertex Buffer Object) for texture coordinates
    if !texcoords.is_empty() {
        let mut vbo_id_texcoords: u32 = 0;
        gl::GenBuffers(1, &mut vbo_id_texcoords);
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo_id_texcoords);
        gl::BufferData(
            gl::ARRAY_BUFFER,
            byte_size_of_array(texcoords),
            pointer_to_array(texcoords),
            gl::STATIC_DRAW,
        );
    }

    // * Configure a VAP for the texture coordinates and enable it
    if !texcoords.is_empty() {
        let texcoord_attribute_index: u32 = 2; // Assuming this is at location 2 in your shader
        gl::VertexAttribPointer(
            texcoord_attribute_index,
            2,  // Texture coordinates have 2 components (u, v)
            gl::FLOAT,
            gl::FALSE,
            2 * size_of::<f32>(),
            std::ptr::null(),
        );
        gl::EnableVertexAttribArray(texcoord_attribute_index);
    }

    // * Generate a IBO and bind it (Indices Buffer Object)
    /*
     Here we generate IBO and bind it

     Even though we now have Vertex connection to the shaders
     We need to index these Vertexes and specify how they are connected to each other to make primitives
     For each of our 3 Vertexes, there must be created a primitive => Triangle per 3 vertexes
     IBO tells OpenGL how these vertexes are combined to make a primitive

     This is not necessary with just a single triangle in practice
     However once one starts to create multiple triangles that interconnect, this becomes a crucial step
     This step ensures and checks that all the primitive Triangles are created in the most optimal way with our specifications and uses the least amount of resources
     How IBO helps us is for example with 2 Triangles that are sharing the same border, instead of defining this border twice (once per triangle), we can define this border once and link it to both primitives that share that same border
     This way rendering happens more efficiently and more structured

     Very similar to VBO, just that now we specify in the command that we want to fill IBO instead
     ELEMENT_ARRAY_BUFFER is the one responsible for this
     Now instead of Vertexes, we specify for Indices, same process as with VBO
     */
    let mut ibo_id: u32 = 0;
    gl::GenBuffers(1, &mut ibo_id);
    gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ibo_id);

    // * Fill it with data
    /*
     Here we fill the IBO with indices data

     Very similar to VBO
     However, it is a bit simpler as we don't have to specify indices attributes
     We only need to put in the data, the VAP already specified how Vertexes are connected to shaders, and thus indirectly how indices should be connected 
     This is because indices describe how Vertexes are connected to each other to build a primitive, in our case triangles

     Very similar to VBO, just that now we specify in the command that we want to fill IBO instead
     ELEMENT_ARRAY_BUFFER is the one responsible for this
     Now instead of Vertexes, we specify for Indices, same process as with VBO
    */
    gl::BufferData(
        gl::ELEMENT_ARRAY_BUFFER,
        byte_size_of_array(indices),
        pointer_to_array(indices),
        gl::STATIC_DRAW
    );

    // * Return the ID of the VAO
    return (vao_id, vbo_id)
}

// * Update VAO with new vertices
pub unsafe fn update_vao_with_new_vertices(vao_id: u32, vertex_buffer_id: u32, vertices: &Vec<f32>) {
    // 1. Bind the VAO
    gl::BindVertexArray(vao_id);

    // 2. Bind the existing VBO
    gl::BindBuffer(gl::ARRAY_BUFFER, vertex_buffer_id);

    // 3. Reallocate and fill the buffer with new data
    gl::BufferData(
        gl::ARRAY_BUFFER,
        byte_size_of_array(vertices),
        pointer_to_array(vertices),
        gl::STATIC_DRAW,
    );

    // 4. Unbind the VAO to prevent accidental modification
    gl::BindVertexArray(0);
}



// * Scaling Transform
pub fn scale_vertices(vertices: &Vec<f32>, scale_x: f32, scale_y: f32, scale_z: f32) -> Vec<f32> {
    // Create a scaling matrix using glm
    let scaling_matrix = glm::scaling(&glm::vec3(scale_x, scale_y, scale_z));
    
    // Create a new vector to store the scaled vertices
    let mut scaled_vertices: Vec<f32> = Vec::with_capacity(vertices.len());

    // Iterate through each vertex in the vertices vector
    for i in 0..(vertices.len() / 3) {
        // Create a vec4 from the vertex position, with w = 1 for homogeneous coordinates
        let vertex = glm::vec4(vertices[3 * i], vertices[3 * i + 1], vertices[3 * i + 2], 1.0);

        // Apply the scaling transformation
        let scaled_vertex = scaling_matrix * vertex;

        // Store the scaled vertex into the new vector
        scaled_vertices.push(scaled_vertex.x);
        scaled_vertices.push(scaled_vertex.y);
        scaled_vertices.push(scaled_vertex.z);
    }

    return scaled_vertices;
}

// * Rotation Transfomations
/// Function to calculate a rotation matrix around the X-axis
pub fn rotation_matrix_x(angle: f32) -> glm::Mat4 {
    glm::rotation(angle, &glm::vec3(1.0, 0.0, 0.0))
}

/// Function to calculate a rotation matrix around the Y-axis
pub fn rotation_matrix_y(angle: f32) -> glm::Mat4 {
    glm::rotation(angle, &glm::vec3(0.0, 1.0, 0.0))
}

/// Function to calculate a rotation matrix around the Z-axis
pub fn rotation_matrix_z(angle: f32) -> glm::Mat4 {
    glm::rotation(angle, &glm::vec3(0.0, 0.0, 1.0))
}

/// Function to apply a rotation to an array of vertices.
/// - `vertices`: The input array of vertex positions.
/// - `rotation_x`: Rotation around the X-axis in radians.
/// - `rotation_y`: Rotation around the Y-axis in radians.
/// - `rotation_z`: Rotation around the Z-axis in radians.
/// Returns a new array of rotated vertices.
pub fn rotate_vertices(
    vertices: &Vec<f32>,
    rotation_x: f32,
    rotation_y: f32,
    rotation_z: f32
) -> Vec<f32> {
    // Step 1: Calculate the individual rotation matrices
    let rotation_matrix_x = rotation_matrix_x(rotation_x);
    let rotation_matrix_y = rotation_matrix_y(rotation_y);
    let rotation_matrix_z = rotation_matrix_z(rotation_z);

    // Step 2: Combine the rotation matrices
    let rotation_matrix = rotation_matrix_z * rotation_matrix_y * rotation_matrix_x;

    // Step 3: Apply the combined rotation matrix to each vertex
    let mut rotated_vertices: Vec<f32> = Vec::with_capacity(vertices.len());

    for i in 0..(vertices.len() / 3) {
        // Create a vec4 from the vertex position, with w = 1 for homogeneous coordinates
        let vertex = glm::vec4(vertices[3 * i], vertices[3 * i + 1], vertices[3 * i + 2], 1.0);

        // Apply the rotation transformation
        let rotated_vertex = rotation_matrix * vertex;

        // Store the rotated vertex into the new vector
        rotated_vertices.push(rotated_vertex.x);
        rotated_vertices.push(rotated_vertex.y);
        rotated_vertices.push(rotated_vertex.z);
    }

    // Return the new array of rotated vertices
    rotated_vertices
}

// * Translation Transformation
/// Function to calculate a translation matrix
pub fn translation_matrix(translate_x: f32, translate_y: f32, translate_z: f32) -> glm::Mat4 {
    glm::translation(&glm::vec3(translate_x, translate_y, translate_z))
}

/// Function to apply a translation to an array of vertices.
/// - `vertices`: The input array of vertex positions.
/// - `translate_x`: Translation along the X-axis.
/// - `translate_y`: Translation along the Y-axis.
/// - `translate_z`: Translation along the Z-axis.
/// Returns a new array of translated vertices.
pub fn translate_vertices(
    vertices: &Vec<f32>,
    translate_x: f32,
    translate_y: f32,
    translate_z: f32
) -> Vec<f32> {
    // Create a translation matrix
    let translation_matrix = translation_matrix(translate_x, translate_y, translate_z);

    // Create a new vector to store the translated vertices
    let mut translated_vertices: Vec<f32> = Vec::with_capacity(vertices.len());

    // Iterate through each vertex and apply the translation
    for i in 0..(vertices.len() / 3) {
        // Create a vec4 from the vertex position, with w = 1 for homogeneous coordinates
        let vertex = glm::vec4(vertices[3 * i], vertices[3 * i + 1], vertices[3 * i + 2], 1.0);

        // Apply the translation transformation
        let translated_vertex = translation_matrix * vertex;

        // Store the translated vertex into the new vector
        translated_vertices.push(translated_vertex.x);
        translated_vertices.push(translated_vertex.y);
        translated_vertices.push(translated_vertex.z);
    }

    translated_vertices
}


// * For calculating direction of camera 
// So that WASD responds to camera movement
// This means even if we are 180*, ie backwards, when we press W (froward)
// Without this function => We would go backwards
// With this function, camera view knows we are back so it takes this into account when transforming 
// Meaning we will move forward as intended
pub fn calculate_direction(yaw: f32, pitch: f32) -> glm::Vec3 {
    glm::vec3(
        yaw.cos() * pitch.cos(),
        pitch.sin(),
        yaw.sin() * pitch.cos(),
    )
}

pub fn calculate_right_vector(yaw: f32) -> glm::Vec3 {
    glm::vec3(yaw.sin(), 0.0, -yaw.cos()).normalize()
}

pub fn calculate_up_vector(forward: glm::Vec3, right: glm::Vec3) -> glm::Vec3 {
    glm::cross(&right, &forward).normalize()
}


