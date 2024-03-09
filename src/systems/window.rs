use crate::components::camera::CameraComponent;
use cgmath::{InnerSpace, SquareMatrix};

pub struct WindowSystem {}

impl WindowSystem {
    pub fn handle_left_click(
        screen_width: f32,
        screen_height: f32,
        position_x: f32,
        position_y: f32,
        globe_radius: f32,
        camera_component: &CameraComponent,
    ) -> Option<(f32, f32)> {
        let view_proj_matrix =
            cgmath::Matrix4::from(camera_component.camera_uniform.view_proj_matrix);

        let mouse_pos_clip_near = cgmath::Vector4::new(
            (position_x * 2.0) / screen_width - 1.0,
            1.0 - (2.0 * position_y) / screen_height,
            camera_component.camera.znear,
            1.0,
        );

        let mouse_pos_clip_far = cgmath::Vector4::new(
            (position_x * 2.0) / screen_width - 1.0,
            1.0 - (2.0 * position_y) / screen_height,
            camera_component.camera.zfar,
            1.0,
        );

        // Transform these points to world space
        let mouse_pos_world_near = (view_proj_matrix).invert().unwrap() * mouse_pos_clip_near;
        let mouse_pos_world_far = (view_proj_matrix).invert().unwrap() * mouse_pos_clip_far;

        // Convert from homogeneous to Cartesian coordinates
        let mouse_pos_world_near = mouse_pos_world_near.truncate() / mouse_pos_world_near.w;
        let mouse_pos_world_far = mouse_pos_world_far.truncate() / mouse_pos_world_far.w;

        // Create the ray
        // needs to be a matrix4
        let ray_origin = cgmath::Vector3::new(
            camera_component.camera.eye.x,
            camera_component.camera.eye.y,
            camera_component.camera.eye.z,
        );
        let ray_direction = (mouse_pos_world_far - mouse_pos_world_near).normalize();

        let oc = ray_origin - cgmath::Vector3::new(0.0, 0.0, 0.0);
        let a = ray_direction.dot(ray_direction);
        let b = 2.0 * oc.dot(ray_direction);
        let c = oc.dot(oc) - globe_radius * globe_radius;
        let discriminant = b * b - 4.0 * a * c;

        // intersection
        if discriminant >= 0.0 {
            let discriminant_sqrt = discriminant.sqrt();
            let t1 = (-b - discriminant_sqrt) / (2.0 * a);
            let t2 = (-b + discriminant_sqrt) / (2.0 * a);

            let t = if t1 > 0.0 && (t2 < 0.0 || t1 < t2) {
                t1
            } else {
                t2
            };

            let intersection_point = ray_origin + ray_direction * t;

            // Calculate Latitude (φ) and Longitude (λ)
            let normalized_point = intersection_point.normalize(); // Make sure it's on the unit sphere

            let latitude = normalized_point.y.asin().to_degrees(); // Convert radians to degrees
            let longitude = normalized_point.z.atan2(normalized_point.x).to_degrees(); // Convert radians to degrees

            println!("lat: {:?}, lon: {:?}", latitude, longitude);
            Some((latitude, longitude))
        } else {
            None
            // No intersection with the sphere
        }
    }
}
