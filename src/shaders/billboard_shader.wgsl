struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) texCoords: vec2<f32>, // Texture coordinates
};

struct CameraUniform {
    view_proj_matrix: mat4x4<f32>,
    view_matrix: mat4x4<f32>,
    eye:  vec4<f32>
};

struct ModelUniform {
    model: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> cameraUniform: CameraUniform;
@group(2) @binding(0) var<uniform> modelUniform: ModelUniform;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>, // Texture coordinates passed from the vertex shader
};

@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    var output: VertexOutput;

    // Extract the rotation part of the view matrix
    let view_rotation: mat3x3<f32> = mat3x3<f32>(
        cameraUniform.view_matrix[0].xyz, // Right vector
        cameraUniform.view_matrix[1].xyz, // Up vector
        cameraUniform.view_matrix[2].xyz  // Forward vector
    );

    // Reverse the rotation to face the camera
    let billboard_rotation: mat3x3<f32> = transpose(view_rotation);

    // Apply the billboard rotation, ignore the model's rotation by not using its upper 3x3 part
    let model_translation: vec3<f32> = modelUniform.model[3].xyz;
    let world_position: vec4<f32> = vec4<f32>(
        billboard_rotation * vertex.position.xyz + model_translation, 1.0
    );

    // Apply the view-projection matrix to transform the vertex position into clip space
    output.clip_position = cameraUniform.view_proj_matrix * world_position;

    // Set the texture coordinates for the fragment shader
    output.tex_coords = vertex.texCoords;

    return output;
}

@group(1) @binding(0) var billboardTexture: texture_2d<f32>;
@group(1) @binding(1) var billboardSampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Sample the texture using the texture coordinates
    let texture_color = textureSample(billboardTexture, billboardSampler, in.tex_coords);

    // Return the sampled color
    return texture_color;
}
