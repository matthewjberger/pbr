pub async fn run_app(
    mut state: impl State + 'static,
    event_loop: winit::event_loop::EventLoop<()>,
    window: winit::window::Window,
) {
    #[cfg(target_arch = "wasm32")]
    let mut file_receiver: Option<futures::channel::oneshot::Receiver<Vec<u8>>> = None;

    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    let mut renderer = crate::render::Renderer::new(window, 1280, 720).await;
    let mut context = Context {
        io: crate::io::Io::default(),
        world: crate::world::World::default(),
        should_exit: false,
        should_reload_view: false,
    };

    state.initialize(&mut context);

    event_loop
        .run(move |event, elwt| {
            if let winit::event::Event::NewEvents(..) = event {
                state.update(&mut context);
            }

            if let winit::event::Event::WindowEvent {
                event: winit::event::WindowEvent::CloseRequested,
                ..
            } = event
            {
                elwt.exit();
            }

            if let winit::event::Event::WindowEvent {
                event:
                    winit::event::WindowEvent::KeyboardInput {
                        event:
                            winit::event::KeyEvent {
                                physical_key: winit::keyboard::PhysicalKey::Code(key_code),
                                state,
                                ..
                            },
                        ..
                    },
                ..
            } = event
            {
                if matches!(
                    (key_code, state),
                    (
                        winit::keyboard::KeyCode::KeyF,
                        winit::event::ElementState::Pressed
                    )
                ) {
                    #[cfg(not(target_arch = "wasm32"))]
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("GLTF / GLB", &["gltf", "glb"])
                        .pick_file()
                    {
                        log::info!("File picked: {path:#?}");
                        context.import_file(path);
                    }

                    #[cfg(target_arch = "wasm32")]
                    {
                        let (sender, receiver) = futures::channel::oneshot::channel::<Vec<u8>>();
                        file_receiver = Some(receiver);
                        let task = rfd::AsyncFileDialog::new()
                            .add_filter("GLTF / GLB", &["gltf", "glb"])
                            .pick_file();
                        wasm_bindgen_futures::spawn_local(async {
                            let file = task.await;
                            if let Some(file) = file {
                                let bytes = file.read().await;
                                let _ = sender.send(bytes);
                            }
                        });
                    }
                }
            }

            if let winit::event::Event::WindowEvent {
                event:
                    winit::event::WindowEvent::Resized(winit::dpi::PhysicalSize { width, height }),
                ..
            } = event
            {
                renderer.resize(width, height);
            }

            context
                .io
                .receive_event(&event, renderer.gpu.window_center());
            state.receive_event(&mut context, &event);

            if context.should_exit {
                elwt.exit();
            }

            if let winit::event::Event::AboutToWait = event {
                // If we're about to wait for an event, we should poll the file receiver
                // to see if we've received a file from the user.
                #[cfg(target_arch = "wasm32")]
                {
                    if let Some(file_receiver) = file_receiver.as_mut() {
                        if let Ok(Some(bytes)) = file_receiver.try_recv() {
                            log::info!("File received: {} bytes", bytes.len());
                            context.import_slice(&bytes);
                        }
                    }
                }

                if context.should_reload_view {
                    renderer.load_world(&context.world);
                    context.should_reload_view = false;
                } else {
                    renderer.render_frame(&mut context.world);
                }
            }
        })
        .expect("Failed to execute frame!");
}

pub struct Context {
    pub io: crate::io::Io,
    pub world: crate::world::World,
    pub should_exit: bool,
    pub should_reload_view: bool,
}

impl Context {
    pub fn import_file(&mut self, path: impl AsRef<std::path::Path>) {
        self.world = crate::gltf::import_gltf_file(path);

        if self.world.scenes.is_empty() {
            self.world.scenes.push(crate::world::Scene::default());
            self.world.default_scene_index = Some(0);
        }

        if let Some(scene_index) = self.world.default_scene_index {
            self.world.add_camera_to_scenegraph(scene_index);
        }

        self.should_reload_view = true;
    }

    // TODO: merge this with above
    pub fn import_slice(&mut self, bytes: &[u8]) {
        self.world = crate::gltf::import_gltf_slice(&bytes);

        if self.world.scenes.is_empty() {
            self.world.scenes.push(crate::world::Scene::default());
            self.world.default_scene_index = Some(0);
        }

        if let Some(scene_index) = self.world.default_scene_index {
            self.world.add_camera_to_scenegraph(scene_index);
        }

        self.should_reload_view = true;
    }
}

pub trait State {
    /// Called once before the main loop
    fn initialize(&mut self, _context: &mut Context) {}

    /// Called when a winit event is received
    fn receive_event(&mut self, _context: &mut Context, _event: &winit::event::Event<()>) {}

    /// Called every frame prior to rendering
    fn update(&mut self, _context: &mut Context) {}
}
