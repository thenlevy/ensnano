use super::helix::{Extremity, Helix};
pub use crate::design::Nucl;
use lyon::math::{rect, Point};
use lyon::path::builder::{BorderRadii, PathBuilder};
use lyon::path::Path;
use lyon::tessellation;
use lyon::tessellation::geometry_builder::simple_builder;
use lyon::tessellation::{StrokeVertex, StrokeVertexConstructor};
use ultraviolet::Vec2;

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
        // TODO use builder with attribute to put z_coordinates
        let mut last_pos = None;
        let mut last_helix = None;
        let mut last_forward = None;
        for (i, nucl) in self.points.iter().enumerate() {
            let position = helices[nucl.helix].get_nucl_position(nucl);
            let point = Point::new(position.x, position.y);
            if i == 0 {
                builder.begin(point);
            } else if Some(nucl.helix) != last_helix && last_pos.is_some() {
                let last_pos = last_pos.unwrap();
                let normal = {
                    let diff = (position - last_pos).normalized();
                    Vec2::new(diff.y, diff.x)
                };
                let control = (last_pos + position) / 2. + normal / 3.;
                builder.quadratic_bezier_to(Point::new(control.x, control.y), point);
            } else {
                builder.line_to(point);
            }
            last_helix = Some(nucl.helix);
            last_pos = Some(position);
            last_forward = Some(nucl.forward);
        }
        // Draw the tick of the 3' end if the strand is not empty
        if last_forward == Some(true) {
            let up = last_pos.unwrap() + 0.075 * Vec2::unit_y();
            let arrow_end = up + Vec2::new(-0.25, 0.25);
            builder.line_to(Point::new(up.x, up.y));
            builder.line_to(Point::new(arrow_end.x, arrow_end.y));
        } else if last_forward == Some(false) {
            let down = last_pos.unwrap() - 0.075 * Vec2::unit_y();
            let arrow_end = down + Vec2::new(0.25, -0.25);
            builder.line_to(Point::new(down.x, down.y));
            builder.line_to(Point::new(arrow_end.x, arrow_end.y));
        }
        builder.end(false);
        let path = builder.build();
        stroke_tess.tessellate_path(
            &path,
            &tessellation::StrokeOptions::tolerance(0.01),
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
