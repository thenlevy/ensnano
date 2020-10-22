use super::{Helix2d, Nucl};
use crate::utils::instance::Instance;
use lyon::math::{rect, Point};
use lyon::path::builder::{BorderRadii, PathBuilder};
use lyon::path::Path;
use lyon::tessellation;
use lyon::tessellation::geometry_builder::simple_builder;
use lyon::tessellation::{StrokeVertex, StrokeVertexConstructor};
use ultraviolet::{Isometry2, Mat2, Rotor2, Vec2, Vec4};

type Vertices = lyon::tessellation::VertexBuffers<GpuVertex, u16>;

pub struct Helix {
    /// The first drawn nucleotide
    left: isize,
    /// The first nucleotide that is not drawn
    right: isize,
    isometry: Isometry2,
    scale: f32,
    color: u32,
    z_index: i32,
    stroke_width: f32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct HelixModel {
    color: Vec4,       // padding 0
    position: Vec2,    // padding 2
    rotation: Mat2,    // padding 2
    z_index: i32,      // padding 3
    stroke_width: f32, // padding 0
}

unsafe impl bytemuck::Zeroable for HelixModel {}
unsafe impl bytemuck::Pod for HelixModel {}

impl Helix {
    pub fn new(left: isize, right: isize, position: Vec2) -> Self {
        Self {
            left,
            right,
            isometry: Isometry2::new(position, Rotor2::identity()),
            scale: 1f32,
            color: 0xFF_4A4946,
            z_index: 0,
            stroke_width: 0.1,
        }
    }

    pub fn update(&mut self, helix2d: &Helix2d) {
        self.left = self.left.min(helix2d.left);
        self.right = self.right.max(helix2d.right);
    }

    pub fn to_vertices(&self, model_id: u32) -> Vertices {
        let mut vertices = Vertices::new();
        let left = self.left as f32;
        let right = self.right as f32 + 1.;
        let top = 0.;
        let bottom = 2.;

        let mut stroke_tess = lyon::tessellation::StrokeTessellator::new();

        let mut builder = Path::builder();

        builder.add_rounded_rectangle(
            &rect(left, top, right - left, bottom - top),
            &BorderRadii::new(0.1),
            lyon::tessellation::path::Winding::Positive,
        );
        for i in (self.left + 1)..=self.right {
            builder.begin(Point::new(i as f32, 0.));
            builder.line_to(Point::new(i as f32, 2.));
            builder.end(false);
        }
        builder.begin(Point::new(left, 1.));
        builder.line_to(Point::new(right, 1.));
        builder.end(false);
        let path = builder.build();
        stroke_tess.tessellate_path(
            &path,
            &tessellation::StrokeOptions::default(),
            &mut tessellation::BuffersBuilder::new(&mut vertices, WithId(model_id)),
        );
        vertices
    }

    pub fn model(&self) -> HelixModel {
        HelixModel {
            color: Instance::color_from_u32(self.color),
            position: self.isometry.translation,
            rotation: self.isometry.rotation.into_matrix(),
            z_index: self.z_index,
            stroke_width: self.stroke_width,
        }
    }

    pub fn get_nucl_position(&self, nucl: &Nucl) -> Vec2 {
        let local_position = nucl.position as f32 * Vec2::unit_x()
            + Vec2::new(0.5, 0.5)
            + if nucl.forward {
                Vec2::zero()
            } else {
                Vec2::unit_y()
            };

        self.isometry * (self.scale * local_position)
    }

    /// Return the nucleotide displayed at position (x, y) or None if (x, y) is outside the helix
    pub fn get_click(&self, x: f32, y: f32) -> Option<(isize, bool)> {
        let click = self.isometry.inversed() * Vec2::new(x, y);
        let forward = if click.y >= 0. && click.y <= 1. {
            Some(true)
        } else if click.y >= 1. && click.y <= 2. {
            Some(false)
        } else {
            None
        }?;
        let position = click.x.floor() as isize;
        if position >= self.left && position <= self.right {
            Some((position, forward))
        } else {
            None
        }
    }

    pub fn get_position(&self) -> Vec2 {
        self.isometry.translation
    }

    pub fn set_position(&mut self, position: Vec2) {
        self.isometry.translation = position
    }

    pub fn set_color(&mut self, color: u32) {
        self.color = color
    }

}

pub enum Extremity {
    Inside,
    Prime5,
    Prime3,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct GpuVertex {
    position: [f32; 2],
    normal: [f32; 2],
    prim_id: u32,
}
unsafe impl bytemuck::Pod for GpuVertex {}
unsafe impl bytemuck::Zeroable for GpuVertex {}

/// This vertex constructor forwards the positions and normals provided by the
/// tessellators and add a shape id.
pub struct WithId(pub u32);

impl StrokeVertexConstructor<GpuVertex> for WithId {
    fn new_vertex(&mut self, vertex: StrokeVertex) -> GpuVertex {
        GpuVertex {
            position: vertex.position_on_path().to_array(),
            normal: vertex.normal().to_array(),
            prim_id: self.0,
        }
    }
}
