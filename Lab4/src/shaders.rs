// shaders.rs
use raylib::prelude::*;
use crate::vertex::Vertex;
use crate::Uniforms;
use crate::matrix::multiply_matrix_vector4;
use crate::fragment::Fragment;
use crate::framebuffer::Framebuffer;
use crate::triangle;
use crate::light::Light;

// Trait para interpolación lineal
pub trait Lerp {
    fn lerp(self, other: Self, t: f32) -> Self;
}

impl Lerp for Vector3 {
    fn lerp(self, other: Self, t: f32) -> Self {
        let t = t.max(0.0).min(1.0);
        Vector3::new(
            self.x + (other.x - self.x) * t,
            self.y + (other.y - self.y) * t,
            self.z + (other.z - self.z) * t,
        )
    }
}

pub fn vertex_shader(vertex: &Vertex, uniforms: &Uniforms) -> Vertex {
    let mut position_vec4 = Vector4::new(
        vertex.position.x,
        vertex.position.y,
        vertex.position.z,
        1.0
    );

    match uniforms.render_type {
        1 => { // rings
            let angle = (vertex.position.x.atan2(vertex.position.y) + uniforms.time * 0.2) % (2.0 * std::f32::consts::PI);
            let base_radius = 1.8 + (vertex.position.z * 0.3).sin() * 0.2;
            position_vec4.x = base_radius * angle.cos();
            position_vec4.z = base_radius * angle.sin();
            position_vec4.y = vertex.position.y * 0.05; // muy delgado
        }
        2 => { // moon
            let moon_orbit_time = uniforms.time * 0.4;
            let moon_distance = 2.8;
            let moon_x = moon_distance * moon_orbit_time.cos();
            let moon_z = moon_distance * moon_orbit_time.sin();
            let moon_y = (moon_orbit_time * 3.0).sin() * 0.2;
            let moon_base = Vector3::new(moon_x, moon_y, moon_z);
            position_vec4.x = moon_base.x + vertex.position.x * 0.25;
            position_vec4.y = moon_base.y + vertex.position.y * 0.25;
            position_vec4.z = moon_base.z + vertex.position.z * 0.25;
        }
        _ => {}
    }

    let world_position = multiply_matrix_vector4(&uniforms.model_matrix, &position_vec4);
    let view_position = multiply_matrix_vector4(&uniforms.view_matrix, &world_position);
    let clip_position = multiply_matrix_vector4(&uniforms.projection_matrix, &view_position);

    let ndc = if clip_position.w != 0.0 {
        Vector3::new(
            clip_position.x / clip_position.w,
            clip_position.y / clip_position.w,
            clip_position.z / clip_position.w,
        )
    } else {
        Vector3::new(clip_position.x, clip_position.y, clip_position.z)
    };

    let ndc_vec4 = Vector4::new(ndc.x, ndc.y, ndc.z, 1.0);
    let screen_position = multiply_matrix_vector4(&uniforms.viewport_matrix, &ndc_vec4);
    
    let transformed_position = Vector3::new(screen_position.x, screen_position.y, screen_position.z);
    
    Vertex {
        position: vertex.position,
        normal: vertex.normal,
        tex_coords: vertex.tex_coords,
        color: vertex.color,
        transformed_position,
        transformed_normal: transform_normal(&vertex.normal, &uniforms.model_matrix),
    }
}

fn transform_normal(normal: &Vector3, model_matrix: &Matrix) -> Vector3 {
    let normal_vec4 = Vector4::new(normal.x, normal.y, normal.z, 0.0);
    let transformed = multiply_matrix_vector4(model_matrix, &normal_vec4);
    let mut n = Vector3::new(transformed.x, transformed.y, transformed.z);
    let len = (n.x * n.x + n.y * n.y + n.z * n.z).sqrt();
    if len > 0.0 { n.x /= len; n.y /= len; n.z /= len; }
    n
}

fn hash31(n: f32) -> f32 {
    let n = (n * 1234567.0).sin() * 43758.5453;
    n - n.floor()
}

fn noise(pos: &Vector3) -> f32 {
    let ix = pos.x.floor() as i32;
    let iy = pos.y.floor() as i32;
    let iz = pos.z.floor() as i32;

    let fx = pos.x - pos.x.floor();
    let fy = pos.y - pos.y.floor();
    let fz = pos.z - pos.z.floor();

    // Smoothstep interpolation
    let u = fx * fx * (3.0 - 2.0 * fx);
    let v = fy * fy * (3.0 - 2.0 * fy);
    let w = fz * fz * (3.0 - 2.0 * fz);

    let n = |i: i32, j: i32, k: i32| -> f32 {
        hash31((ix + i) as f32 + (iy + j) as f32 * 57.0 + (iz + k) as f32 * 113.0)
    };

    // Interpolación lineal manual para f32: a + (b - a) * t
    let x1 = n(0, 0, 0) + (n(1, 0, 0) - n(0, 0, 0)) * u;
    let x2 = n(0, 1, 0) + (n(1, 1, 0) - n(0, 1, 0)) * u;
    let x3 = n(0, 0, 1) + (n(1, 0, 1) - n(0, 0, 1)) * u;
    let x4 = n(0, 1, 1) + (n(1, 1, 1) - n(0, 1, 1)) * u;

    let y1 = x1 + (x2 - x1) * v;
    let y2 = x3 + (x4 - x3) * v;

    y1 + (y2 - y1) * w
}

fn fractal_noise(pos: &Vector3, octaves: i32) -> f32 {
    let mut value = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = 1.0;
    let mut weight = 1.0;
    for _ in 0..octaves {
        value += noise(&Vector3::new(pos.x * frequency, pos.y * frequency, pos.z * frequency)) * amplitude * weight;
        amplitude *= 0.5;
        frequency *= 2.0;
        weight *= 0.7;
    }
    value
}

fn simulate_lighting(normal: &Vector3, light_dir: &Vector3) -> f32 {
    let dot = normal.x * light_dir.x + normal.y * light_dir.y + normal.z * light_dir.z;
    dot.max(0.1).min(1.0) // mínimo ambiente
}

fn rotate_planet_position(pos: &Vector3, time: f32, speed: f32) -> Vector3 {
    let angle = time * speed;
    let cos_a = angle.cos();
    let sin_a = angle.sin();
    Vector3::new(
        pos.x * cos_a - pos.z * sin_a,
        pos.y,
        pos.x * sin_a + pos.z * cos_a
    )
}

// 0: Rocky (Mars-like)
fn rocky_planet_color(pos: &Vector3, time: f32) -> Vector3 {
    let rotated = rotate_planet_position(pos, time, 0.25);
    let base_noise = fractal_noise(&rotated, 4);
    let detail = fractal_noise(&Vector3::new(rotated.x * 8.0, rotated.y * 8.0, rotated.z * 8.0), 2);
    let elevation = (base_noise + detail * 0.3) * 0.5 + 0.5;

    let low = Vector3::new(0.55, 0.25, 0.15);
    let high = Vector3::new(0.75, 0.45, 0.25);
    let crater = Vector3::new(0.2, 0.15, 0.1);

    let mut color = if elevation < 0.3 {
        low
    } else if elevation > 0.7 {
        high
    } else {
        low.lerp(high, (elevation - 0.3) / 0.4)
    };

    // Cráteres
    let centers = [
        Vector3::new(0.6, 0.2, 0.1),
        Vector3::new(-0.5, -0.3, 0.2),
        Vector3::new(0.1, 0.8, -0.2),
    ];
    for c in &centers {
        let d = ((rotated.x - c.x).powi(2) + (rotated.y - c.y).powi(2) + (rotated.z - c.z).powi(2)).sqrt();
        if d < 0.18 {
            let blend = (1.0 - (d / 0.18).min(1.0)).powi(2);
            color = color.lerp(crater, blend * 0.8);
        }
    }

    let light_dir = Vector3::new(1.0, 1.0, 1.0);
    let lighting = simulate_lighting(&Vector3::new(rotated.x, rotated.y, rotated.z), &light_dir);
    color * lighting
}

// 1: Gaseous (Jupiter-like)
fn gaseous_planet_color(pos: &Vector3, time: f32) -> Vector3 {
    let rotated = rotate_planet_position(pos, time, 1.3);
    let r = (rotated.x.powi(2) + rotated.y.powi(2) + rotated.z.powi(2)).sqrt().max(0.001);
    let lat = (rotated.z / r).asin();

    let band1 = (lat * 9.0 + time * 0.25).sin().abs();
    let band2 = (lat * 14.0 + time * 0.35 + 0.7).cos().abs();

    let mut color = Vector3::new(0.92, 0.82, 0.65);

    if band1 > 0.75 {
        color = color.lerp(Vector3::new(0.55, 0.35, 0.2), 0.5);
    }
    if band2 > 0.8 {
        color = color.lerp(Vector3::new(0.4, 0.3, 0.6), 0.4);
    }

    // Tormenta animada
    let storm_phase = time * 0.1;
    let storm_x = rotated.x + 0.35 + storm_phase.sin() * 0.05;
    let storm_y = rotated.y - 0.2 + storm_phase.cos() * 0.03;
    let storm_d = (storm_x * storm_x + storm_y * storm_y).sqrt();
    if storm_d < 0.22 {
        let blend = (1.0 - storm_d / 0.22).powi(2);
        color = color.lerp(Vector3::new(0.88, 0.25, 0.18), blend * 0.7);
    }

    // Nubes
    let cloud = fractal_noise(&Vector3::new(rotated.x * 25.0, rotated.y * 25.0, time * 0.12), 4);
    color = color + Vector3::new(1.0, 1.0, 1.0) * (cloud * 0.3).max(0.0);

    let light_dir = Vector3::new(1.0, 1.0, 1.0);
    let lighting = simulate_lighting(&Vector3::new(rotated.x, rotated.y, rotated.z), &light_dir);
    color * lighting.clamp(0.3, 1.0)
}

// 2: Sci-fi Bioluminescent Planet
fn biolum_planet_color(pos: &Vector3, time: f32) -> Vector3 {
    let rotated = rotate_planet_position(pos, time, 0.6);
    let r = (rotated.x.powi(2) + rotated.y.powi(2) + rotated.z.powi(2)).sqrt().max(0.001);
    let lat = (rotated.z / r).asin();
    let _lon = rotated.y.atan2(rotated.x); // no se usa, pero lo dejamos comentado

    let terrain = fractal_noise(&rotated, 4);
    let elevation = terrain * 0.5 + 0.5;

    let ocean = Vector3::new(0.02, 0.05, 0.15);
    let land = Vector3::new(0.1, 0.3, 0.1);
    let glow_plants = Vector3::new(0.2, 0.8, 0.4); // verde brillante

    let mut color = if elevation < 0.4 {
        ocean
    } else {
        land
    };

    // Flora bioluminiscente en zonas altas
    let glow_noise = fractal_noise(&Vector3::new(rotated.x * 6.0, rotated.y * 6.0, rotated.z * 6.0), 3);
    let is_glowing = glow_noise > 0.6 && elevation > 0.5;
    if is_glowing {
        color = color.lerp(glow_plants, 0.7);
    }

    // Polos helados
    if lat.abs() > 1.1 {
        color = Vector3::new(0.85, 0.9, 1.0);
    }

    // Iluminación suave + emisión nocturna
    let light_dir = Vector3::new(1.0, 1.0, 1.0);
    let dot = rotated.x * light_dir.x + rotated.y * light_dir.y + rotated.z * light_dir.z;
    let is_day = dot > 0.0;
    let lighting = if is_day {
        dot.max(0.2)
    } else {
        0.1 // noche
    };

    let mut final_color = color * lighting;
    if !is_day && is_glowing {
        final_color = final_color + glow_plants * 0.3; // brilla en la noche
    }

    final_color
}

// 3: Ringed Planet (Saturn-like)
fn ringed_planet_color(pos: &Vector3, time: f32) -> Vector3 {
    let rotated = rotate_planet_position(pos, time, 0.5);
    let r = (rotated.x.powi(2) + rotated.y.powi(2) + rotated.z.powi(2)).sqrt().max(0.001);
    let lat = (rotated.z / r).asin();

    let base = Vector3::new(0.75, 0.65, 0.5);
    let bands = (lat * 7.0 + time * 0.08).sin().abs();
    let color = base.lerp(Vector3::new(0.85, 0.75, 0.4), bands * 0.35);

    let light_dir = Vector3::new(1.0, 1.0, 1.0);
    let lighting = simulate_lighting(&Vector3::new(rotated.x, rotated.y, rotated.z), &light_dir);
    color * lighting
}

// 4: Ice Crystal Planet
fn ice_planet_color(pos: &Vector3, time: f32) -> Vector3 {
    let rotated = rotate_planet_position(pos, time, 0.3);
    let noise_val = fractal_noise(&rotated, 5);
    let fractures = fractal_noise(&Vector3::new(rotated.x * 10.0, rotated.y * 10.0, rotated.z * 10.0 + time), 3);

    let base_ice = Vector3::new(0.85, 0.95, 1.0);
    let deep_ice = Vector3::new(0.6, 0.8, 0.95);
    let crystal_core = Vector3::new(0.9, 0.98, 1.0);

    let mut color = if noise_val < 0.3 {
        deep_ice
    } else if fractures > 0.7 {
        crystal_core
    } else {
        base_ice
    };

    // Efecto de refracción simulado
    let light_dir = Vector3::new(1.0, 1.0, 1.0);
    let dot = rotated.x * light_dir.x + rotated.y * light_dir.y + rotated.z * light_dir.z;
    let fresnel = (1.0 - dot.abs()).powi(3);
    color = color.lerp(Vector3::new(1.0, 1.0, 1.0), fresnel * 0.3);

    color * dot.max(0.2)
}

// Render rings with procedural texture
pub fn render_rings(framebuffer: &mut Framebuffer, uniforms: &Uniforms, vertex_array: &[Vertex], light: &Light) {
    let mut transformed_vertices = Vec::with_capacity(vertex_array.len());
    let mut ring_uniforms = uniforms.clone();
    ring_uniforms.render_type = 1;

    for vertex in vertex_array {
        let transformed = vertex_shader(vertex, &ring_uniforms);
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
        fragments.extend(triangle::triangle(&tri[0], &tri[1], &tri[2], light));
    }

    for fragment in fragments {
        // Aproximación de posición en mundo para los anillos
        let dx = fragment.world_position.x;
        let dz = fragment.world_position.z;
        let radius = (dx * dx + dz * dz).sqrt();

        // Solo renderizar entre ciertos radios
        if radius < 1.6 || radius > 2.4 {
            continue;
        }

        let pattern = (radius * 40.0 + uniforms.time * 0.15).sin().abs();
        let base = Vector3::new(0.88, 0.82, 0.65);
        let dark = Vector3::new(0.65, 0.58, 0.4);
        let ring_color = base.lerp(dark, pattern * 0.5);

        let ring_normal = Vector3::new(0.0, 1.0, 0.0);
        let light_dir = Vector3::new(1.0, 1.0, 1.0);
        let lighting = simulate_lighting(&ring_normal, &light_dir);

        let final_color = ring_color * lighting;
        framebuffer.point(
            fragment.position.x as i32,
            fragment.position.y as i32,
            final_color,
            fragment.depth,
        );
    }
}

// Render moon only for rocky planet
pub fn render_moon(framebuffer: &mut Framebuffer, uniforms: &Uniforms, vertex_array: &[Vertex], light: &Light) {
    let mut transformed_vertices = Vec::with_capacity(vertex_array.len());
    let mut moon_uniforms = uniforms.clone();
    moon_uniforms.render_type = 2;

    for vertex in vertex_array {
        let transformed = vertex_shader(vertex, &moon_uniforms);
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
        fragments.extend(triangle::triangle(&tri[0], &tri[1], &tri[2], light));
    }

    for fragment in fragments {
        let moon_base = Vector3::new(0.65, 0.62, 0.6);
        let crater_noise = fractal_noise(&Vector3::new(
            fragment.world_position.x * 8.0,
            fragment.world_position.y * 8.0,
            fragment.world_position.z * 8.0
        ), 2);
        let moon_color = if crater_noise > 0.6 {
            Vector3::new(0.5, 0.48, 0.45)
        } else {
            moon_base
        };

        let moon_normal = Vector3::new(fragment.world_position.x, fragment.world_position.y, fragment.world_position.z);
        let light_dir = Vector3::new(1.0, 1.0, 1.0);
        let lighting = simulate_lighting(&moon_normal, &light_dir);
        let final_color = moon_color * lighting;

        framebuffer.point(
            fragment.position.x as i32,
            fragment.position.y as i32,
            final_color,
            fragment.depth,
        );
    }
}

pub fn fragment_shader(fragment: &Fragment, uniforms: &Uniforms) -> Vector3 {
    let pos = fragment.world_position;
    let time = uniforms.time;
    let planet_type = uniforms.planet_type;
    
    let color = match planet_type {
        0 => rocky_planet_color(&pos, time),
        1 => gaseous_planet_color(&pos, time),
        2 => biolum_planet_color(&pos, time), // ¡Planeta de ciencia ficción!
        3 => ringed_planet_color(&pos, time),
        4 => ice_planet_color(&pos, time),
        _ => Vector3::new(0.5, 0.5, 0.5),
    };
    
    Vector3::new(
        color.x.max(0.0).min(1.0),
        color.y.max(0.0).min(1.0),
        color.z.max(0.0).min(1.0),
    )
}