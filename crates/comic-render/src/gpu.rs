//! The GPU's picture: `comic.wgsl` as a compute pass on a headless device (no window, no surface), one workgroup per
//! tile, the pixels packed into a buffer and read back. The only file that touches wgpu. A software adapter (a CPU
//! dressed as a GPU) is refused: the CPU path draws the same picture, exactly.

use crate::bins::{Bins, bin};
use crate::{Raster, RenderError, RenderOptions, Renderer, check_size, unpremultiply};
use bytemuck::{Pod, Zeroable};
use comic_core::Scene;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct Params {
    width: u32,
    height: u32,
    tiles_x: u32,
    tiles_n: u32,
    layers: u32,
    pad: [u32; 3],
    clear: [f32; 4],
    paper: [f32; 4],
    ink: [f32; 4],
}

/// Draws on the GPU.
pub struct GpuRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    adapter: String,
}

impl GpuRenderer {
    /// A device on the best GPU here, and the pipeline compiled.
    pub fn new() -> Result<GpuRenderer, RenderError> {
        let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        descriptor.backends = wgpu::Backends::PRIMARY;
        let instance = wgpu::Instance::new(descriptor);
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
            ..Default::default()
        }))
        .map_err(|e| RenderError::NoGpu(e.to_string()))?;
        let info = adapter.get_info();
        if info.device_type == wgpu::DeviceType::Cpu {
            return Err(RenderError::NoGpu(format!(
                "{} is a software adapter",
                info.name
            )));
        }
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("comic"),
            ..Default::default()
        }))
        .map_err(|e| RenderError::NoGpu(e.to_string()))?;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("comic.wgsl"),
            source: wgpu::ShaderSource::Wgsl(include_str!("comic.wgsl").into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("comic"),
            layout: None,
            module: &module,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        Ok(GpuRenderer {
            device,
            queue,
            pipeline,
            adapter: format!("{} ({:?})", info.name, info.backend),
        })
    }

    /// The adapter's name and backend.
    pub fn adapter(&self) -> &str {
        &self.adapter
    }

    fn storage<T: Pod>(&self, label: &str, items: &[T]) -> wgpu::Buffer {
        // A binding is never empty: one zeroed element stands in.
        let zero = [T::zeroed()];
        let items: &[T] = if items.is_empty() { &zero } else { items };
        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: bytemuck::cast_slice(items),
                usage: wgpu::BufferUsages::STORAGE,
            })
    }

    fn draw(&self, bins: &Bins, options: &RenderOptions) -> Result<Vec<u8>, RenderError> {
        let params = Params {
            width: bins.width,
            height: bins.height,
            tiles_x: bins.tiles_x,
            tiles_n: bins.tile_count(),
            layers: bins.layers,
            pad: [0; 3],
            clear: options.clear,
            paper: [options.paper[0], options.paper[1], options.paper[2], 1.0],
            ink: [options.ink[0], options.ink[1], options.ink[2], 1.0],
        };
        let size = u64::from(bins.width) * u64::from(bins.height) * 4;
        let limit = self.device.limits().max_storage_buffer_binding_size;
        if size > limit {
            return Err(RenderError::TooLarge {
                width: bins.width,
                height: bins.height,
            });
        }
        let uniform = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let capsules = self.storage("capsules", &bins.capsules);
        let fills = self.storage("fills", &bins.fills);
        let edges = self.storage("edges", &bins.edges);
        let tiles = self.storage("tiles", &bins.tiles);
        let items = self.storage("items", &bins.items);
        let pixels = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("pixels"),
            size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let layout = self.pipeline.get_bind_group_layout(0);
        let group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("comic"),
            layout: &layout,
            entries: &[&uniform, &capsules, &fills, &edges, &tiles, &items, &pixels]
                .iter()
                .enumerate()
                .map(|(i, b)| wgpu::BindGroupEntry {
                    binding: i as u32,
                    resource: b.as_entire_binding(),
                })
                .collect::<Vec<_>>(),
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("comic"),
            });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("comic"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.dispatch_workgroups(bins.tiles_x, bins.tiles_y, 1);
        }
        encoder.copy_buffer_to_buffer(&pixels, 0, &readback, 0, size);
        self.queue.submit([encoder.finish()]);
        let slice = readback.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|e| RenderError::Gpu(e.to_string()))?;
        receiver
            .recv()
            .map_err(|e| RenderError::Gpu(e.to_string()))?
            .map_err(|e| RenderError::Gpu(e.to_string()))?;
        let bytes = slice
            .get_mapped_range()
            .map_err(|e| RenderError::Gpu(e.to_string()))?
            .to_vec();
        readback.unmap();
        Ok(bytes)
    }
}

impl Renderer for GpuRenderer {
    fn name(&self) -> &'static str {
        "gpu"
    }

    fn render(&mut self, scene: &Scene, options: &RenderOptions) -> Result<Raster, RenderError> {
        check_size(scene)?;
        let bins = bin(scene);
        let mut rgba = self.draw(&bins, options)?;
        unpremultiply(&mut rgba);
        Ok(Raster {
            width: scene.width,
            height: scene.height,
            rgba,
        })
    }
}
