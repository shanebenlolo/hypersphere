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
@group(1) @binding(0) var<uniform> model_uniform: ModelUniform; 

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

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
   return vec4<f32>(1.0, 1.0, 0.0, 1.0);
}