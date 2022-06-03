/*
ENSnano, a 3d graphical application for DNA nanostructures.
    Copyright (C) 2021  Nicolas Levy <nicolaspierrelevy@gmail.com> and Nicolas Schabanel <nicolas.schabanel@ens-lyon.fr>

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/
use super::*;
use fontdue::layout::Layout;
use ultraviolet::Rotor2;

pub struct TextDrawer {
    char_drawers: HashMap<char, CharDrawer>,
    char_map: HashMap<char, Vec<CharInstance>>,
    layout: Layout<()>,
}

pub struct Sentence<'a> {
    pub text: &'a str,
    pub size: f32,
    pub z_index: i32,
    pub color: Vec4,
    pub rotation: Rotor2,
    pub symetry: Vec2,
}

const PX_PER_SQUARE: f32 = 512.0;

impl TextDrawer {
    pub fn new(
        chars: &[char],
        device: Rc<Device>,
        queue: Rc<Queue>,
        globals_layout: &BindGroupLayout,
    ) -> Self {
        let mut char_drawers = HashMap::new();
        let mut char_map = HashMap::new();
        for c in chars
            .iter()
            .chain(['A', 'a'].iter().filter(|c| !chars.contains(c)))
        {
            char_drawers.insert(
                *c,
                CharDrawer::new(device.clone(), queue.clone(), globals_layout, *c),
            );
            char_map.insert(*c, Vec::new());
        }
        Self {
            char_map,
            char_drawers,
            layout: Layout::new(fontdue::layout::CoordinateSystem::PositiveYDown),
        }
    }

    pub fn clear(&mut self) {
        for v in self.char_map.values_mut() {
            v.clear();
        }
    }

    pub fn draw<'a>(&'a mut self, render_pass: &mut RenderPass<'a>) {
        for (c, v) in self.char_map.iter() {
            if let Some(drawer_mut) = self.char_drawers.get_mut(c) {
                drawer_mut.new_instances(Rc::new(v.clone()))
            } else {
                eprintln!("Unprintable char {}", c);
            }
        }
        for d in self.char_drawers.values_mut() {
            d.draw(render_pass);
        }
    }

    pub fn add_sentence(&mut self, sentence: Sentence<'_>, center_position: Vec2, bound: Line) {
        let fonts = if sentence
            .text
            .chars()
            .next()
            .map(|c| c.is_ascii_uppercase())
            .unwrap_or_default()
        {
            [&self.char_drawers[&'A'].letter.font]
        } else {
            [&self.char_drawers[&'a'].letter.font]
        };
        self.layout
            .reset(&fontdue::layout::LayoutSettings::default());
        self.layout.append(
            &fonts,
            &fontdue::layout::TextStyle::new(sentence.text, PX_PER_SQUARE, 0),
        );
        let rectangle = SentenceRectangle::new(
            self.layout.glyphs(),
            PX_PER_SQUARE / sentence.size,
            sentence.rotation,
            sentence.symetry,
        );
        let shift = rectangle.shift(bound, center_position);

        for g in rectangle.glyphs.iter() {
            let c = g.parent;
            let pos = (Vec2 {
                x: g.x / PX_PER_SQUARE * sentence.size,
                y: g.y / PX_PER_SQUARE * sentence.size,
            } * sentence.symetry)
                .rotated_by(sentence.rotation)
                + shift;
            self.char_map.entry(c).or_default().push(CharInstance {
                top_left: pos,
                rotation: Mat2::identity(),
                z_index: sentence.z_index,
                color: sentence.color,
                size: sentence.size,
            })
        }
    }
}

struct SentenceRectangle<'a> {
    glyphs: &'a Vec<fontdue::layout::GlyphPosition<()>>,
    top: f32,
    bottom: f32,
    size_px: f32,
    rotation: Rotor2,
    symetry: Vec2,
}

impl<'a> SentenceRectangle<'a> {
    fn new(
        glyphs: &'a Vec<fontdue::layout::GlyphPosition<()>>,
        size_px: f32,
        rotation: Rotor2,
        symetry: Vec2,
    ) -> Self {
        let bottom = glyphs
            .iter()
            .map(|g| g.y + g.height as f32)
            .fold(f32::NEG_INFINITY, |x, y| if x > y { x } else { y });
        let top = glyphs
            .iter()
            .map(|g| g.y)
            .fold(f32::INFINITY, |x, y| if x < y { x } else { y });
        Self {
            glyphs,
            top,
            bottom,
            size_px,
            rotation,
            symetry,
        }
    }
    fn left(&self) -> f32 {
        self.glyphs.first().map(|g| g.x).unwrap_or_default()
    }

    fn right(&self) -> f32 {
        self.glyphs
            .last()
            .map(|g| g.x + g.width as f32)
            .unwrap_or_default()
    }

    fn top(&self) -> f32 {
        self.top
    }

    fn bottom(&self) -> f32 {
        self.bottom
    }

    fn center(&self) -> Vec2 {
        (Vec2 {
            x: self.left(),
            y: self.top(),
        } + Vec2 {
            x: self.right(),
            y: self.bottom(),
        }) / 2.
            / self.size_px
    }

    fn corners(&self) -> [Vec2; 4] {
        [
            Vec2::new(self.left(), self.top()) / self.size_px,
            Vec2::new(self.left(), self.bottom()) / self.size_px,
            Vec2::new(self.right(), self.top()) / self.size_px,
            Vec2::new(self.right(), self.bottom()) / self.size_px,
        ]
    }

    fn shift(&self, line: Line, center: Vec2) -> Vec2 {
        let mut ret = Vec2::zero();
        let mut mag = 0.0;

        for c in self.corners().iter() {
            let point_no_shift =
                center + ((*c - self.center()) * self.symetry).rotated_by(self.rotation);
            let shift = line.shift(point_no_shift);
            if shift.mag() > mag {
                mag = shift.mag();
                ret = shift;
            }
        }
        center - (self.center() * self.symetry).rotated_by(self.rotation) + ret
        //center - self.center()
    }
}

/// A 2d line given by an origin and a direction vector.
///
/// The equation of the line is (x - origin.x) * direction.y - (y + origin.y) * direction.x = 0
#[derive(Debug)]
pub struct Line {
    pub origin: Vec2,
    pub direction: Vec2,
}

impl Line {
    fn project_point(&self, point: Vec2) -> Vec2 {
        (point - self.origin).dot(self.direction.normalized()) * self.direction.normalized()
            + self.origin
    }

    fn equation(&self, point: Vec2) -> f32 {
        (point.y - self.origin.y) * self.direction.x - (point.x - self.origin.x) * self.direction.y
    }

    /// Return the smallest translation to be applied to point to put in on the positive side of self
    fn shift(&self, point: Vec2) -> Vec2 {
        if self.equation(point) > 0.0 {
            self.project_point(point) - point
        } else {
            Vec2::zero()
        }
    }
}
