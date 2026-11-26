use std::sync::Arc;

use anyhow::Context;

use winit::window::Window;

/// Base application context
pub struct AppContext {
    /// wgpu Instance
    instance: wgpu::Instance,
    /// Application render surface
    surface: wgpu::Surface<'static>,
    /// Application render surface config
    surface_config: wgpu::SurfaceConfiguration,
    /// Application graphics device handle
    adapter: wgpu::Adapter,
    /// Application graphics device connection
    device: wgpu::Device,
    /// Application device command queue handle
    queue: wgpu::Queue,
}

impl AppContext {
    /// Synchronously initialize a new application context for a provided window
    pub fn init(window: Arc<Window>) -> anyhow::Result<Self> {
        pollster::block_on(Self::init_async(window))
    }

    /// Asynchronously initialize a new application context for a provided window
    pub async fn init_async(window: Arc<Window>) -> anyhow::Result<Self> {
        // Create wgpu instance
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());

        // Create render surface
        let surface = instance.create_surface(window.clone())?;

        // Create graphics device handle
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                apply_limit_buckets: true,
            })
            .await?;

        // Create device connection and command queue
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await?;

        let window_size = window.inner_size();

        // Create render surface config
        let mut surface_config = surface
            .get_default_config(
                &adapter,
                window_size.width.max(1),
                window_size.height.max(1),
            )
            .context("Surface is not supported by the adapter")?;

        // Request compatibility for sRGB texture views
        surface_config
            .view_formats
            .push(surface_config.format.add_srgb_suffix());

        surface.configure(&device, &surface_config);

        Ok(Self {
            instance,
            surface,
            surface_config,
            adapter,
            device,
            queue,
        })
    }

    pub fn render(&mut self, window: Arc<Window>) -> anyhow::Result<()> {
        // Get next texture to be drawn to the render surface
        let surface_texture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture) => texture,
            // Skip frame if surface is timed-out or occluded
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(());
            }
            // Drop suboptimal frame, reconfigure and retry
            wgpu::CurrentSurfaceTexture::Suboptimal(texture) => {
                drop(texture);
                self.surface.configure(&self.device, &self.surface_config);

                match self.surface.get_current_texture() {
                    wgpu::CurrentSurfaceTexture::Success(texture)
                    | wgpu::CurrentSurfaceTexture::Suboptimal(texture) => texture,
                    other => anyhow::bail!("Failed to aquire next surface texture: {other:?}"),
                }
            }
            // Reconfigure and retry
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.surface_config);

                match self.surface.get_current_texture() {
                    wgpu::CurrentSurfaceTexture::Success(texture)
                    | wgpu::CurrentSurfaceTexture::Suboptimal(texture) => texture,
                    _ => anyhow::bail!("Failed to aquire next surface texture"),
                }
            }
            // Recreate surface and retry
            wgpu::CurrentSurfaceTexture::Lost => {
                self.surface = self.instance.create_surface(window.clone())?;
                self.surface.configure(&self.device, &self.surface_config);

                match self.surface.get_current_texture() {
                    wgpu::CurrentSurfaceTexture::Success(texture)
                    | wgpu::CurrentSurfaceTexture::Suboptimal(texture) => texture,
                    _ => anyhow::bail!("Failed to aquire next surface texture"),
                }
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                unreachable!("No error scope registered, so validation errors will panic")
            }
        };

        // Create a texture view for the surface texture
        let surface_texture_view =
            surface_texture
                .texture
                .create_view(&wgpu::TextureViewDescriptor {
                    // Ensure view texture is gamma correct for texture format
                    format: Some(self.surface_config.format.add_srgb_suffix()),
                    ..Default::default()
                });

        // Create render command encoder
        let mut command_encoder =
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        // Create render pass
        #[allow(unused_variables)]
        let render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &surface_texture_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        // Render pass draw commands

        // End the render pass and free the encoder
        drop(render_pass);

        // Submit render command to the queue
        self.queue.submit([command_encoder.finish()]);

        // Present rendered frame to the applicaiton window
        window.pre_present_notify();
        self.queue.present(surface_texture);

        Ok(())
    }

    /// Resize application context render surface
    pub fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        // Set render surface size and set the new configuration
        self.surface_config.width = size.width.max(1);
        self.surface_config.height = size.height.max(1);
        self.surface.configure(&self.device, &self.surface_config);
    }
}
