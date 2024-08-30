// Uncomment these following global attributes to silence most warnings of "low" interest:
/*
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(unreachable_code)]
#![allow(unused_mut)]
#![allow(unused_unsafe)]
#![allow(unused_variables)]
*/
extern crate nalgebra_glm as glm;
use std::{ mem, ptr, os::raw::c_void };
use std::thread;
use std::sync::{Mutex, Arc, RwLock};

mod shader;
mod util;

use glutin::event::{Event, WindowEvent, DeviceEvent, KeyboardInput, ElementState::{Pressed, Released}, VirtualKeyCode::{self, *}};
use glutin::event_loop::ControlFlow;

// initial window size
const INITIAL_SCREEN_W: u32 = 800;
const INITIAL_SCREEN_H: u32 = 600;

// == // Helper functions to make interacting with OpenGL a little bit prettier. You *WILL* need these! // == //

// Get the size of an arbitrary array of numbers measured in bytes
// Example usage:  byte_size_of_array(my_array)
fn byte_size_of_array<T>(val: &[T]) -> isize {
    std::mem::size_of_val(&val[..]) as isize
}

// Get the OpenGL-compatible pointer to an arbitrary array of numbers
// Example usage:  pointer_to_array(my_array)
fn pointer_to_array<T>(val: &[T]) -> *const c_void {
    &val[0] as *const T as *const c_void
}

// Get the size of the given type in bytes
// Example usage:  size_of::<u64>()
fn size_of<T>() -> i32 {
    mem::size_of::<T>() as i32
}

// Get an offset in bytes for n units of type T, represented as a relative pointer
// Example usage:  offset::<u64>(4)
fn offset<T>(n: u32) -> *const c_void {
    (n * mem::size_of::<T>() as u32) as *const T as *const c_void
}

// Get a null pointer (equivalent to an offset of 0)
// ptr::null()


// == // Generate your VAO here
unsafe fn create_vao(vertices: &Vec<f32>, indices: &Vec<u32>) -> u32 {
    // Specify how many objects we want to go into VAO
    let triangle_count: i32 = (vertices.len()/9) as i32; // Calculates how many triangles were passed into the function

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
    gl::GenVertexArrays(triangle_count, &mut vao_id); 
    /*
     Bind VAO
     Here we just specify where our VAO ID is located at 
     This will allow us later to link VBO to shaders using VAO, as VAO will be bound
     */
    gl::BindVertexArray(vao_id);

    // * Generate a VBO and bind it (Vertex Buffer Object)
    /*
     This step is very similar to the VAO generation, only with VBO ID instead and binding that
     
     Where it differs is the Binding process
     As the VBO is a buffer that will hold all the data that will go to VAO, we need to specify target type of buffer
     We are going to be using very basic ARRAY type buffer for all our data storage
     There are other but I have no idea what they do, supposedly better performance and space usage for different data buffer types
     */
    let mut vbo_id: u32 = 0;
    gl::GenBuffers(triangle_count, &mut vbo_id);
    gl::BindBuffer(gl::ARRAY_BUFFER, vbo_id);

    // * Fill it with data
    /*
     Here we fill the VBO with data by calling a function
     Then we specify data we want to fill teh VBO with

     NOTE: Only 1 VBO ID can be filled at the time, so to fill multiple VBO we need to bind different VBO ID

     We must specify what kind of data the VBO should have, it should be the same as the VBO itself obviously lol
     Then we specify size of the data we fill VBO with, we specify in BYTES as memory is stored in BYTES (Must remember to say)
     Then we point to the data we want to fill VBO with, using pointers to pint to the memory location
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
     Specify if we want to normalize the data, we don't, that is just cursed unless you ware working with very big and large values at the same time, which we don't
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

    // * Generate a IBO and bind it (Indices Buffer Object)
    /*
     Here we generate IBO and bind it

     Even though we now have Vertex connection to the shaders
     We need to index these Vertexes and specify how they are connected to each other to make primitives
     For each of our 3 Vertexes, there must be created a primitive => Triangle per 3 vertexes
     IBO tells OpenGL how these vertexes are combined to make a primitive

     This is not necessary with just a single triangle in practice
     However once one starts to create multiple triangles that interconnect, this becomes a crucial step
     This step ensures and check that all the primitive Triangles are created in the most optimal way with our specifications and uses teh least amount of recourses
     How IBO helps us is for example with 2 Triangles that are sharing the same border, instead of defining this border twice (once per triangle), we can define this border once and link it to bot primitives that share that same border
     This way rendering happens more efficiently and more structured

     Very similar to VBO, just that now we specify in the command that we want to fill IBO instead
     ELEMENT_ARRAY_BUFFER is the one responsible for this
     Now instead of Vertexes, we specify for Indices, same process as with VBO
     */
    let mut ibo_id: u32 = 0;
    gl::GenBuffers(triangle_count, &mut ibo_id);
    gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ibo_id);

    // * Fill it with data
    /*
     Here we will the IBO with indices data

     Very similar to VBO
     However, its a bit more simpler as we don't have to specify indices attributes
     We only need to put inn the data, the VAP already specified how Vertexes are connected to shaders, and thus indirectly how indices should be connected 
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
    return vao_id
}


fn main() {
    // Set up the necessary objects to deal with windows and event handling
    let el = glutin::event_loop::EventLoop::new();
    let wb = glutin::window::WindowBuilder::new()
        .with_title("Gloom-rs")
        .with_resizable(true)
        .with_inner_size(glutin::dpi::LogicalSize::new(INITIAL_SCREEN_W, INITIAL_SCREEN_H));
    let cb = glutin::ContextBuilder::new()
        .with_vsync(true);
    let windowed_context = cb.build_windowed(wb, &el).unwrap();
    // Uncomment these if you want to use the mouse for controls, but want it to be confined to the screen and/or invisible.
    // windowed_context.window().set_cursor_grab(true).expect("failed to grab cursor");
    // windowed_context.window().set_cursor_visible(false);

    // Set up a shared vector for keeping track of currently pressed keys
    let arc_pressed_keys = Arc::new(Mutex::new(Vec::<VirtualKeyCode>::with_capacity(10)));
    // Make a reference of this vector to send to the render thread
    let pressed_keys = Arc::clone(&arc_pressed_keys);

    // Set up shared tuple for tracking mouse movement between frames
    let arc_mouse_delta = Arc::new(Mutex::new((0f32, 0f32)));
    // Make a reference of this tuple to send to the render thread
    let mouse_delta = Arc::clone(&arc_mouse_delta);

    // Set up shared tuple for tracking changes to the window size
    let arc_window_size = Arc::new(Mutex::new((INITIAL_SCREEN_W, INITIAL_SCREEN_H, false)));
    // Make a reference of this tuple to send to the render thread
    let window_size = Arc::clone(&arc_window_size);

    // Spawn a separate thread for rendering, so event handling doesn't block rendering
    let render_thread = thread::spawn(move || {
        // Acquire the OpenGL Context and load the function pointers.
        // This has to be done inside of the rendering thread, because
        // an active OpenGL context cannot safely traverse a thread boundary
        let context = unsafe {
            let c = windowed_context.make_current().unwrap();
            gl::load_with(|symbol| c.get_proc_address(symbol) as *const _);
            c
        };

        let mut window_aspect_ratio = INITIAL_SCREEN_W as f32 / INITIAL_SCREEN_H as f32;

        // Set up openGL
        unsafe {
            gl::Enable(gl::DEPTH_TEST);
            gl::DepthFunc(gl::LESS);
            gl::Enable(gl::CULL_FACE);
            gl::Disable(gl::MULTISAMPLE);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            gl::Enable(gl::DEBUG_OUTPUT_SYNCHRONOUS);
            gl::DebugMessageCallback(Some(util::debug_callback), ptr::null());

            // Print some diagnostics
            println!("{}: {}", util::get_gl_string(gl::VENDOR), util::get_gl_string(gl::RENDERER));
            println!("OpenGL\t: {}", util::get_gl_string(gl::VERSION));
            println!("GLSL\t: {}", util::get_gl_string(gl::SHADING_LANGUAGE_VERSION));
        }

        // * Set up Vertices
        /*
         Because of teh way simulating objects visually work, the vector becomes an array, ie v_computer=v_on_paper.Transposed
         */
        let vertices: Vec<f32> = vec![
            // Triangle 1
            (-0.6), (-0.6), 0.0, // v0
              0.6 , (-0.6), 0.0, // v1
              0.0 ,   0.6 , 0.0, // v2

            // Triangle 2
            (-0.9),   0.0 , 0.0, // v3
            (-0.7),   0.0 , 0.0, // v4
            (-0.8),   0.2 , 0.0, // v5

            // Triangle 3
              0.7 ,   0.0 , 0.0, // v6
              0.9 ,   0.0 , 0.0, // v7
              0.8 ,   0.2 , 0.0, // v8
            
            // Triangle 4
            (-0.9), (-0.1), 0.0, // v9
            (-0.8), (-0.3), 0.0, // v10
            (-0.7), (-0.1), 0.0, // v11

            // Triangle 5
              0.7 , (-0.1), 0.0, // v12
              0.8 , (-0.3), 0.0, // v13
              0.9 , (-0.1), 0.0, // v14
        ];

        // * Set up Indices
        let indices: Vec<u32> = vec![
            0 , 1 , 2 , // Triangle 1 (v0 , v1 , v2 )
            3 , 4 , 5 , // Triangle 2 (v3 , v4 , v5 )
            6 , 7 , 8 , // Triangle 3 (v6 , v7 , v8 )
            9 , 10, 11, // Triangle 4 (v9 , v10, v11)
            12, 13, 14, // Triangle 4 (v12, v13, v14)
        ];

        // * Set up VAO
        let vao_id: u32 = unsafe { 
            create_vao(&vertices, &indices)          
        };


        // * Load, Compile and Link the shader pair
        let simple_shader = unsafe {
            shader::ShaderBuilder::new()
            .attach_file("shaders/simple.vert")
            .attach_file("shaders/simple.frag")
                .link()
        };

        // * Activate the shader program
        unsafe {
            simple_shader.activate();
        }

        // Basic usage of shader helper:
        // The example code below creates a 'shader' object.
        // It which contains the field `.program_id` and the method `.activate()`.
        // The `.` in the path is relative to `Cargo.toml`.
        // This snippet is not enough to do the exercise, and will need to be modified (outside
        // of just using the correct path), but it only needs to be called once

        /*
        let simple_shader = unsafe {
            shader::ShaderBuilder::new()
                .attach_file("./path/to/simple/shader.file")
                .link()
        };
        */


        // Used to demonstrate keyboard handling for exercise 2.
        let mut _arbitrary_number = 0.0; // feel free to remove


        // The main rendering loop
        let first_frame_time = std::time::Instant::now();
        let mut previous_frame_time = first_frame_time;
        loop {
            // Compute time passed since the previous frame and since the start of the program
            let now = std::time::Instant::now();
            let elapsed = now.duration_since(first_frame_time).as_secs_f32();
            let delta_time = now.duration_since(previous_frame_time).as_secs_f32();
            previous_frame_time = now;

            // Handle resize events
            if let Ok(mut new_size) = window_size.lock() {
                if new_size.2 {
                    context.resize(glutin::dpi::PhysicalSize::new(new_size.0, new_size.1));
                    window_aspect_ratio = new_size.0 as f32 / new_size.1 as f32;
                    (*new_size).2 = false;
                    println!("Window was resized to {}x{}", new_size.0, new_size.1);
                    unsafe { gl::Viewport(0, 0, new_size.0 as i32, new_size.1 as i32); }
                }
            }

            // Handle keyboard input
            if let Ok(keys) = pressed_keys.lock() {
                for key in keys.iter() {
                    match key {
                        // The `VirtualKeyCode` enum is defined here:
                        //    https://docs.rs/winit/0.25.0/winit/event/enum.VirtualKeyCode.html

                        VirtualKeyCode::A => {
                            _arbitrary_number += delta_time;
                        }
                        VirtualKeyCode::D => {
                            _arbitrary_number -= delta_time;
                        }


                        // default handler:
                        _ => { }
                    }
                }
            }
            // Handle mouse movement. delta contains the x and y movement of the mouse since last frame in pixels
            if let Ok(mut delta) = mouse_delta.lock() {

                // == // Optionally access the accumulated mouse movement between
                // == // frames here with `delta.0` and `delta.1`

                *delta = (0.0, 0.0); // reset when done
            }

            // == // Please compute camera transforms here (exercise 2 & 3)


            unsafe {
                // Clear the color and depth buffers
                gl::ClearColor(0.035, 0.046, 0.078, 1.0); // night sky
                gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT); // Clear the screen


                // * Draw objects
                /*
                 We draw elements/object on the screen
                 
                 We specify that the element we want to draw is a triangle so that OpenGL can be prepared for drawing triangle primitives
                 Then we specify the length of the indices array to tell OpenGL how to draw the triangle 
                 Then de declare the datatype that the indices are using
                 Then finally we say 0 for the shift in the array of IBO, as we don't have any exotic IBO
                 */
                unsafe {
                    let indices_array_length: i32 = indices.len() as i32;

                    gl::DrawElements(
                        gl::TRIANGLES,
                        indices_array_length,
                        gl::UNSIGNED_INT,
                        std::ptr::null()
                    );
                }
            }

            // Display the new color buffer on the display
            context.swap_buffers().unwrap(); // we use "double buffering" to avoid artifacts
        }
    });


    // == //
    // == // From here on down there are only internals.
    // == //


    // Keep track of the health of the rendering thread
    let render_thread_healthy = Arc::new(RwLock::new(true));
    let render_thread_watchdog = Arc::clone(&render_thread_healthy);
    thread::spawn(move || {
        if !render_thread.join().is_ok() {
            if let Ok(mut health) = render_thread_watchdog.write() {
                println!("Render thread panicked!");
                *health = false;
            }
        }
    });

    // Start the event loop -- This is where window events are initially handled
    el.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        // Terminate program if render thread panics
        if let Ok(health) = render_thread_healthy.read() {
            if *health == false {
                *control_flow = ControlFlow::Exit;
            }
        }

        match event {
            Event::WindowEvent { event: WindowEvent::Resized(physical_size), .. } => {
                println!("New window size received: {}x{}", physical_size.width, physical_size.height);
                if let Ok(mut new_size) = arc_window_size.lock() {
                    *new_size = (physical_size.width, physical_size.height, true);
                }
            }
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                *control_flow = ControlFlow::Exit;
            }
            // Keep track of currently pressed keys to send to the rendering thread
            Event::WindowEvent { event: WindowEvent::KeyboardInput {
                    input: KeyboardInput { state: key_state, virtual_keycode: Some(keycode), .. }, .. }, .. } => {

                if let Ok(mut keys) = arc_pressed_keys.lock() {
                    match key_state {
                        Released => {
                            if keys.contains(&keycode) {
                                let i = keys.iter().position(|&k| k == keycode).unwrap();
                                keys.remove(i);
                            }
                        },
                        Pressed => {
                            if !keys.contains(&keycode) {
                                keys.push(keycode);
                            }
                        }
                    }
                }

                // Handle Escape and Q keys separately
                match keycode {
                    Escape => { *control_flow = ControlFlow::Exit; }
                    Q      => { *control_flow = ControlFlow::Exit; }
                    _      => { }
                }
            }
            Event::DeviceEvent { event: DeviceEvent::MouseMotion { delta }, .. } => {
                // Accumulate mouse movement
                if let Ok(mut position) = arc_mouse_delta.lock() {
                    *position = (position.0 + delta.0 as f32, position.1 + delta.1 as f32);
                }
            }
            _ => { }
        }
    });
}
