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
mod mesh;
mod scene_graph;
mod toolbox;

use glutin::event::{Event, WindowEvent, DeviceEvent, KeyboardInput, ElementState::{Pressed, Released}, VirtualKeyCode::{self, *}};
use glutin::event_loop::ControlFlow;
use scene_graph::SceneNode;


// initial window size
const INITIAL_SCREEN_W: u32 = 800;
const INITIAL_SCREEN_H: u32 = 600;

// Draw Scene
unsafe fn draw_scene(
    node: &scene_graph::SceneNode,
    view_projection_matrix: &glm::Mat4,
    transformation_so_far: &glm::Mat4,
    shader: &shader::Shader,
) {
    // Translation matrix to move the object to the reference point (move to origin of rotation)
    let translate_to_reference = glm::translation(&(-node.reference_point));
    
    // Regular translation for the node's position
    let translation_matrix = glm::translation(&node.position);

    // Scaling matrix
    let scale_matrix = glm::scaling(&node.scale);

    let rotation_matrix_z = glm::rotation(node.rotation.z, &glm::vec3(0.0, 0.0, 1.0)); // yaw
    let rotation_matrix_y = glm::rotation(node.rotation.y, &glm::vec3(0.0, 1.0, 0.0)); // pitch
    let rotation_matrix_x = glm::rotation(node.rotation.x, &glm::vec3(1.0, 0.0, 0.0)); // roll

    // Intrinsic rotation order: Z (yaw), Y (pitch), X (roll)
    let rotation_matrix = rotation_matrix_x * rotation_matrix_y * rotation_matrix_z;

    // Calculate the final transformation matrix:
    let transformation_matrix = transformation_so_far
        * translation_matrix            // Translate to the node's position
        * glm::translation(&node.reference_point)  // Translate back after rotation
        * rotation_matrix                // Apply rotation at the reference point
        * translate_to_reference         // Move to reference point 
        * scale_matrix;                  // Apply scaling               // Apply scaling

    let mvp_matrix = view_projection_matrix * transformation_matrix;

    let model_matrix = transformation_matrix;

    // If the node has a VAO, draw it
    if node.vao_id != 0 {
        shader.set_uniform_mat4("mvp_matrix", &mvp_matrix);
        shader.set_uniform_mat4("model_matrix", &model_matrix);

        
        // Draw the VAO
        gl::BindVertexArray(node.vao_id);
        gl::DrawElements(gl::TRIANGLES, node.index_count, gl::UNSIGNED_INT, std::ptr::null());
    }

    // Recursively draw the children
    for &child_ptr in &node.children {
        if let Some(child) = child_ptr.as_ref() {
            draw_scene(child, view_projection_matrix, &transformation_matrix, shader);
        }
    }
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
    
    let mut camera_position = glm::vec3(0.0, 0.0, 0.0);
    let camera_speed = 200.0;
    
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

        // * Load, Compile and Link the shader pair
        let shader = unsafe {
            shader::ShaderBuilder::new()
                .attach_file("shaders/simple.vert")
                .attach_file("shaders/simple.frag")
                .link()
        };

        let lunar_surface = mesh::Terrain::load("resources/lunarsurface.obj");

        let helicopter = mesh::Helicopter::load("resources/helicopter.obj");

        let (vao_id_terrain, vbo_id_terrain): (u32, u32) = unsafe { 
            util::create_vao(&lunar_surface.vertices, &lunar_surface.indices, &lunar_surface.colors, &lunar_surface.normals)          
        };

        let (vao_id_helicopter_body, vbo_id_helicopter_body): (u32, u32) = unsafe { 
            util::create_vao(&helicopter.body.vertices, &helicopter.body.indices, &helicopter.body.colors, &helicopter.body.normals)          
        };

        let (vao_id_helicopter_door, vbo_id_helicopter_door): (u32, u32) = unsafe { 
            util::create_vao(&helicopter.door.vertices, &helicopter.door.indices, &helicopter.door.colors, &helicopter.door.normals)          
        };

        let (vao_id_helicopter_main_rotor, vbo_id_helicopter_main_rotor): (u32, u32) = unsafe { 
            util::create_vao(&helicopter.main_rotor.vertices, &helicopter.main_rotor.indices, &helicopter.main_rotor.colors, &helicopter.main_rotor.normals)          
        };

        let (vao_id_helicopter_tail_rotor, vbo_id_helicopter_tail_rotor): (u32, u32) = unsafe { 
            util::create_vao(&helicopter.tail_rotor.vertices, &helicopter.tail_rotor.indices, &helicopter.tail_rotor.colors, &helicopter.tail_rotor.normals)          
        };

        // Create a vector to store the root nodes of the helicopters
        let mut helicopters: Vec<*mut SceneNode> = Vec::new();

        // * Set up the scene graph
        let mut scene_graph = SceneNode::new();
        let mut terrain_node = SceneNode::from_vao(vao_id_terrain, lunar_surface.index_count);

        scene_graph.add_child(&mut terrain_node);

        // Set up the root node for each helicopter
        for i in 0..5 {
            let mut helicopter_root_node = SceneNode::new();
            let mut helicopter_body_node = SceneNode::from_vao(vao_id_helicopter_body, helicopter.body.index_count);
            let mut helicopter_door_node = SceneNode::from_vao(vao_id_helicopter_door, helicopter.door.index_count);
            let mut helicopter_main_rotor_node = SceneNode::from_vao(vao_id_helicopter_main_rotor, helicopter.main_rotor.index_count);
            let mut helicopter_tail_rotor_node = SceneNode::from_vao(vao_id_helicopter_tail_rotor, helicopter.tail_rotor.index_count);

            // Set the reference point for the tail rotor
            helicopter_tail_rotor_node.reference_point = glm::vec3(0.35, 2.3, 10.4);

            // Build the scene graph for each helicopter
            helicopter_root_node.add_child(&mut helicopter_body_node);
            helicopter_root_node.add_child(&mut helicopter_door_node);
            helicopter_root_node.add_child(&mut helicopter_main_rotor_node);
            helicopter_root_node.add_child(&mut helicopter_tail_rotor_node);

            // Push each helicopter's root node into the vector (as raw pointers)
            unsafe {
                helicopters.push(helicopter_root_node.as_mut().get_unchecked_mut());
            }

            // Add the helicopter to the scene graph
            scene_graph.add_child(&mut helicopter_root_node);
        }
    
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

            // Update each helicopter's position and rotation
            for (i, helicopter_root_node) in helicopters.iter_mut().enumerate() {
                let heading_animation = toolbox::simple_heading_animation(elapsed + (i as f32) * 0.8); // Offset for each helicopter

                // Dereference the pointer to access the fields
                unsafe {
                    // Dereference twice to get access to the fields
                    (*(*helicopter_root_node)).position = glm::vec3(heading_animation.x, 0.0, heading_animation.z);
                    (*(*helicopter_root_node)).rotation.x = heading_animation.pitch;
                    (*(*helicopter_root_node)).rotation.y = heading_animation.yaw;
                    (*(*helicopter_root_node)).rotation.z = heading_animation.roll;

                    // Update the rotors
                    let helicopter_main_rotor_node = (*(*helicopter_root_node)).get_child(2); // Assuming rotor is the 3rd child
                    let helicopter_tail_rotor_node = (*(*helicopter_root_node)).get_child(3); // Assuming tail rotor is the 4th child

                    // Update the rotation of the helicopter's rotors based on the elapsed time
                    (*helicopter_main_rotor_node).rotation.y = elapsed * 5.0; // Main rotor spinning continuously
                    (*helicopter_tail_rotor_node).rotation.x = elapsed * 8.0; // Tail rotor spinning continuously
                }
            }


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
            let view_projection_matrix: glm::Mat4 = util::calculate_transformation_from_camera_to_world_view(
                window_aspect_ratio,
                camera_position,
                camera_forward,
                camera_up
            );

            // * Render Objects
            unsafe {
                // Clear the color and depth buffers
                gl::ClearColor(0.035, 0.046, 0.078, 1.0); // night sky
                gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT); // Clear the screen

                shader.activate();

                // Render the scene graph
                draw_scene(&*scene_graph, &view_projection_matrix, &glm::identity(), &shader);


              
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
