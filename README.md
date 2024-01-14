# mandelbrot-wgpu

A simple GPU based [MandelBrot](https://en.wikipedia.org/wiki/Mandelbrot_set) image (png) generator using Rust + WebGPU [Compute Shader](https://webgpufundamentals.org/webgpu/lessons/webgpu-compute-shaders.html).

![Sample](samples/mandelbrot.png)

# Performance

To render a 32768 x 32768 pixel image it takes about **5m** on my desktop with a RTX3060 GPU. The [CPU version](https://github.com/vishpat/Practice/blob/master/rust/mandelbrot/src/main.rs) (12th Gen Intel(R) Core(TM) i7-12700F) the same machine takes about **10m**