struct CameraUniform {
    view_proj: mat4x4<f32>,
};

struct VertexInput {
    @location(0) position: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_direction: vec3<f32>,
};

@group(1) @binding(0) var<uniform> camera: CameraUniform;

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(model.position, 1.0);
    out.world_direction = normalize(model.position); // Use the vertex position as the direction for a sphere centered at the origin
    return out;
}

// fragment
@group(0) @binding(0) var<uniform> u_Color: vec4<f32>;
@group(0) @binding(1) var<uniform> u_lightDirection: vec4<f32>; // vec4 instead of 3 to be 16 bytes so its WebGL2 compliant

@group(2) @binding(0) var globeTexture: texture_cube<f32>;
@group(2) @binding(1) var globeSampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(in.world_direction); // Correctly calculate the normal
    let light_dir = normalize(u_lightDirection.xyz);
    let lambertian = max(dot(normal, light_dir), 0.0);

    // Soften the diffuse lighting by raising lambertian to a fraction power
    let diffuse = pow(lambertian, 0.5); // Raise to the power of 0.5 to soften
    let ambient = 0.1;

    let lighting = ambient + diffuse * 0.9; // Reduce the effect of diffuse lighting slightly
    let color = u_Color * lighting;

    let cubemap_color = textureSample(globeTexture, globeSampler, in.world_direction);
    
    // Adjust the mix factor to create a more gradual transition from light to shadow
    let mix_factor = smoothstep(0.0, 1.0, lambertian); // Smoothstep can create a smoother transition
    return mix(color, cubemap_color, mix_factor); // Blend base color with cubemap using the smoothstep result
}
