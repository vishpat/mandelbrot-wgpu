# mandelbrot-wgpu

A simple GPU-based [MandelBrot](https://en.wikipedia.org/wiki/Mandelbrot_set) image (png) generator using Rust + WebGPU [Compute Shader](https://webgpufundamentals.org/webgpu/lessons/webgpu-compute-shaders.html). I get a speed of 4 for a 32k x 32k image. I ran these experiments on my Ubuntu Machine with an RTX3060 compared to the [CPU version](https://github.com/vishpat/Practice/tree/master/rust/mandelbrot) (parallelized over 20 cores).

![Sample](samples/mandelbrot.png)

