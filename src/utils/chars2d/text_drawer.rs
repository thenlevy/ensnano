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
use fontdue::{layout::Layout, Font};

pub struct TextDrawer {
    char_drawers: HashMap<char, CharDrawer>,
    char_map: HashMap<char, Vec<CharInstance>>,
    layout: Layout<()>,
}

pub struct Sentence<'a> {
    text: &'a str,
    size: f32,
    z_index: i32,
    color: Vec4,
}

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

    pub fn add_sentence(&mut self, sentence: Sentence<'_>, center_position: Vec2, bound: Line) {
        let fonts = [
            &self.char_drawers[&'A'].letter.font,
            &self.char_drawers[&'a'].letter.font,
        ];
        self.layout
            .reset(&fontdue::layout::LayoutSettings::default());
        self.layout.append(
            &fonts,
            &fontdue::layout::TextStyle::new(sentence.text, 30.0, 0),
        );
        let rectangle = SentenceRectangle::new(self.layout.glyphs());
        let shift = rectangle.shift(bound, center_position);

        for g in rectangle.glyphs.iter() {
            let c = g.parent;
            let pos = Vec2 {
                x: (g.x + g.width as f32) / 2.,
                y: (g.y + g.height as f32) / 2.,
            };
            self.char_map.entry(c).or_default().push(CharInstance {
                center: pos,
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
}

impl<'a> SentenceRectangle<'a> {
    fn new(glyphs: &'a Vec<fontdue::layout::GlyphPosition<()>>) -> Self {
        let top = glyphs
            .iter()
            .map(|g| g.y)
            .fold(0.0, |x, y| if x < y { x } else { y });
        let bottom = glyphs
            .iter()
            .map(|g| g.y + g.height as f32)
            .fold(0.0, |x, y| if x > y { x } else { y });
        Self {
            glyphs,
            top,
            bottom,
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
    }

    fn nb_char(&self) -> usize {
        self.glyphs.len()
    }

    fn corners(&self) -> [Vec2; 4] {
        [
            Vec2::new(self.top(), self.left()),
            Vec2::new(self.top(), self.right()),
            Vec2::new(self.bottom(), self.left()),
            Vec2::new(self.bottom(), self.right()),
        ]
    }

    fn shift(&self, line: Line, center: Vec2) -> Vec2 {
        let mut ret = Vec2::zero();
        let mut mag = 0.0;

        for c in self.corners().iter() {
            let shift = line.shift(*c + center - self.center());
            if shift.mag() > mag {
                mag = shift.mag();
                ret = shift;
            }
        }
        center - self.center() + ret
    }
}

pub struct Line {
    origin: Vec2,
    direction: Vec2,
}

impl Line {
    fn ceil(&self) -> f32 {
        self.origin.x * self.direction.y - self.origin.y * self.direction.x
    }

    fn signed_dist(&self, point: Vec2) -> f32 {
        self.origin.x * point.y - self.origin.y * point.x - self.ceil()
    }

    /// Return the smallest translation to be applied to point to put in on the positive side of self
    fn shift(&self, point: Vec2) -> Vec2 {
        let signed_dist = self.signed_dist(point);
        if signed_dist < 0.0 {
            signed_dist
                * Vec2 {
                    x: -self.direction.y,
                    y: self.direction.x,
                }
        } else {
            Vec2::zero()
        }
    }
}
