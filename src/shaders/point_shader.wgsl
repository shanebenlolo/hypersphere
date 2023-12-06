struct CameraUniform {
    view_proj: mat4x4<f32>,
    eye:  vec4<f32>
};

struct VertexInput {
    @location(0) position: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_direction: vec3<f32>,
};

struct ModelUniform {
    model: mat4x4<f32>,
};


@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(2) @binding(0) var<uniform> model_uniform: ModelUniform; 

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    // Apply the model matrix to transform the vertex position to world space
    let world_position = model_uniform.model * vec4<f32>(model.position, 1.0);

    // Transform the position from world space to clip space
    out.clip_position = camera.view_proj * world_position;

    // Use the transformed world position for the direction calculation
    out.world_direction = normalize(world_position.xyz - camera.eye.xyz);

    return out;
}

// fragment
@group(1) @binding(0) var globeTexture: texture_cube<f32>;
@group(1) @binding(1) var globeSampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Sample the texture using the texture coordinates from the VertexOutput
    let cubemap_color = textureSample(globeTexture, globeSampler, in.world_direction);

    // Return the sampled color directly without any lighting calculations
    return cubemap_color;
}