#![recursion_limit = "1024"]

mod app;
mod gltf;
mod gpu;
mod io;
mod render;
mod view;
mod world;

#[cfg(target_arch = "wasm32")]
use console_error_panic_hook::set_once as set_panic_hook;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

pub fn start_app() {
    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

    #[allow(unused_mut)]
    let mut builder = winit::window::WindowBuilder::new();

    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::WindowBuilderExtWebSys;
        let canvas = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .get_element_by_id("canvas")
            .unwrap()
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .unwrap();
        builder = builder.with_canvas(Some(canvas));
    }

    let window = builder.build(&event_loop).unwrap();

    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
        pollster::block_on(app::run_app(Viewer::default(), event_loop, window));
    }
    #[cfg(target_arch = "wasm32")]
    {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init().expect("could not initialize logger");
        wasm_bindgen_futures::spawn_local(app::run_app(Viewer::default(), event_loop, window));
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(inline_js = "export function snippetTest() { console.log('Hello from JS FFI!'); }")]
extern "C" {
    fn snippetTest();
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub async fn run() {
    set_panic_hook();
    snippetTest();
    start_app();
}

#[derive(Default)]
pub struct Viewer;

impl app::State for Viewer {
    fn initialize(&mut self, context: &mut app::Context) {
        context.import_slice(include_bytes!("../glb/DamagedHelmet.glb"));
    }

    fn update(&mut self, context: &mut app::Context) {
        camera_system(context);
    }
}

fn camera_system(context: &mut app::Context) {
    let Some(scene_index) = context.world.default_scene_index else {
        return;
    };

    let scene = &context.world.scenes[scene_index];

    let camera_node_index = scene.graph[scene
        .default_camera_graph_node_index
        .expect("No camera is available in the active scene!")];
    let camera_node = &mut context.world.nodes[camera_node_index];

    let metadata = &context.world.metadata[camera_node.metadata_index];
    if metadata.name != "Main Camera" {
        return;
    }

    let transform = &mut context.world.transforms[camera_node.transform_index];
    let camera = &mut context.world.cameras[camera_node.camera_index.unwrap()];

    let mut sync_transform = false;

    let delta_time = 0.01; // simulate delta time for wasm
    let speed = 2.0 * delta_time as f32;

    if context.io.is_key_pressed(winit::keyboard::KeyCode::KeyW) {
        camera.orientation.offset -= camera.orientation.direction() * speed;
        sync_transform = true;
    }

    if context.io.is_key_pressed(winit::keyboard::KeyCode::KeyA) {
        camera.orientation.offset += camera.orientation.right() * speed;
        sync_transform = true;
    }

    if context.io.is_key_pressed(winit::keyboard::KeyCode::KeyS) {
        camera.orientation.offset += camera.orientation.direction() * speed;
        sync_transform = true;
    }

    if context.io.is_key_pressed(winit::keyboard::KeyCode::KeyD) {
        camera.orientation.offset -= camera.orientation.right() * speed;
        sync_transform = true;
    }

    if context.io.is_key_pressed(winit::keyboard::KeyCode::Space) {
        camera.orientation.offset += camera.orientation.up() * speed;
        sync_transform = true;
    }

    if context
        .io
        .is_key_pressed(winit::keyboard::KeyCode::ShiftLeft)
    {
        camera.orientation.offset -= camera.orientation.up() * speed;
        sync_transform = true;
    }

    camera
        .orientation
        .zoom(6.0 * context.io.mouse.wheel_delta.y * (delta_time as f32));

    if context.io.mouse.is_middle_clicked {
        camera
            .orientation
            .pan(&(context.io.mouse.position_delta * delta_time as f32));
        sync_transform = true;
    }

    if context.io.mouse.is_right_clicked
        && context.io.mouse.position_delta != nalgebra_glm::vec2(0.0, 0.0)
    {
        let mut delta = context.io.mouse.position_delta * delta_time as f32;
        delta.x *= -1.0;
        delta.y *= -1.0;
        camera.orientation.rotate(&delta);
        sync_transform = true;
    }

    if context.io.touch.moved {
        let delta = context.io.touch.touch_delta * delta_time as f32 * 0.02; // arbitrary scaledown touch input
        camera.orientation.rotate(&delta);
        sync_transform = true;
    }

    if sync_transform {
        transform.translation = camera.orientation.position();
        transform.rotation = camera.orientation.look_at_offset();
    }
}
