// Uncomment these following global attributes to silence most warnings of "low" interest:

#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(unreachable_code)]
#![allow(unused_mut)]
#![allow(unused_unsafe)]
#![allow(unused_variables)]

extern crate nalgebra_glm as glm;
use std::thread;
use std::sync::{Mutex, Arc, RwLock};

mod shader;
mod util;

use glm::scaling;
use glutin::event::{Event, WindowEvent, DeviceEvent, KeyboardInput, ElementState::{Pressed, Released}, VirtualKeyCode::{self, *}};
use glutin::event_loop::ControlFlow;


// initial window size
const INITIAL_SCREEN_W: u32 = 800;
const INITIAL_SCREEN_H: u32 = 600;

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
    //windowed_context.window().set_cursor_grab(true).expect("failed to grab cursor");
    //windowed_context.window().set_cursor_visible(false);

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

    // * Camera variables used in 3D scene to move camera around
    let mut camera_position = glm::vec3(0.0, 0.0, 1.0);
    let camera_speed = 2.0;
    
    let mut camera_yaw: f32 = 0.0;
    let mut camera_pitch: f32 = 0.0;
    let mouse_sensitivity: f32 = 0.005; // Mouse sensitivity for rotation
    let mut mouse_right_button_pressed = false;



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
            gl::DebugMessageCallback(Some(util::debug_callback), util::null());

            // Print some diagnostics
            println!("{}: {}", util::get_gl_string(gl::VENDOR), util::get_gl_string(gl::RENDERER));
            println!("OpenGL\t: {}", util::get_gl_string(gl::VERSION));
            println!("GLSL\t: {}", util::get_gl_string(gl::SHADING_LANGUAGE_VERSION));
        }

        // * Load Objects
        // Load Orca model
        let (vertices_orca, normals_orca, texcoords_orca, indices_orca) = util::load_obj("resources/orca.obj");
        
        // Load square model
        let (vertices_square, normals_square, texcoords_square, indices_square) = util::load_obj("resources/square.obj");

        // * Color buffer creation
        // Since many vertices in the orca model, set up an initial color buffer
        // NOTE: The Orca object is rendered with a dynamic RGB color matrix later on. 
        // The values here represent the base RGB percentages, which will be scaled by the shader's color transformation.
        // Each vertex is assigned a base color (in this case, a shade of blue with partial transparency).
        let mut color_orca: Vec<f32> = Vec::new();
        for _ in 0..indices_orca.len() {
            // Append color for each vertex
            color_orca.extend_from_slice(&[0.3, 0.6, 1.0, 0.5]);
        }

        // Add color to square
        let mut color_square: Vec<f32> = Vec::new();
        for _ in 0..indices_square.len() {
            // Append color for each vertex
            color_square.extend_from_slice(&[1.0, 0.0, 1.0, 0.8]);
        }

        // * Create data structure for particle effects
        // Concatenate vertices, colors, and indices for all particles
        let mut vertices_particles: Vec<f32> = Vec::new();
        vertices_particles.extend(vertices_square);

        let mut colors_particles: Vec<f32> = Vec::new();
        colors_particles.extend(color_square);

        // Adjust indices for each particle
        let mut indices_particles: Vec<u32> = Vec::new();
        let indices_particle1 = indices_square.clone();
        indices_particles.extend(indices_particle1);



        // * Set up VAO
        let (vao_id_orca, vbo_id_orca): (u32, u32) = unsafe { 
            util::create_vao(&vertices_orca, &indices_orca, &color_orca, &texcoords_orca)          
        };

        let (vao_id_particles, vbo_id_particles): (u32, u32) = unsafe { 
            util::create_vao(&vertices_particles, &indices_particles, &colors_particles, &texcoords_square)          
        };



        // * Load, Compile and Link the shader pair
        let shader_orca = unsafe {
            shader::ShaderBuilder::new()
                .attach_file("shaders/orca.vert")
                .attach_file("shaders/orca.frag")
                .link()
        };

        let shader_particles = unsafe {
            shader::ShaderBuilder::new()
                .attach_file("shaders/particles.vert")
                .attach_file("shaders/particles.frag")
                .link()
        };

        // The main rendering loop
        let first_frame_time = std::time::Instant::now();
        let mut previous_frame_time = first_frame_time;

        // Keep track of the last time rotation was updated
        let mut last_rotation_update = 0.0;

        loop {
            // Compute time passed since the previous frame and since the start of the program
            let now = std::time::Instant::now();
            let elapsed = now.duration_since(first_frame_time).as_secs_f32();
            let delta_time = now.duration_since(previous_frame_time).as_secs_f32();
            previous_frame_time = now;

            // Calculate the camera direction based on the yaw and pitch
            let camera_forward = util::calculate_direction(camera_yaw, camera_pitch);
            let camera_right = glm::normalize(&glm::cross(&glm::vec3(0.0, 1.0, 0.0), &camera_forward));
            let camera_up = glm::normalize(&glm::cross(&camera_forward, &camera_right));

            // Handle resize events
            if let Ok(mut new_size) = window_size.lock() {
                if new_size.2 {
                    context.resize(glutin::dpi::PhysicalSize::new(new_size.0, new_size.1));
                    // ! window_aspect_ratio = new_size.0 as f32 / new_size.1 as f32;
                    (*new_size).2 = false;
                    println!("Window was resized to {}x{}", new_size.0, new_size.1);
                    unsafe { gl::Viewport(0, 0, new_size.0 as i32, new_size.1 as i32); }
                }
            }

            // Handle keyboard input
            if let Ok(keys) = pressed_keys.lock() {
                for key in keys.iter() {
                    let movement_vector: glm::Vec3 = match key {
                        VirtualKeyCode::W => camera_forward * camera_speed * delta_time,     // Move forward
                        VirtualKeyCode::S => -camera_forward * camera_speed * delta_time,    // Move backward
                        VirtualKeyCode::A => camera_right * camera_speed * delta_time,       // Move left
                        VirtualKeyCode::D => -camera_right * camera_speed * delta_time,      // Move right
                        VirtualKeyCode::Space => camera_up * camera_speed * delta_time,      // Move up
                        VirtualKeyCode::LShift => -camera_up * camera_speed * delta_time,    // Move down
                        _ => glm::vec3(0.0, 0.0, 0.0)
                    };
    
                    // Update camera position based on movement
                    camera_position += movement_vector;
                }
            }

            // Handle mouse movement. delta contains the x and y movement of the mouse since last frame in pixels
            if let Ok(mut delta) = mouse_delta.lock() {
                camera_pitch -= delta.1 * mouse_sensitivity; // Update pitch (vertical)
                camera_yaw += delta.0 * mouse_sensitivity; // Update yaw (horizontal)

                // Clamp the pitch value to avoid excessive rotation
                camera_pitch = camera_pitch.clamp(-std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2);

                // Reset the mouse delta after applying it
                *delta = (0.0, 0.0);
            }



            // * Apply transformations to the world from camera view
            // Calculate camera perspective
            let camera_aspect_ratio = window_aspect_ratio;
            let camera_perspective_matrix: glm::Mat4 = glm::perspective(camera_aspect_ratio, 45.0_f32.to_radians(), 1.0, 100.0);
            
            // Calculate camera transformations
            // Build the view matrix based on the camera position and orientation
            let camera_rotation_matrix = glm::look_at(
                &camera_position, 
                &(camera_position + camera_forward), 
                &camera_up
            );

            // Combine the matrices
            let view_projection_matrix: glm::Mat4 = camera_perspective_matrix * camera_rotation_matrix;

            // * Animate RGB changing color Orca while its spinning and going up and down 
            // Change RGB colors
            let r: f32 = (elapsed * 0.5).sin() * 0.5 + 0.5;
            let g: f32 = (elapsed * 0.7).sin() * 0.5 + 0.5;
            let b: f32 = (elapsed * 0.9).sin() * 0.5 + 0.5;
            let a: f32 = 1.0;

            // Create a diagonal 4x4 RGBA matrix for scaling
            let rgb_vec = glm::vec4(r, g, b, a);
            let changing_color_matrix_orca = glm::diagonal4x4(&rgb_vec);

            // Compute rotation
            let orca_rotation_x: f32 = 0.0;
            let orca_rotation_y: f32 = elapsed % std::f32::consts::TAU;
            let orca_rotation_z: f32 = 0.0;
            let orca_rotation_animation_matrix = glm::rotation(orca_rotation_y, &glm::vec3(0.0, 1.0, 0.0));
            
            // Compute upp and down motion
            let orca_linear_y: f32 = 0.5 * (elapsed * 1.0).sin();
            let orca_linear_animation_matrix = glm::translation(&glm::vec3(0.0, orca_linear_y, 0.0));

            // Define Orca position in world frame
            let orca_position = &glm::vec3(5.0, 0.0, 0.0);

            // Before we do anything we scale, rotate and put object into start position
            // 1. Scale
            // 2. Rotate
            // 3. Translate
            let orca_start_transform_matrix: glm::Mat4 = 
                glm::translation(&orca_position) *
                glm::rotation(std::f32::consts::PI/2.0, &glm::vec3(0.0, 1.0, 0.0)) *
                glm::scaling(&glm::vec3(3.0, 3.0, 3.0));

            // Combine matrices to form view projection of rotating orca matrix
            // 1. Apply start position of Orca
            // 2. Translate Orca to the origin
            // 3. Apply rotation animation
            // 4. Apply translation animation
            // 5. Translate Orca back to its original position
            // 6. Apply view-projection transformation
            let view_projection_matrix_orca: glm::Mat4 = 
                view_projection_matrix * 
                glm::translation(&orca_position) *
                orca_linear_animation_matrix *
                orca_rotation_animation_matrix *
                glm::translation(&-orca_position) *
                orca_start_transform_matrix;

            // * Animate Particles to always face viewer, move around and change color
            let changing_color_matrix_particles = glm::diagonal4x4(&glm::vec4(1.0, 1.0, 1.0, 1.0));

            // Define particle position in world frame
            let particles_position = &glm::vec3(1.0, 0.0, 0.0);

            // Before we do anything we scale, rotate and put object into start position
            // 1. Scale
            // 2. Rotate
            // 3. Translate
            let particles_start_transform_matrix: glm::Mat4 = 
                glm::translation(&particles_position) *
                glm::rotation(0.0, &glm::vec3(0.0, 0.0, 0.0)) *
                glm::scaling(&glm::vec3(0.1, 0.1, 0.1));

            // Compute rotation for particles to always face the camera (START) --------------------------------------------------
            // Aka billboarding effect
            // The goal is to align the particle's orientation with the camera's view direction,
            // making it appear as though the particle always faces the camera regardless of the camera's movement.
            // Step 1: Calculate the vector from the particle to the camera.
            // This is done by subtracting the particle's position from the camera's position.
            // This vector represents the direction from the particle to the camera.
            let particle_to_camera = camera_position - particles_position;
            
            // Step 2: Normalize the particle-to-camera vector to get a direction vector.
            // The normalized vector provides the direction in standard form (unit length),
            // which is essential for calculating angles between the particle and the camera's axes.
            let particle_to_camera_direction = glm::normalize(&particle_to_camera);

            // Step 3: Calculate the yaw angle (rotation around the Y-axis).
            // We use the x and z components of the particle-to-camera direction vector
            // to determine how much the particle needs to rotate horizontally to face the camera.
            let angle_y = particle_to_camera_direction.x.atan2(particle_to_camera_direction.z);  // Yaw (rotation around Y-axis)

            // Step 4: Calculate the pitch angle (rotation around the X-axis).
            // The pitch is the vertical rotation needed for the particle to align with the camera's view.
            // We use the y component of the direction vector and the length of the x-z projection 
            // (which is the horizontal distance) to compute the vertical angle.
            let angle_x = particle_to_camera_direction.y.atan2(glm::length(&glm::vec2(particle_to_camera_direction.x, particle_to_camera_direction.z)));  // Pitch (rotation around X-axis)

            // Step 5: Create the rotation matrices for yaw and pitch.
            // These matrices will rotate the particle to face the camera in the horizontal (yaw) and vertical (pitch) directions.
            let rotation_matrix_y = glm::rotation(angle_y, &glm::vec3(0.0, 1.0, 0.0));  // Yaw rotation (around Y-axis)
            let rotation_matrix_x = glm::rotation(angle_x, &glm::vec3(1.0, 0.0, 0.0));  // Pitch rotation (around X-axis)

            // Step 6: Apply a 180-degree rotation around the Y-axis to correct the particle's orientation.
            // Since the camera is often offset by 90 degrees from the world frame (depending on the camera setup),
            // we need to rotate the particle by 180 degrees around the Y-axis so that the particle's face is correctly aligned with the camera.
            let rotation_y_180 = glm::rotation(std::f32::consts::PI, &glm::vec3(0.0, 1.0, 0.0));  // 180-degree rotation around Y-axis

            // Step 7: Combine all the rotation matrices to form the final particle rotation matrix.
            // We first apply the 180-degree correction (rotation_y_90), then the yaw (rotation_matrix_y),
            // and finally the pitch (rotation_matrix_x). This ensures the particle is rotated correctly to always face the camera.
            let particles_rotation_animation_matrix = rotation_y_180 * rotation_matrix_y * rotation_matrix_x;
            // Compute rotation for particles to always face the camera (STOP) --------------------------------------------------

            // Compute linear motion
            // Introducing a small variable semi random motion
            let particles_linear_x: f32 = 0.0123 * (elapsed * 0.13).sin(); // Semi random motion in x-axis
            let particles_linear_y: f32 = 0.0456 * (elapsed * 0.07).sin(); // Semi random motion in y-axis
            let particles_linear_z: f32 = 0.0789 * (elapsed * 0.74).sin(); // Semi random motion in z-axis
            let particles_linear_animation_matrix = glm::translation(&glm::vec3(
                particles_linear_x,
                particles_linear_y,
                particles_linear_z
            ));

            // Combine matrices to form view projection of rotating orca matrix
            // 1. Apply start position of Orca
            // 2. Translate Orca to the origin
            // 3. Apply rotation animation
            // 4. Apply translation animation
            // 5. Translate Orca back to its original position
            // 6. Apply view-projection transformation
            let view_projection_matrix_particles: glm::Mat4 = 
                view_projection_matrix * 
                glm::translation(&particles_position) *
                particles_linear_animation_matrix *
                particles_rotation_animation_matrix *
                glm::translation(&-particles_position) *
                particles_start_transform_matrix;



            // * Render Objects
            unsafe {
                // Clear the color and depth buffers
                gl::ClearColor(0.035, 0.046, 0.078, 1.0); // night sky
                gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT); // Clear the screen

                // * Render Orca
                shader_orca.activate();
                shader_orca.set_uniform_mat4("viewProjectionMatrix", &view_projection_matrix_orca);
                shader_orca.set_uniform_mat4("changingColorMatrix", &changing_color_matrix_orca);

                gl::BindVertexArray(vao_id_orca);
                gl::DrawElements(
                    gl::TRIANGLES,
                    indices_orca.len() as i32,
                    gl::UNSIGNED_INT,
                    std::ptr::null()
                );

                // * Render Particles
                shader_particles.activate();
                shader_particles.set_uniform_mat4("viewProjectionMatrix", &view_projection_matrix_particles);
                shader_particles.set_uniform_mat4("changingColorMatrix", &changing_color_matrix_particles);

                gl::BindVertexArray(vao_id_particles);
                gl::DrawElements(
                    gl::TRIANGLES,
                    indices_particles.len() as i32,
                    gl::UNSIGNED_INT,
                    std::ptr::null()
                );                
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
            // Handle mouse button events (right click for rotation)
            Event::WindowEvent { event: WindowEvent::MouseInput { button, state, .. }, .. } => {
                if button == glutin::event::MouseButton::Right {
                    if state == Pressed {
                        mouse_right_button_pressed = true;
                    } else {
                        mouse_right_button_pressed = false;
                    }
                }
            }
            // Handle mouse movement events
            Event::DeviceEvent { event: DeviceEvent::MouseMotion { delta }, .. } => {
                // Accumulate mouse movement
                // if let Ok(mut position) = arc_mouse_delta.lock() {
                //     *position = (position.0 + delta.0 as f32, position.1 + delta.1 as f32);
                // }

                // Only accumulate movement when right mouse button is pressed
                if mouse_right_button_pressed {  
                    if let Ok(mut mouse_delta) = arc_mouse_delta.lock() {
                        // Accumulate mouse movement for pitch and yaw
                        mouse_delta.0 += delta.0 as f32;
                        mouse_delta.1 += delta.1 as f32;
                    }
                }
            }
            _ => { }
        }
    });
}
