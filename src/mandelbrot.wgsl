struct Params {
    width: u32,
    height: u32,
    x: f32,
    y: f32,
    x_range: f32,
    y_range: f32,
    max_iter: u32,
};

@group(0)
@binding(0)
var<storage, read_write> v_indices: array<u32>; 

@group(1)
@binding(0)
var<uniform> params: Params;

@compute
@workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let row = global_id.x / params.width;
    let col = global_id.x % params.width;

    var min_x = (params.x - params.x_range)/2.0;
    var max_y = (params.y + params.y_range)/2.0;

    var x = min_x + (params.x_range * f32(col)) / f32(params.width);
    var y = max_y - (params.y_range * f32(row)) / f32(params.height);

    var old_x = x;
    var old_y = y;

    for (var i = 0u; i < params.max_iter; i = i + 1u) {
        let a = x * x - y * y;
        let b = 2.0 * x * y;
        x = a + old_x;
        y = b + old_y;
        if (x * x + y * y > 4.0) {
            v_indices[global_id.x] = 0u;
            return;
        }
    }

    v_indices[global_id.x] = 16777215u;
}