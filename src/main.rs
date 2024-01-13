use ndarray::Array2;

const WORKGROUP_SIZE: u64 = 32;
const WIDTH: usize = (WORKGROUP_SIZE * 1) as usize;
const HEIGHT: usize = (WORKGROUP_SIZE * 1) as usize;
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
    param_bind_group: wgpu::BindGroup,
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
            .request_device(&wgpu::DeviceDescriptor::default(), None)
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
            label: Some("Param Uniform Buffer"),
            size: std::mem::size_of::<Params>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
            mapped_at_creation: false,
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: None,
            module: &shader,
            entry_point: "main",
        });

        let param_group_layout = pipeline.get_bind_group_layout(0);
        let param_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &param_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: param_buf.as_entire_binding(),
            }],
        });

        let bind_group_layout = pipeline.get_bind_group_layout(1);
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: gpu_buf.as_entire_binding(),
            }],
        });

        WgpuContext {
            device,
            queue,
            pipeline,
            param_bind_group,
            bind_group,
            param_buf,
            cpu_buf,
            gpu_buf,
        }
    }
}

async fn run() {
    let size = (SIZE as usize * std::mem::size_of::<u32>()) as usize;
    let context = WgpuContext::new(
        size, size    
    )
    .await;

    let params = Params {
        width: WIDTH as u32,
        height: HEIGHT as u32,
        x: -0.65,
        y: 0.0,
        x_range: 3.4,
        y_range: 3.4,
        max_iter: 1000,
    };

    println!("params: {:?}", params);

    context
        .queue
        .write_buffer(&context.param_buf, 0, bytemuck::cast_slice(&[params]));

    let mut encoder = context
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Command Encoder"),
        });

    {
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Compute Pass"),
            timestamp_writes: None,
        });

        cpass.set_pipeline(&context.pipeline);
        cpass.set_bind_group(0, &context.param_bind_group, &[]);
        cpass.set_bind_group(1, &context.bind_group, &[]);
        cpass.insert_debug_marker("MandelBrot Compute Pass");
        cpass.dispatch_workgroups((SIZE / WORKGROUP_SIZE) as u32, 1, 1);
    }

    encoder.copy_buffer_to_buffer(&context.gpu_buf, 0, &context.cpu_buf, 0, SIZE);
    context.queue.submit(Some(encoder.finish()));

    let buffer_slice = context.cpu_buf.slice(..);
    let (sender, receiver) = flume::bounded(1);
    buffer_slice.map_async(wgpu::MapMode::Read, move |v| sender.send(v).unwrap());
    context.device.poll(wgpu::Maintain::Wait);

    if let Ok(Ok(())) = receiver.recv_async().await {
        let data = buffer_slice.get_mapped_range();
        let _result: Vec<u32> = bytemuck::cast_slice(&data).to_vec();
        println!("result: {:?}", _result);
        let pixels = Array2::from_shape_vec((HEIGHT, WIDTH), _result).unwrap();
        let img = image::ImageBuffer::from_fn(WIDTH as u32, HEIGHT as u32, |x, y| {
            let pixel = pixels[[y as usize, x as usize]];
            image::Rgb([(pixel >> 16) as u8, (pixel >> 8) as u8, pixel as u8])
        });
        img.save("mandelbrot.png").unwrap();
        drop(data);
        context.cpu_buf.unmap();
    } else {
        panic!("failed to run compute on gpu!")
    }
}

fn main() {
    env_logger::init();
    pollster::block_on(run());
}
