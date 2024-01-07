use pollster::block_on;

const WORKGROUP_SIZE: u64 = 128;
const WIDTH: usize = (WORKGROUP_SIZE * 4) as usize;
const HEIGHT: usize = (WORKGROUP_SIZE * 4) as usize;
const SIZE: wgpu::BufferAddress = (WIDTH * HEIGHT) as wgpu::BufferAddress;

async fn gpu_device_queue() -> (wgpu::Device, wgpu::Queue) {
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
        label: Some("MandelBrot Fragment Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("mandelbrot.wgsl").into()),
    })
}

async fn cpu_buffer(device: &wgpu::Device, size: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("CPU Buffer"),
        size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

async fn gpu_buffer(device: &wgpu::Device, size: u64) -> wgpu::Buffer {
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
    let (device, queue) = gpu_device_queue().await;

    let size = (SIZE as usize * std::mem::size_of::<u32>()) as u64;
    let cpu_buf = cpu_buffer(&device, size).await;
    let gpu_buf = gpu_buffer(&device, size).await;

    let cs_module = compute_shader(&device).await;
    let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("MandelBrot Compute Pipeline"),
        layout: None,
        module: &cs_module,
        entry_point: "main",
    });

    let bind_group_layout = compute_pipeline.get_bind_group_layout(0);
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: gpu_buf.as_entire_binding(),
        }],
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Command Encoder"),
    });

    {
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Compute Pass"),
            timestamp_writes: None,
        });

        cpass.set_pipeline(&compute_pipeline);
        cpass.set_bind_group(0, &bind_group, &[]);
        cpass.insert_debug_marker("MandelBrot Compute Pass");
        cpass.dispatch_workgroups((SIZE / WORKGROUP_SIZE) as u32, 1, 1);
    }

    encoder.copy_buffer_to_buffer(&gpu_buf, 0, &cpu_buf, 0, size);

    queue.submit(Some(encoder.finish()));
    let buffer_slice = cpu_buf.slice(..);
    let (sender, receiver) = flume::bounded(1);
    buffer_slice.map_async(wgpu::MapMode::Read, move |v| sender.send(v).unwrap());
    device.poll(wgpu::Maintain::Wait);

    if let Ok(Ok(())) = receiver.recv_async().await {
        let data = buffer_slice.get_mapped_range();
        let result: Vec<u32> = bytemuck::cast_slice(&data).to_vec();
        println!("{:?}", result);
        println!("Size {}", SIZE);
        drop(data);
        cpu_buf.unmap(); 
        
    } else {
        panic!("failed to run compute on gpu!")
    }
}

fn main() {
    env_logger::init();
    block_on(run());
}
