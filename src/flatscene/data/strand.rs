use super::Helix;
pub use crate::design::Nucl;
use lyon::math::{rect, Point};
use lyon::path::builder::{BorderRadii, PathBuilder};
use lyon::path::Path;
use lyon::tessellation;
use lyon::tessellation::geometry_builder::simple_builder;
use lyon::tessellation::{StrokeVertex, StrokeVertexConstructor};

type Vertices = lyon::tessellation::VertexBuffers<StrandVertex, u16>;

pub struct Strand {
    pub color: u32,
    pub points: Vec<Nucl>,
}

impl Strand {
    pub fn new(color: u32, points: Vec<Nucl>) -> Self {
        Self { color, points }
    }

    pub fn to_vertices(&self, helices: &Vec<Helix>) -> Vertices {
        let mut vertices = Vertices::new();
        let color = crate::utils::instance::Instance::color_from_u32(self.color);
        let color = [color.x, color.y, color.z, color.w];
        let mut stroke_tess = lyon::tessellation::StrokeTessellator::new();

        let mut builder = Path::builder();
        let mut init = true;
        // TODO use builder with attribute to put z_coordinates
        for nucl in self.points.iter() {
            let position = helices[nucl.helix].get_nucl_position(nucl);
            let point = Point::new(position.x, position.y);
            if init {
                builder.begin(point);
                init = false;
            } else {
                builder.line_to(point);
            }
        }
        builder.end(false);
        let path = builder.build();
        stroke_tess.tessellate_path(
            &path,
            &tessellation::StrokeOptions::default(),
            &mut tessellation::BuffersBuilder::new(&mut vertices, WithColor(color)),
        );
        vertices
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct StrandVertex {
    position: [f32; 2],
    normal: [f32; 2],
    color: [f32; 4],
}
unsafe impl bytemuck::Pod for StrandVertex {}
unsafe impl bytemuck::Zeroable for StrandVertex {}

pub struct WithColor([f32; 4]);

impl StrokeVertexConstructor<StrandVertex> for WithColor {
    fn new_vertex(&mut self, vertex: StrokeVertex) -> StrandVertex {
        StrandVertex {
            position: vertex.position_on_path().to_array(),
            normal: vertex.normal().to_array(),
            color: self.0,
        }
    }
}
