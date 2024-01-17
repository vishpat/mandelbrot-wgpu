use ndarray::Array2;
use pollster::block_on;

const CHUNK_LEN: usize = 8;
const WIDTH: usize = CHUNK_LEN * 512 * 8;
const HEIGHT: usize = CHUNK_LEN * 512 * 8;

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

async fn device_queue() -> (wgpu::Device, wgpu::Queue) {
    let instance = wgpu::Instance::default();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .unwrap();

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default(), None)
        .await
        .unwrap();

    (device, queue)
}

async fn compute_shader(device: &wgpu::Device) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("MandelBrot Compute Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("mandelbrot.wgsl").into()),
    })
}

async fn host_buffer(device: &wgpu::Device, size: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("CPU Buffer"),
        size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

async fn device_buffer(device: &wgpu::Device, size: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("GPU Buffer"),
        size,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    })
}

async fn run() {
    let (device, queue) = device_queue().await;

    let gpu_param_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("GPU Parameter Buffer"),
        size: std::mem::size_of::<Params>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let work_size = (WIDTH * HEIGHT * std::mem::size_of::<u32>()) as u64;
    let host_buf = host_buffer(&device, work_size).await;
    let device_buf = device_buffer(&device, work_size).await;

    let cs_module = compute_shader(&device).await;

    let param_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    // Going to have this be None just to be safe.
                    min_binding_size: None,
                },
                count: None,
            }],
        });
    let param_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &param_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: gpu_param_buf.as_entire_binding(),
        }],
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                // Going to have this be None just to be safe.
                min_binding_size: None,
            },
            count: None,
        }],
    });
    // This ties actual resources stored in the GPU to our metaphorical function
    // through the binding slots we defined above.
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: device_buf.as_entire_binding(),
        }],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&param_bind_group_layout, &bind_group_layout],
        push_constant_ranges: &[],
    });

    let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        module: &cs_module,
        entry_point: "main",
    });

    let params = Params {
        width: WIDTH as u32,
        height: HEIGHT as u32,
        x: -0.65,
        y: 0.0,
        x_range: 3.4,
        y_range: 3.4,
        max_iter: 1000,
    };
    queue.write_buffer(&gpu_param_buf, 0, bytemuck::cast_slice(&[params]));

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Command Encoder"),
    });

    {
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Compute Pass"),
            timestamp_writes: None,
        });

        cpass.set_pipeline(&compute_pipeline);

        cpass.set_bind_group(0, &param_bind_group, &[]);
        cpass.set_bind_group(1, &bind_group, &[]);
        cpass.insert_debug_marker("MandelBrot Compute Pass");
        let workgroups_x = (WIDTH / CHUNK_LEN) as u32;
        let workgroups_y = (HEIGHT / CHUNK_LEN) as u32;
        cpass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
    }

    encoder.copy_buffer_to_buffer(&device_buf, 0, &host_buf, 0, work_size);
    queue.submit(Some(encoder.finish()));
    let buffer_slice = host_buf.slice(..);
    let (sender, receiver) = flume::bounded(1);
    buffer_slice.map_async(wgpu::MapMode::Read, move |v| sender.send(v).unwrap());
    device.poll(wgpu::Maintain::Wait);

    if let Ok(Ok(())) = receiver.recv_async().await {
        let data = buffer_slice.get_mapped_range();
        let _result: Vec<u32> = bytemuck::cast_slice(&data).to_vec();
        let pixels = Array2::from_shape_vec((HEIGHT, WIDTH), _result).unwrap();
        let img = image::ImageBuffer::from_fn(WIDTH as u32, HEIGHT as u32, |x, y| {
            let pixel = pixels[[y as usize, x as usize]];
            image::Rgb([(pixel >> 16) as u8, (pixel >> 8) as u8, pixel as u8])
        });
        img.save("mandelbrot.png").unwrap();

        drop(data);
        host_buf.unmap();
    } else {
        panic!("failed to run compute on gpu!")
    }
}

fn main() {
    env_logger::init();
    block_on(run());
}
