use super::super::view::InsertionInstance;
use super::super::FlatSelection;
use super::helix::{Helix, Shift};
use super::FlatNucl;
use lyon::math::Point;
use lyon::path::Path;
use lyon::tessellation;
use lyon::tessellation::{StrokeVertex, StrokeVertexConstructor};
use ultraviolet::Vec2;

type Vertices = lyon::tessellation::VertexBuffers<StrandVertex, u16>;

pub struct Strand {
    pub color: u32,
    pub points: Vec<FlatNucl>,
    pub insertions: Vec<FlatNucl>,
    pub id: usize,
    pub highlight: bool,
}

impl Strand {
    pub fn new(
        color: u32,
        points: Vec<FlatNucl>,
        insertions: Vec<FlatNucl>,
        id: usize,
        highlight: bool,
    ) -> Self {
        Self {
            color,
            points,
            id,
            insertions,
            highlight,
        }
    }

    pub fn to_vertices(
        &self,
        helices: &[Helix],
        free_end: &Option<FreeEnd>,
        selection: &FlatSelection,
    ) -> Vertices {
        let mut vertices = Vertices::new();
        if self.points.len() == 0 {
            return vertices;
        }
        let color = if self.highlight {
            crate::utils::instance::Instance::color_from_au32(self.color)
        } else {
            crate::utils::instance::Instance::color_from_u32(self.color)
        };
        let color = [color.x, color.y, color.z, color.w];
        let mut stroke_tess = lyon::tessellation::StrokeTessellator::new();

        let mut builder = Path::builder_with_attributes(2);
        let mut last_nucl: Option<FlatNucl> = None;
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
        let mut nb_point_helix = 0;

        for (i, nucl) in self.points.iter().enumerate() {
            let position = helices[nucl.helix].get_nucl_position(&nucl, Shift::Prime5);
            let depth = helices[nucl.helix].get_depth();
            let point = Point::new(position.x, position.y);
            let xover = if last_point.is_some() {
                if Some(nucl.helix) == last_nucl.map(|n| n.helix) {
                    nb_point_helix += 1;
                    nb_point_helix % 2 == 0
                } else {
                    nb_point_helix = 0;
                    true
                }
            } else {
                false
            };
            if i == 0 && last_point.is_none() {
                builder.begin(point, &[depth, sign]);
            //} else if last_point.is_some() && Some(nucl.helix) != last_nucl.map(|n| n.helix) {
            } else if xover {
                let cst = if let FlatSelection::Bound(_, n1, n2) = *selection {
                    if n1 == *nucl || n2 == *nucl {
                        5.
                    } else {
                        1.
                    }
                } else {
                    1.
                };
                let depth = depth.min(last_depth.unwrap_or(depth));
                if let Some(nucl) = last_nucl {
                    // We are drawing a xover
                    let point = helices[nucl.helix].get_nucl_position(&nucl, Shift::Prime3);
                    builder.line_to(Point::new(point.x, point.y), &[depth, sign]);
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
            let point = helices[nucl.helix].get_nucl_position(&nucl, Shift::Prime3);
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
                let depth = 1e-4;
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
                &mut tessellation::BuffersBuilder::new(
                    &mut vertices,
                    WithAttributes {
                        color,
                        highlight: self.highlight,
                    },
                ),
            )
            .expect("Error durring tessellation");
        vertices
    }

    pub fn get_insertions(&self, helices: &[Helix]) -> Vec<InsertionInstance> {
        let mut ret = Vec::with_capacity(self.insertions.len());
        for i in self.insertions.iter() {
            ret.push(helices[i.helix].insertion_instance(i, self.color));
        }
        ret
    }

    pub fn indication(nucl1: FlatNucl, nucl2: FlatNucl, helices: &[Helix]) -> Vertices {
        let mut vertices = Vertices::new();
        let mut builder = Path::builder_with_attributes(2);
        let color = [0.823, 0.525, 0.058, 0.75];
        let start = helices[nucl1.helix].get_nucl_position(&nucl1, Shift::No);
        let end = helices[nucl2.helix].get_nucl_position(&nucl2, Shift::No);

        builder.begin(Point::new(start.x, start.y), &[1e-4, 1.]);
        builder.line_to(Point::new(end.x, end.y), &[1e-4, 1.]);
        let mut stroke_tess = lyon::tessellation::StrokeTessellator::new();

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
                &mut tessellation::BuffersBuilder::new(
                    &mut vertices,
                    WithAttributes {
                        color,
                        highlight: false,
                    },
                ),
            )
            .expect("Error durring tessellation");
        vertices
    }

    pub fn highlighted(&self, color: u32) -> Self {
        Self {
            color,
            highlight: true,
            points: self.points.clone(),
            insertions: self.insertions.clone(),
            ..*self.clone()
        }
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

pub struct WithAttributes {
    color: [f32; 4],
    highlight: bool,
}

impl StrokeVertexConstructor<StrandVertex> for WithAttributes {
    fn new_vertex(&mut self, mut vertex: StrokeVertex) -> StrandVertex {
        let mut width = vertex.interpolated_attributes()[1].powi(2).max(0.3);
        if self.highlight {
            width *= 1.3;
        }
        let color = self.color;

        let mut depth = if vertex.interpolated_attributes()[1] > 1.00001 {
            1e-7
        } else {
            vertex.interpolated_attributes()[0]
        };
        if self.highlight {
            depth *= 0.99;
        }

        StrandVertex {
            position: vertex.position_on_path().to_array(),
            normal: vertex.normal().to_array(),
            color,
            depth,
            width,
        }
    }
}

pub struct FreeEnd {
    pub strand_id: usize,
    pub point: Vec2,
    pub prime3: bool,
}
