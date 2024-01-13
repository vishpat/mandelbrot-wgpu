use ndarray::Array2;
use pollster::block_on;
use wgpu::util::DeviceExt;

const WORKGROUP_SIZE: u64 = 64;
const WIDTH: usize = (WORKGROUP_SIZE * 20) as usize;
const HEIGHT: usize = (WORKGROUP_SIZE * 20) as usize;
const SIZE: wgpu::BufferAddress = (WIDTH * HEIGHT) as wgpu::BufferAddress;

#[repr(C)]
#[derive(Debug, bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
pub struct Params {
    pub width: u32,
    pub height: u32,
    pub x: f32,
    pub y: f32,
    pub x_range: f32,
    pub y_range: f32,
    pub max_iter: u32,
}

struct WgpuContext {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    bind_group: wgpu::BindGroup,
    param_buf: wgpu::Buffer,
    cpu_buf: wgpu::Buffer,
    gpu_buf: wgpu::Buffer,
}

impl WgpuContext {
    async fn new(cpu_buffer_size: usize, gpu_buffer_size: usize) -> WgpuContext {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_defaults(),
                },
                None,
            )
            .await
            .unwrap();

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("MandelBrot Fragment Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("mandelbrot.wgsl").into()),
        });

        let cpu_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("CPU Buffer"),
            size: cpu_buffer_size as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let gpu_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GPU Buffer"),
            size: gpu_buffer_size as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let param_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Parameter Buffer"),
            size: std::mem::size_of::<Params>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
            mapped_at_creation: false,
        });

        // This can be though of as the function signature for our CPU-GPU function.
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: cpu_buf.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });

        WgpuContext {
            device,
            queue,
            pipeline,
            bind_group,
            cpu_buf,
            gpu_buf,
        }
    }
}

fn run() {}

fn main() {
    env_logger::init();
    block_on(run());
}
