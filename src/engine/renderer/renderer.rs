use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};

use cgmath::Point3;
use crossbeam::atomic::AtomicCell;
use wgpu::{util::DeviceExt, Buffer, CommandEncoder, RenderPipeline, SurfaceTexture};

use super::{
    backend::Backend, camera::Camera, pass::Pass, pipeline::voxels::voxel_pipeline,
    texture::Texture,
};

pub struct Renderer<'a> {
    // Backend
    backend: Backend<'a>,
    // Size (in Pixels)
    size: AtomicCell<(u32, u32)>,
    // Flag if the surface has been resized
    resized: AtomicBool,
    // Voxel pipeline
    voxel_pipeline: RenderPipeline,
    // Camera
    camera: Camera,
    // Depth texture
    depth_texture: Mutex<Option<Texture>>,
    // Quad
    quad: Buffer,
}

impl<'a> Renderer<'a> {
    pub fn new(backend: Backend<'a>, size: (u32, u32)) -> Self {
        // Camera related
        let camera = Camera::new(
            Point3::new(0.0, 5.0, 2.0),
            size.0 as f32 / size.1 as f32,
            backend.device(),
            backend.queue().clone(),
        );

        // Quad
        let quad = backend
            .device()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("vengine::voxel_quad"),
                contents: bytemuck::cast_slice(&[
                    [0.0f32, 0.0f32, -1.0f32],
                    [0.0f32, 0.0f32, 0.0f32],
                    [1.0f32, 0.0f32, -1.0f32],
                    [1.0f32, 0.0f32, 0.0f32],
                ]),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let lock = backend.surface_configuration().lock().unwrap();

        let depth_texture =
            Texture::create_depth_texture(backend.device(), &lock, "engine::depth_texture");

        drop(lock);

        let voxel_pipeline = voxel_pipeline(backend.device(), &camera, *backend.surface_format());

        Self {
            backend,
            size: AtomicCell::new(size),
            camera,
            resized: AtomicBool::new(false),
            depth_texture: Mutex::new(Some(depth_texture)),
            quad,
            voxel_pipeline,
        }
    }

    pub fn start_render_pass(
        &self,
        surface_texture: &SurfaceTexture,
    ) -> Result<Pass, wgpu::SurfaceError> {
        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder =
            self.backend()
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("vengine::render_encoder"),
                });

        let depth = self
            .depth_texture
            .lock()
            .unwrap()
            .take()
            .expect("depth texture was already taken, did you finish the render pass");

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        render_pass.set_pipeline(&self.voxel_pipeline);
        render_pass.set_bind_group(0, self.camera.bind_group(), &[]);

        // Quad buffer (bleibt für alle Chunks gleich)
        render_pass.set_vertex_buffer(0, self.quad.slice(..));

        Ok(Pass::new(render_pass.forget_lifetime(), encoder, depth))
    }

    pub fn finish_render_pass(&self, pass: Pass) -> CommandEncoder {
        let (encoder, pass, depth) = pass.into_inner();

        drop(pass);

        let mut lock = self.depth_texture.lock().unwrap();
        *lock = Some(depth);

        encoder
    }

    pub fn backend(&self) -> &Backend {
        &self.backend
    }

    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    pub fn resize(&self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.size.store((width, height));
            self.resized.store(true, Ordering::Relaxed);
        }
    }

    pub fn dimensions(&self) -> (u32, u32) {
        self.size.load()
    }

    pub fn handle_resize(&self) {
        if self.resized.load(Ordering::Relaxed) {
            let (width, height) = self.size.load();
            let mut surface_lock = self.backend().surface_configuration().lock().unwrap();
            surface_lock.width = width;
            surface_lock.height = height;

            self.backend()
                .surface()
                .configure(self.backend().device(), &surface_lock);
            self.camera.set_aspect(width as f32 / height as f32);

            let mut texture_lock = self.depth_texture.lock().unwrap();

            *texture_lock = Some(Texture::create_depth_texture(
                self.backend().device(),
                &surface_lock,
                "engine::depth_texture",
            ));
            self.resized.store(false, Ordering::Relaxed);
        }
    }
}
