// main.rs
mod framebuffer;
mod triangle;
mod obj;
mod matrix;
mod fragment;
mod vertex;
mod camera;
mod shaders;
mod light;

use triangle::triangle;
use obj::Obj;
use framebuffer::Framebuffer;
use raylib::prelude::*;
use std::thread;
use std::time::Duration;
use std::f32::consts::PI;
use matrix::{create_model_matrix, create_projection_matrix, create_viewport_matrix};
use vertex::Vertex;
use camera::Camera;
use shaders::{vertex_shader, fragment_shader, render_rings, render_moon};
use light::Light;

#[derive(Clone)]
pub struct Uniforms {
    pub model_matrix: Matrix,
    pub view_matrix: Matrix,
    pub projection_matrix: Matrix,
    pub viewport_matrix: Matrix,
    pub time: f32,
    pub dt: f32,
    pub planet_type: i32,
    pub render_type: i32,
}

fn render_planet(framebuffer: &mut Framebuffer, uniforms: &Uniforms, vertex_array: &[Vertex], light: &Light) {
    let mut transformed_vertices = Vec::with_capacity(vertex_array.len());
    let mut planet_uniforms = uniforms.clone();
    planet_uniforms.render_type = 0;
    
    for vertex in vertex_array {
        let transformed = vertex_shader(vertex, &planet_uniforms);
        transformed_vertices.push(transformed);
    }

    let mut triangles = Vec::new();
    for i in (0..transformed_vertices.len()).step_by(3) {
        if i + 2 < transformed_vertices.len() {
            triangles.push([
                transformed_vertices[i].clone(),
                transformed_vertices[i + 1].clone(),
                transformed_vertices[i + 2].clone(),
            ]);
        }
    }

    let mut fragments = Vec::new();
    for tri in &triangles {
        fragments.extend(triangle(&tri[0], &tri[1], &tri[2], light));
    }

    for fragment in fragments {      
        let final_color = fragment_shader(&fragment, uniforms);
        framebuffer.point(
            fragment.position.x as i32,
            fragment.position.y as i32,
            final_color,
            fragment.depth,
        );
    }
}

fn main() {
    let window_width = 1300;
    let window_height = 900;

    let (mut window, raylib_thread) = raylib::init()
        .size(window_width, window_height)
        .title("Planetas Procedurales - Laboratorio")
        .log_level(TraceLogLevel::LOG_WARNING)
        .build();

    let mut framebuffer = Framebuffer::new(window_width, window_height);
    let mut camera = Camera::new(
        Vector3::new(0.0, 0.0, 8.0),
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
    );

    let translation = Vector3::new(0.0, 0.0, 0.0);
    let scale = 1.0;
    let rotation = Vector3::new(0.0, 0.0, 0.0);
    let light = Light::new(Vector3::new(5.0, 5.0, 5.0));

    let obj = Obj::load("./models/sphere.obj").expect("Failed to load sphere.obj");
    let vertex_array = obj.get_vertex_array();

    framebuffer.set_background_color(Color::new(30, 30, 30, 255));

    let mut time = 0.0;
    let mut planet_type = 0;

    while !window.window_should_close() {
        let dt = window.get_frame_time();
        time += dt;
        
        if window.is_key_pressed(KeyboardKey::KEY_ONE) { planet_type = 0; }
        if window.is_key_pressed(KeyboardKey::KEY_TWO) { planet_type = 1; }
        if window.is_key_pressed(KeyboardKey::KEY_THREE) { planet_type = 2; }
        if window.is_key_pressed(KeyboardKey::KEY_FOUR) { planet_type = 3; }
        if window.is_key_pressed(KeyboardKey::KEY_FIVE) { planet_type = 4; }
        
        camera.process_input(&window);
        framebuffer.clear();

        let model_matrix = create_model_matrix(translation, scale, rotation);
        let view_matrix = camera.get_view_matrix();
        let projection_matrix = create_projection_matrix(PI / 3.0, window_width as f32 / window_height as f32, 0.1, 100.0);
        let viewport_matrix = create_viewport_matrix(0.0, 0.0, window_width as f32, window_height as f32);

        let planet_uniforms = Uniforms {
            model_matrix,
            view_matrix,
            projection_matrix,
            viewport_matrix,
            time,
            dt,
            planet_type,
            render_type: 0,
        };

        render_planet(&mut framebuffer, &planet_uniforms, &vertex_array, &light);

        // Anillos SOLO para planeta 3
        if planet_type == 3 {
            render_rings(&mut framebuffer, &planet_uniforms, &vertex_array, &light);
        }

        // Luna SOLO para planeta 0 (rocoso)
        if planet_type == 0 {
            render_moon(&mut framebuffer, &planet_uniforms, &vertex_array, &light);
        }

        framebuffer.swap_buffers(&mut window, &raylib_thread);
        thread::sleep(Duration::from_millis(16));
    }
}