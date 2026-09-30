//! Proyecto 2: Raytracing en CPU — ventana interactiva.
//!
//! raylib solo se usa para: abrir la ventana, subir nuestro framebuffer a una
//! textura y leer teclado/mouse. Cada píxel lo calcula nuestro raytracer.
//!
//! Controles:
//!   Mouse izq. + arrastrar / A D / ← →   rotar alrededor del diorama
//!   W S / ↑ ↓                            inclinar la cámara
//!   Rueda del mouse / Q E                acercar / alejar (zoom)
//!   Espacio                              rotación automática on/off
//!   P                                    guardar captura (BMP)

// Algunas funciones se usan hasta los pasos 6 y 7
#![allow(dead_code)]

mod block;
mod camera;
mod image;
mod material;
mod math;
mod perlin;
mod ray;
mod render;
mod scene;
mod texture;
mod voxel;
mod world;

use raylib::prelude::*;
use render::Framebuffer;
use std::path::Path;
use std::time::Instant;

const WIN_W: i32 = 1280;
const WIN_H: i32 = 720;
/// Mientras la cámara se mueve se renderiza a 1/PREVIEW_SCALE de resolución
/// para que se sienta fluido; al soltar, se renderiza en resolución completa.
const PREVIEW_SCALE: usize = 3;
/// Segundos sin mover la cámara antes de hacer el render completo
const IDLE_BEFORE_FULL: f32 = 0.15;

fn main() {
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());

    let t = Instant::now();
    let scene = scene::Scene::new();
    println!(
        "Escena lista en {:.0} ms: {} bloques, {} hilos",
        t.elapsed().as_secs_f32() * 1000.0,
        scene.world.solid_count(),
        threads
    );
    println!("Materiales: {}", scene.materials.names().join(", "));
    for r in &scene.world.regions {
        println!("Región (AABB broadphase): {}", r.name);
    }

    let (mut rl, thread) = raylib::init()
        .size(WIN_W, WIN_H)
        .title("Proyecto 2 - Raytracing (CPU)")
        .build();
    rl.set_target_fps(60);

    let mut orbit = scene.default_orbit();
    let aspect = WIN_W as f32 / WIN_H as f32;

    // Dos framebuffers: preview (baja resolución) y completo
    let mut fb_full = Framebuffer::new(WIN_W as usize, WIN_H as usize);
    let mut fb_prev = Framebuffer::new(WIN_W as usize / PREVIEW_SCALE, WIN_H as usize / PREVIEW_SCALE);
    let mut tex_full = make_texture(&mut rl, &thread, &fb_full);
    let mut tex_prev = make_texture(&mut rl, &thread, &fb_prev);
    tex_prev.set_texture_filter(&thread, TextureFilter::TEXTURE_FILTER_POINT);

    let mut auto_rotate = false;
    let mut idle = 0.0f32;
    let mut full_ready = false;
    let mut showing_full = false;
    let mut last_render_ms = 0.0f32;
    let mut first_frame = true;

    while !rl.window_should_close() {
        let dt = rl.get_frame_time();
        let before = orbit;

        // ------------------------- entrada -------------------------
        if rl.is_key_pressed(KeyboardKey::KEY_SPACE) {
            auto_rotate = !auto_rotate;
        }
        let rot_speed = 1.6 * dt;
        if rl.is_key_down(KeyboardKey::KEY_A) || rl.is_key_down(KeyboardKey::KEY_LEFT) {
            orbit.rotate(-rot_speed, 0.0);
        }
        if rl.is_key_down(KeyboardKey::KEY_D) || rl.is_key_down(KeyboardKey::KEY_RIGHT) {
            orbit.rotate(rot_speed, 0.0);
        }
        if rl.is_key_down(KeyboardKey::KEY_W) || rl.is_key_down(KeyboardKey::KEY_UP) {
            orbit.rotate(0.0, rot_speed * 0.6);
        }
        if rl.is_key_down(KeyboardKey::KEY_S) || rl.is_key_down(KeyboardKey::KEY_DOWN) {
            orbit.rotate(0.0, -rot_speed * 0.6);
        }
        if rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT) {
            let d = rl.get_mouse_delta();
            orbit.rotate(-d.x * 0.006, d.y * 0.006);
        }
        let wheel = rl.get_mouse_wheel_move();
        if wheel != 0.0 {
            orbit.zoom(1.0 - wheel * 0.1);
        }
        if rl.is_key_down(KeyboardKey::KEY_Q) {
            orbit.zoom(1.0 - 1.2 * dt);
        }
        if rl.is_key_down(KeyboardKey::KEY_E) {
            orbit.zoom(1.0 + 1.2 * dt);
        }
        if auto_rotate {
            orbit.rotate(0.5 * dt, 0.0);
        }

        // ------------------------- render --------------------------
        let moved = orbit != before || first_frame;
        first_frame = false;
        if moved {
            // cámara en movimiento: preview rápido en baja resolución
            idle = 0.0;
            full_ready = false;
            let t = Instant::now();
            render::render(&scene, &orbit.camera(aspect), &mut fb_prev, threads, 1);
            last_render_ms = t.elapsed().as_secs_f32() * 1000.0;
            let _ = tex_prev.update_texture(&fb_prev.pixels);
            showing_full = false;
        } else {
            idle += dt;
            if !full_ready && idle >= IDLE_BEFORE_FULL {
                // cámara quieta: un solo render en resolución completa
                let t = Instant::now();
                render::render(&scene, &orbit.camera(aspect), &mut fb_full, threads, 1);
                last_render_ms = t.elapsed().as_secs_f32() * 1000.0;
                let _ = tex_full.update_texture(&fb_full.pixels);
                full_ready = true;
                showing_full = true;
            }
        }

        if rl.is_key_pressed(KeyboardKey::KEY_P) {
            if !full_ready {
                render::render(&scene, &orbit.camera(aspect), &mut fb_full, threads, 1);
            }
            let name = format!("captura_{}.bmp", chrono_stamp());
            match image::save(Path::new(&name), fb_full.width, fb_full.height, &fb_full.to_rgb()) {
                Ok(()) => println!("Captura guardada: {name}"),
                Err(e) => eprintln!("No se pudo guardar la captura: {e}"),
            }
        }

        // ------------------------- dibujo --------------------------
        let fps = rl.get_fps();
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::BLACK);
        let tex = if showing_full { &tex_full } else { &tex_prev };
        let src = Rectangle::new(0.0, 0.0, tex.width() as f32, tex.height() as f32);
        let dst = Rectangle::new(0.0, 0.0, WIN_W as f32, WIN_H as f32);
        d.draw_texture_pro(tex, src, dst, Vector2::zero(), 0.0, Color::WHITE);

        // HUD
        d.draw_rectangle(8, 8, 330, 118, Color::new(0, 0, 0, 140));
        d.draw_text(
            &format!("FPS {fps}  |  render {last_render_ms:.1} ms  |  {threads} hilos"),
            16,
            16,
            16,
            Color::WHITE,
        );
        d.draw_text(
            if showing_full { "Resolucion completa" } else { "Preview (moviendo)" },
            16,
            38,
            16,
            Color::LIGHTGRAY,
        );
        d.draw_text("Arrastrar / A D: rotar   W S: inclinar", 16, 62, 14, Color::LIGHTGRAY);
        d.draw_text("Rueda / Q E: zoom   Espacio: auto-rotar", 16, 80, 14, Color::LIGHTGRAY);
        d.draw_text("P: guardar captura BMP", 16, 98, 14, Color::LIGHTGRAY);
    }
}

fn make_texture(rl: &mut RaylibHandle, thread: &RaylibThread, fb: &Framebuffer) -> Texture2D {
    let img = Image::gen_image_color(fb.width as i32, fb.height as i32, Color::BLACK);
    rl.load_texture_from_image(thread, &img)
        .expect("no se pudo crear la textura")
}

/// Marca de tiempo simple (segundos desde 1970) para nombrar capturas
fn chrono_stamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}