use bytemuck::NoUninit;
use glam::Vec2;
use skrifa::instance::Size;
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::prelude::LocationRef;
use skrifa::{FontRef, MetadataProvider};
use std::convert;

#[repr(u32)]
#[derive(Copy, Clone, Debug, NoUninit)]
pub enum Verb {
    MoveTo = 0,
    LineTo = 1,
    QuadTo = 2,
    Close = 3,
}

pub struct PointVerb {
    pub modifier: Box<dyn FnMut(Vec2) -> Vec2>,

    pub points: Vec<Vec2>,
    pub verbs: Vec<Verb>,
}

impl Default for PointVerb {
    fn default() -> Self {
        Self {
            modifier: Box::new(convert::identity),
            points: Vec::new(),
            verbs: Vec::new(),
        }
    }
}

impl PointVerb {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_point(&mut self, x: f32, y: f32) {
        let vec = Vec2::new(x, y);
        self.points.push((*self.modifier)(vec));
    }

    pub fn set_modifier(&mut self, modifier: impl FnMut(Vec2) -> Vec2 + 'static) {
        self.modifier = Box::new(modifier);
    }
}

impl OutlinePen for PointVerb {
    fn move_to(&mut self, x: f32, y: f32) {
        self.add_point(x, y);
        self.verbs.push(Verb::MoveTo);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.add_point(x, y);
        self.verbs.push(Verb::LineTo);
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.add_point(cx0, cy0);
        self.add_point(x, y);
        self.verbs.push(Verb::QuadTo);
    }

    fn curve_to(&mut self, _: f32, _: f32, _: f32, _: f32, _: f32, _: f32) {
        panic!("Cubic curves are not supported.");
    }

    fn close(&mut self) {
        self.verbs.push(Verb::Close);
    }
}

pub fn main() {
    let data = FontRef::from_index(include_bytes!("../../JetBrainsMono.ttf"), 0).unwrap();
    let glyph_id = data.charmap().map('b').unwrap();
    let outline = data.outline_glyphs().get(glyph_id).unwrap();

    let mut pen = PointVerb::new();
    outline
        .draw(
            DrawSettings::unhinted(Size::unscaled(), LocationRef::default()),
            &mut pen,
        )
        .unwrap();

    println!(
        "{:?}",
        pen.points
            .into_iter()
            .map(|p| (p.x, p.y))
            .collect::<Vec<_>>()
    );
    println!(
        "{:?}",
        pen.verbs
            .into_iter()
            .map(|v| match v {
                Verb::MoveTo => 1,
                Verb::LineTo => 1,
                Verb::QuadTo => 2,
                Verb::Close => 0,
            })
            .collect::<Vec<_>>()
    );
}
