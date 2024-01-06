use env_logger;
use pollster::block_on;
use wgpu;

async fn GPUDeviceQueue() -> (wgpu::Device, wgpu::Queue) {
    let instance = wgpu::Instance::default();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .unwrap();

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default(), None)
        .await
        .unwrap();

    return (device, queue);
}

async fn run() {
    let (device, queue) = GPUDeviceQueue().await;
    let cs_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Compute Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("mandelbrot.wgsl").into()),
    });
}

fn main() {
    env_logger::init();
    block_on(run());
}
