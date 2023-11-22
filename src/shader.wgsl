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
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>, // This will be used for normal calculation in the fragment shader
};

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(model.position, 1.0);
    out.world_position = model.position; // Pass the position to the fragment shader for normal calculation
    return out;
}
@group(0) @binding(0)
var<uniform> u_Color: vec4<f32>;

@group(0) @binding(1)
var<uniform> u_lightDirection: vec3<f32>;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(in.world_position.xyz); // Normalize the position to use as a normal
    let light_dir = normalize(u_lightDirection); // Ensure the light direction is normalized
    let lambertian = max(dot(normal, light_dir), 0.0); // Lambertian reflectance

    let diffuse = lambertian; // Simple diffuse lighting
    let ambient = 0.1; // Ambient light level to ensure that the dark side isn't completely black

    let lighting = ambient + diffuse; // Combine diffuse and ambient light
    let color = u_Color * lighting; // Apply lighting to the base color

    return color;
}