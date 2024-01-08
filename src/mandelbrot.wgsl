struct ImageSize {
    width: u32,
    height: u32,
};

@group(0)
@binding(0)
var<storage, read_write> v_indices: array<u32>; 

@group(1)
@binding(0)
var<uniform> size: ImageSize;

@compute
@workgroup_size(128, 1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    v_indices[global_id.x] = global_id.x + size.width;
}