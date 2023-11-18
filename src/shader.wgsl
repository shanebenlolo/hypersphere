// vertex shader
struct CameraUniform {
    view_proj: mat4x4<f32>,
};
@group(1) @binding(0) 
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
}

struct VertexOutput {
    // clip_position === gl_position 
    // if your window is 800x600, the x and y of 
    // clip_position would be between 0-800 and 0-600 
    // respectively with the y = 0 being the top of the screen.
    @builtin(position) clip_position: vec4<f32>,
};

@vertex
fn vs_main (
   model: VertexInput
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(model.position, 1.0);
    return out;
}

@group(0) @binding(0)
var<uniform> u_Color: vec4<f32>;

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return u_Color; // Use the uniform color
}