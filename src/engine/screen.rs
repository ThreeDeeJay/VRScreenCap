use cgmath::Zero;

use super::{entity::Entity, geometry::Mesh};

pub struct Screen {
    pub mesh: Mesh,
    pub entity: Entity,
    pub aspect_ratio: f32,
    pub scale: f32
}

impl Screen {
    pub fn new(distance: f32, scale: f32, aspect_ratio: f32) -> Screen {
        Screen {
            mesh: Mesh::get_plane_rectangle(100, 100, 1.0, 1.0, 0.0),
            entity: Entity::new(
                0,
                cgmath::Vector3 {
                    x: 0.0,
                    y: 0.0,
                    z: distance,
                },
                cgmath::Quaternion::zero(),
                cgmath::Vector3 {
                    x: scale * aspect_ratio,
                    y: scale,
                    z: scale,
                },
            ),
            scale,
            aspect_ratio,
        }
    }

    pub fn change_aspect_ratio(&mut self, aspect_ratio: f32) {
        self.aspect_ratio = aspect_ratio;
        self.entity.scale.x = self.scale * self.aspect_ratio;
        self.entity.update_matrices(&[]);
    }

    pub fn change_scale(&mut self, scale: f32) {
        self.scale = scale;
        self.entity.scale.x = self.scale * self.aspect_ratio;
        self.entity.scale.y = self.scale;
        self.entity.scale.z = self.scale;
        self.entity.update_matrices(&[]);
    }

    pub fn change_distance(&mut self, distance: f32) {
        self.entity.position.z = -distance;
        self.entity.update_matrices(&[]);
    }
}