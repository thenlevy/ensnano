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
        if self.points.len() == 0 {
            return vertices;
        }
        let color = crate::utils::instance::Instance::color_from_u32(self.color);
        let color = [color.x, color.y, color.z, color.w];
        let mut stroke_tess = lyon::tessellation::StrokeTessellator::new();

        let mut builder = Path::builder_with_attributes(2);
        let mut last_nucl: Option<Nucl> = None;
        let mut last_point = match free_end {
            Some(FreeEnd {
                point,
                strand_id,
                prime3,
            }) if *strand_id == self.id && !prime3 => Some(*point),
            _ => None,
        };

        let mut last_depth = None;
        let mut sign = 1.;
        for (i, nucl) in self.points.iter().enumerate() {
            let position = helices[nucl.helix].get_nucl_position(nucl, false);
            let depth = helices[nucl.helix].get_depth();
            let point = Point::new(position.x, position.y);
            if i == 0 && last_point.is_none() {
                builder.begin(point, &[depth, sign]);
            } else if last_point.is_some() && Some(nucl.helix) != last_nucl.map(|n| n.helix) {
                if let Some(nucl) = last_nucl {
                    // We are drawing a xover
                    let point = helices[nucl.helix].get_nucl_position(&nucl, true);
                    builder.line_to(Point::new(point.x, point.y), &[last_depth.unwrap(), sign]);
                } else {
                    // We are drawing the free end
                    let position = last_point.unwrap();
                    builder.begin(Point::new(position.x, position.y), &[depth, sign]);
                }
                let last_pos = last_point.unwrap();
                let normal = {
                    let diff = (position - last_pos).normalized();
                    Vec2::new(diff.y, diff.x)
                };
                let control = (last_pos + position) / 2. + normal / 3.;
                let depth = depth.min(last_depth.unwrap_or(depth));
                sign *= -1.;
                builder.quadratic_bezier_to(
                    Point::new(control.x, control.y),
                    point,
                    &[depth, sign],
                );
            } else {
                builder.line_to(point, &[depth, sign]);
            }
            last_point = Some(position);
            last_nucl = Some(*nucl);
            last_depth = Some(depth);
        }
        if let Some(nucl) = last_nucl {
            let point = helices[nucl.helix].get_nucl_position(&nucl, true);
            builder.line_to(Point::new(point.x, point.y), &[last_depth.unwrap(), sign]);
        }
        match free_end {
            Some(FreeEnd {
                strand_id,
                point: position,
                prime3,
            }) if *strand_id == self.id && *prime3 => {
                let last_pos = last_point.unwrap();
                let point = Point::new(position.x, position.y);
                let normal = {
                    let diff = (*position - last_pos).normalized();
                    Vec2::new(diff.y, diff.x)
                };
                let control = (last_pos + *position) / 2. + normal / 3.;
                let depth = 1e-6;
                sign *= -1.;
                builder.quadratic_bezier_to(
                    Point::new(control.x, control.y),
                    point,
                    &[depth, sign],
                );
            }
            _ => {
                // Draw the tick of the 3' end if the strand is not empty
                if let Some(nucl) = last_nucl {
                    let position = helices[nucl.helix].get_arrow_end(&nucl);
                    let point = Point::new(position.x, position.y);
                    builder.line_to(point, &[last_depth.unwrap(), sign]);
                }
            }
        }
        builder.end(false);
        let path = builder.build();
        stroke_tess
            .tessellate_path(
                &path,
                &tessellation::StrokeOptions::tolerance(0.01)
                    .with_line_cap(tessellation::LineCap::Round)
                    .with_end_cap(tessellation::LineCap::Round)
                    .with_start_cap(tessellation::LineCap::Round)
                    .with_line_join(tessellation::LineJoin::Round),
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
    width: f32,
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
            width: vertex.interpolated_attributes()[1].powi(2).max(0.3),
        }
    }
}

pub struct FreeEnd {
    pub strand_id: usize,
    pub point: Vec2,
    pub prime3: bool,
}
