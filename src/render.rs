pub struct Renderer<'window> {
    pub gpu: crate::gpu::Gpu<'window>,
    pub view: Option<crate::view::WorldRender>,
    pub depth_texture_view: wgpu::TextureView,
}

impl<'window> Renderer<'window> {
    pub async fn new(
        window: impl Into<wgpu::SurfaceTarget<'window>>,
        width: u32,
        height: u32,
    ) -> Self {
        let gpu = crate::gpu::Gpu::new(window, width, height).await;
        let depth_texture_view =
            gpu.create_depth_texture(gpu.surface_config.width, gpu.surface_config.height);
        Self {
            gpu,
            view: None,
            depth_texture_view,
        }
    }

    pub fn load_world(&mut self, world: &crate::world::World) {
        let _ = std::mem::replace(
            &mut self.view,
            Some(crate::view::WorldRender::new(&self.gpu, world)),
        );
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.gpu.resize(width, height);
        self.depth_texture_view = self.gpu.create_depth_texture(
            self.gpu.surface_config.width,
            self.gpu.surface_config.height,
        );
    }

    pub fn render_frame(&mut self, world: &mut crate::world::World) {
        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let surface_texture = self
            .gpu
            .surface
            .get_current_texture()
            .expect("Failed to get surface texture!");

        let surface_texture_view =
            surface_texture
                .texture
                .create_view(&wgpu::TextureViewDescriptor {
                    label: wgpu::Label::default(),
                    aspect: wgpu::TextureAspect::default(),
                    format: None,
                    dimension: None,
                    base_mip_level: 0,
                    mip_level_count: None,
                    base_array_layer: 0,
                    array_layer_count: None,
                });

        encoder.insert_debug_marker("Render scene");

        // This scope around the render_pass prevents the
        // render_pass from holding a borrow to the encoder,
        // which would prevent calling `.finish()` in
        // preparation for queue submission.
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.19,
                            g: 0.24,
                            b: 0.42,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            if let Some(view) = self.view.as_mut() {
                view.render(&mut render_pass, &self.gpu, world);
            }
        }

        self.gpu.queue.submit(std::iter::once(encoder.finish()));

        surface_texture.present();
    }
}

impl From<crate::world::Sampler> for wgpu::SamplerDescriptor<'static> {
    fn from(sampler: crate::world::Sampler) -> Self {
        let min_filter = match sampler.min_filter {
            crate::world::MinFilter::Linear
            | crate::world::MinFilter::LinearMipmapLinear
            | crate::world::MinFilter::LinearMipmapNearest => wgpu::FilterMode::Linear,
            crate::world::MinFilter::Nearest
            | crate::world::MinFilter::NearestMipmapLinear
            | crate::world::MinFilter::NearestMipmapNearest => wgpu::FilterMode::Nearest,
        };

        let mipmap_filter = match sampler.min_filter {
            crate::world::MinFilter::Linear
            | crate::world::MinFilter::LinearMipmapLinear
            | crate::world::MinFilter::LinearMipmapNearest => wgpu::FilterMode::Linear,
            crate::world::MinFilter::Nearest
            | crate::world::MinFilter::NearestMipmapLinear
            | crate::world::MinFilter::NearestMipmapNearest => wgpu::FilterMode::Nearest,
        };

        let mag_filter = match sampler.mag_filter {
            crate::world::MagFilter::Linear => wgpu::FilterMode::Linear,
            crate::world::MagFilter::Nearest => wgpu::FilterMode::Nearest,
        };

        let address_mode_u = match sampler.wrap_s {
            crate::world::WrappingMode::ClampToEdge => wgpu::AddressMode::ClampToEdge,
            crate::world::WrappingMode::MirroredRepeat => wgpu::AddressMode::MirrorRepeat,
            crate::world::WrappingMode::Repeat => wgpu::AddressMode::Repeat,
        };

        let address_mode_v = match sampler.wrap_t {
            crate::world::WrappingMode::ClampToEdge => wgpu::AddressMode::ClampToEdge,
            crate::world::WrappingMode::MirroredRepeat => wgpu::AddressMode::MirrorRepeat,
            crate::world::WrappingMode::Repeat => wgpu::AddressMode::Repeat,
        };

        let address_mode_w = wgpu::AddressMode::Repeat;

        wgpu::SamplerDescriptor {
            address_mode_u,
            address_mode_v,
            address_mode_w,
            mag_filter,
            min_filter,
            mipmap_filter,
            ..Default::default()
        }
    }
}
