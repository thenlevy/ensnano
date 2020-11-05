use super::helix::Helix;
pub use crate::design::Nucl;
use lyon::math::Point;
use lyon::path::Path;
use lyon::tessellation;
use lyon::tessellation::{StrokeVertex, StrokeVertexConstructor};
use ultraviolet::Vec2;

type Vertices = lyon::tessellation::VertexBuffers<StrandVertex, u16>;

pub struct Strand {
    pub color: u32,
    pub points: Vec<Nucl>,
    pub id: usize,
}

impl Strand {
    pub fn new(color: u32, points: Vec<Nucl>, id: usize) -> Self {
        Self { color, points, id }
    }

    pub fn to_vertices(&self, helices: &[Helix], free_end: &Option<FreeEnd>) -> Vertices {
        let mut vertices = Vertices::new();
        let color = crate::utils::instance::Instance::color_from_u32(self.color);
        let color = [color.x, color.y, color.z, color.w];
        let mut stroke_tess = lyon::tessellation::StrokeTessellator::new();

        let mut builder = Path::builder_with_attributes(1);
        // TODO use builder with attribute to put z_coordinates
        let mut last_pos = None;
        let mut last_helix = None;
        let mut last_forward = None;
        let mut last_depth = None;
        for (i, nucl) in self.points.iter().enumerate() {
            let position = helices[nucl.helix].get_nucl_position(nucl);
            let depth = helices[nucl.helix].get_depth();
            let point = Point::new(position.x, position.y);
            if i == 0 {
                builder.begin(point, &[depth]);
            } else if Some(nucl.helix) != last_helix && last_pos.is_some() {
                let last_pos = last_pos.unwrap();
                let normal = {
                    let diff = (position - last_pos).normalized();
                    Vec2::new(diff.y, diff.x)
                };
                let control = (last_pos + position) / 2. + normal / 3.;
                let depth = depth.min(last_depth.unwrap());
                builder.quadratic_bezier_to(Point::new(control.x, control.y), point, &[depth]);
            } else {
                builder.line_to(point, &[depth]);
            }
            last_helix = Some(nucl.helix);
            last_pos = Some(position);
            last_forward = Some(nucl.forward);
            last_depth = Some(depth);
        }
        match free_end {
            Some(FreeEnd {
                strand_id,
                point: position,
            }) if *strand_id == self.id => {
                let last_pos = last_pos.unwrap();
                let point = Point::new(position.x, position.y);
                let normal = {
                    let diff = (*position - last_pos).normalized();
                    Vec2::new(diff.y, diff.x)
                };
                let control = (last_pos + *position) / 2. + normal / 3.;
                let depth = 1e-6;
                builder.quadratic_bezier_to(Point::new(control.x, control.y), point, &[depth]);
            }
            _ => {
                // Draw the tick of the 3' end if the strand is not empty
                if last_forward == Some(true) {
                    let depth = last_depth.expect("last depth");
                    let up = last_pos.unwrap() + 0.075 * Vec2::unit_y();
                    let arrow_end = up + Vec2::new(-0.25, 0.25);
                    builder.line_to(Point::new(up.x, up.y), &[depth]);
                    builder.line_to(Point::new(arrow_end.x, arrow_end.y), &[depth]);
                } else if last_forward == Some(false) {
                    let depth = last_depth.expect("last depth");
                    let down = last_pos.unwrap() - 0.075 * Vec2::unit_y();
                    let arrow_end = down + Vec2::new(0.25, -0.25);
                    builder.line_to(Point::new(down.x, down.y), &[depth]);
                    builder.line_to(Point::new(arrow_end.x, arrow_end.y), &[depth]);
                }
            }
        }
        builder.end(false);
        let path = builder.build();
        stroke_tess
            .tessellate_path(
                &path,
                &tessellation::StrokeOptions::tolerance(0.01),
                &mut tessellation::BuffersBuilder::new(&mut vertices, WithColor(color)),
            )
            .expect("Error durring tessellation");
        vertices
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct StrandVertex {
    position: [f32; 2],
    normal: [f32; 2],
    color: [f32; 4],
    depth: f32,
}
unsafe impl bytemuck::Pod for StrandVertex {}
unsafe impl bytemuck::Zeroable for StrandVertex {}

pub struct WithColor([f32; 4]);

impl StrokeVertexConstructor<StrandVertex> for WithColor {
    fn new_vertex(&mut self, mut vertex: StrokeVertex) -> StrandVertex {
        StrandVertex {
            position: vertex.position_on_path().to_array(),
            normal: vertex.normal().to_array(),
            color: self.0,
            depth: vertex.interpolated_attributes()[0],
        }
    }
}

pub struct FreeEnd {
    pub strand_id: usize,
    pub point: Vec2,
}
