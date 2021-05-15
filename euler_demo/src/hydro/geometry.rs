



/**
 * A 3D vector
 */
pub struct Vector3d(f64, f64, f64);




// ============================================================================
impl Vector3d {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Vector3d(x, y, z)
    }
}




/**
 * Enum to hold a unit vector in 3D space
 */
#[derive(Clone, Copy)]
pub enum Direction { I, J, K }




// ============================================================================
impl Direction {
    pub fn along(&self, other: Direction) -> f64 {
        match (self, other) {
            (Direction::I, Direction::I) => 1.0,
            (Direction::J, Direction::J) => 1.0,
            (Direction::K, Direction::K) => 1.0,
            _ => 0.0,
        }
    }
}
