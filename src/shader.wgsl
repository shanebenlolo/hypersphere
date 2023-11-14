// vertex shader

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
    // clip_position === gl_position 
    // if your window is 800x600, the x and y of 
    // clip_position would be between 0-800 and 0-600 
    // respectively with the y = 0 being the top of the screen.
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>, 
};

@vertex
fn vs_main (
   model: VertexInput
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(model.position, 1.0);
    out.tex_coords = model.tex_coords;
    return out;
}

// fragment shader
@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0)@binding(1)
var s_diffuse: sampler;

@fragment
// @location(0) tell WGPU to store the vec4 value 
// returned by this function in the first tex_coords target.
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, in.tex_coords);
}