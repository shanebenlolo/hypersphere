pub struct CoordinatesSystem {}

impl CoordinatesSystem {
    pub fn lat_lon_to_cartesian(lat: f32, lon: f32, radius: f32) -> [f32; 3] {
        let phi = (90.0 - lat).to_radians();
        let theta = (360.0 - lon).to_radians();

        let x = radius * phi.sin() * theta.cos();
        let y = radius * phi.cos();
        let z = radius * phi.sin() * theta.sin();

        [x, y, z]
    }
}
