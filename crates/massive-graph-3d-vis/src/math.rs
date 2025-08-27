use cgmath::{Vector3, Matrix4, Point3, InnerSpace, EuclideanSpace};

#[derive(Debug, Clone, Copy)]
pub struct Plane {
    pub normal: Vector3<f32>,
    pub distance: f32,
}

impl Plane {
    pub fn new(normal: Vector3<f32>, distance: f32) -> Self {
        Self { normal, distance }
    }
    
    pub fn from_point_normal(point: Point3<f32>, normal: Vector3<f32>) -> Self {
        let normalized = normal.normalize();
        let distance = -normalized.dot(point.to_vec());
        Self { normal: normalized, distance }
    }
    
    // Distance from point to plane (positive = in front, negative = behind)
    pub fn distance_to_point(&self, point: Point3<f32>) -> f32 {
        self.normal.dot(point.to_vec()) + self.distance
    }
}

#[derive(Debug, Clone)]
pub struct Frustum {
    pub planes: [Plane; 6], // left, right, bottom, top, near, far
}

impl Frustum {
    // Extract frustum planes from view-projection matrix
    pub fn from_view_proj_matrix(view_proj: Matrix4<f32>) -> Self {
        // Extract frustum planes using Gribb-Hartmann method
        // Each plane is extracted from the view-projection matrix rows
        
        let m = view_proj;
        
        // Left plane: m[3] + m[0]
        let left = Plane::new(
            Vector3::new(m[3][0] + m[0][0], m[3][1] + m[0][1], m[3][2] + m[0][2]).normalize(),
            m[3][3] + m[0][3]
        );
        
        // Right plane: m[3] - m[0]  
        let right = Plane::new(
            Vector3::new(m[3][0] - m[0][0], m[3][1] - m[0][1], m[3][2] - m[0][2]).normalize(),
            m[3][3] - m[0][3]
        );
        
        // Bottom plane: m[3] + m[1]
        let bottom = Plane::new(
            Vector3::new(m[3][0] + m[1][0], m[3][1] + m[1][1], m[3][2] + m[1][2]).normalize(),
            m[3][3] + m[1][3]
        );
        
        // Top plane: m[3] - m[1]
        let top = Plane::new(
            Vector3::new(m[3][0] - m[1][0], m[3][1] - m[1][1], m[3][2] - m[1][2]).normalize(),
            m[3][3] - m[1][3]
        );
        
        // Near plane: m[3] + m[2]
        let near = Plane::new(
            Vector3::new(m[3][0] + m[2][0], m[3][1] + m[2][1], m[3][2] + m[2][2]).normalize(),
            m[3][3] + m[2][3]
        );
        
        // Far plane: m[3] - m[2]
        let far = Plane::new(
            Vector3::new(m[3][0] - m[2][0], m[3][1] - m[2][1], m[3][2] - m[2][2]).normalize(),
            m[3][3] - m[2][3]
        );
        
        Self {
            planes: [left, right, bottom, top, near, far],
        }
    }
    
    // Test if a sphere is inside the frustum
    pub fn contains_sphere(&self, center: Point3<f32>, radius: f32) -> bool {
        for plane in &self.planes {
            let distance = plane.distance_to_point(center);
            if distance < -radius {
                // Sphere is completely outside this plane
                return false;
            }
        }
        // Sphere is inside or intersecting all planes
        true
    }
    
    // Test if an axis-aligned bounding box is inside the frustum
    pub fn contains_aabb(&self, min: Point3<f32>, max: Point3<f32>) -> bool {
        for plane in &self.planes {
            // Test all 8 corners of the AABB against the plane
            let corners = [
                Point3::new(min.x, min.y, min.z),
                Point3::new(max.x, min.y, min.z),
                Point3::new(min.x, max.y, min.z),
                Point3::new(max.x, max.y, min.z),
                Point3::new(min.x, min.y, max.z),
                Point3::new(max.x, min.y, max.z),
                Point3::new(min.x, max.y, max.z),
                Point3::new(max.x, max.y, max.z),
            ];
            
            let mut all_outside = true;
            for corner in &corners {
                if plane.distance_to_point(*corner) >= 0.0 {
                    all_outside = false;
                    break;
                }
            }
            
            if all_outside {
                // All corners are outside this plane
                return false;
            }
        }
        // AABB intersects or is inside all planes
        true
    }
}

// Simple bounding sphere for objects
#[derive(Debug, Clone, Copy)]
pub struct BoundingSphere {
    pub center: Point3<f32>,
    pub radius: f32,
}

impl BoundingSphere {
    pub fn new(center: Point3<f32>, radius: f32) -> Self {
        Self { center, radius }
    }
    
    pub fn from_array(center: [f32; 3], radius: f32) -> Self {
        Self {
            center: Point3::new(center[0], center[1], center[2]),
            radius,
        }
    }
    
    // Create bounding sphere for a cube at given position with given size
    pub fn for_cube(position: Point3<f32>, size: f32) -> Self {
        // Cube extends from -size/2 to +size/2 in each direction
        // Sphere radius is the distance from center to corner
        let radius = (size * size * 3.0).sqrt() * 1.0; // sqrt(3) * size/2
        Self::new(position, radius)
    }
} 