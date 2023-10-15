// vertex shader

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
}

struct VertexOutput {
    // clip_position === gl_position 
    // if your window is 800x600, the x and y of 
    // clip_position would be between 0-800 and 0-600 
    // respectively with the y = 0 being the top of the screen.
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>, 
};

@vertex
fn vs_main (
   model: VertexInput
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(model.position, 1.0);
    out.color = model.color;
    return out;
}

// fragment shader
@fragment
// @location(0) tell WGPU to store the vec4 value 
// returned by this function in the first color target.
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}