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
    pub text: &'a str,
    pub size: f32,
    pub z_index: i32,
    pub color: Vec4,
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
            self.char_drawers
                .get_mut(c)
                .unwrap()
                .new_instances(Rc::new(v.clone()))
        }
        for d in self.char_drawers.values_mut() {
            d.draw(render_pass);
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
            &fontdue::layout::TextStyle::new(sentence.text, PX_PER_SQUARE, 0),
        );
        let rectangle = SentenceRectangle::new(
            self.layout.glyphs(),
            PX_PER_SQUARE / sentence.size,
            &self.char_drawers,
        );
        let shift = rectangle.shift(bound, center_position);

        println!("Start sentence");
        for g in rectangle.glyphs.iter() {
            println!(
                "{}, x {}, y {}, width {}, height {}",
                g.parent, g.x, g.y, g.width, g.height
            );
            println!("bottom {}", rectangle.bottom());
            let c = g.parent;
            let pos = Vec2 {
                x: g.x / PX_PER_SQUARE * sentence.size,
                y: g.y / PX_PER_SQUARE * sentence.size,
            } + shift;
            let true_witdh = self.char_drawers[&c].letter.width;
            let advance = self.char_drawers[&c].letter.advance;
            let height = self.char_drawers[&c].letter.height;
            println!(
                "true {}, glyph {}, advance {} ",
                true_witdh,
                g.width as f32 / PX_PER_SQUARE,
                advance
            );
            println!("pos {:?}", pos);
            println!(
                "glyph center: {}, {}",
                (g.x / PX_PER_SQUARE + true_witdh / 2.) * sentence.size + shift.x,
                (g.y / PX_PER_SQUARE + height / 2.) * sentence.size + shift.y
            );
            println!("desired: {:?}", center_position);
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
    right: f32,
    size_px: f32,
}

impl<'a> SentenceRectangle<'a> {
    fn new(
        glyphs: &'a Vec<fontdue::layout::GlyphPosition<()>>,
        size_px: f32,
        char_drawers: &HashMap<char, CharDrawer>,
    ) -> Self {
        let bottom = glyphs
            .iter()
            .flat_map(|g| {
                char_drawers
                    .get(&g.parent)
                    .map(|c| g.y + c.letter.height * PX_PER_SQUARE)
            })
            .fold(f32::NEG_INFINITY, |x, y| if x > y { x } else { y });
        let top = glyphs
            .iter()
            .map(|g| g.y)
            .fold(f32::INFINITY, |x, y| if x < y { x } else { y });
        let right = glyphs
            .last()
            .and_then(|g| {
                char_drawers
                    .get(&g.parent)
                    .map(|c| g.x + c.letter.width * PX_PER_SQUARE)
            })
            .unwrap_or_default();
        Self {
            glyphs,
            top,
            bottom,
            right,
            size_px,
        }
    }
    fn left(&self) -> f32 {
        self.glyphs.first().map(|g| g.x).unwrap_or_default()
    }

    fn right(&self) -> f32 {
        self.right
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
        //center - self.center() + ret
        center - self.center()
    }
}

pub struct Line {
    pub origin: Vec2,
    pub direction: Vec2,
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
