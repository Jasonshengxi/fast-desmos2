use super::fonts::PointVerb;
use crate::utils::OptExt;
use crate::windowing::render::text::GlyphInstance;
use crate::windowing::Devices;
use bytemuck::{Pod, Zeroable};
use glam::Vec2;
use skrifa::metrics::{GlyphMetrics, Metrics};
use skrifa::outline::DrawSettings;
use skrifa::prelude::{LocationRef, Size};
use skrifa::{FontRef, MetadataProvider};
use std::array;
use std::collections::HashMap;
use wgpu::*;

fn make_bind_group_layout<const BIND_COUNT: usize>(
    devices: &Devices,
    label: &'static str,
    visibilities: [ShaderStages; BIND_COUNT],
) -> BindGroupLayout {
    let entries: [_; BIND_COUNT] = array::from_fn(|index| BindGroupLayoutEntry {
        binding: index as u32,
        count: None,
        visibility: visibilities[index],
        ty: BindingType::Buffer {
            ty: BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
    });

    devices
        .device
        .create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some(label),
            entries: &entries,
        })
}

pub fn new(devices: &Devices, data: &[u8]) -> (GpuGlyphData, CpuGlyphData) {
    let font = FontRef::from_index(data, 0).unwrap();
    let outlines = font.outline_glyphs();
    let glyph_metrics = GlyphMetrics::new(&font, Size::unscaled(), LocationRef::default());
    let metrics = Metrics::new(&font, Size::unscaled(), LocationRef::default());

    let glyph_height = metrics.ascent - metrics.descent;
    let scale = glyph_height.recip();

    let mut point_verb = PointVerb::new();
    let mut glyph_starts = Vec::new();
    let mut bounds = Vec::new();
    let mut glyph_info = HashMap::new();

    for (index, (char_id, glyph_id)) in font.charmap().mappings().enumerate() {
        glyph_starts.push(GlyphStarts {
            point_start: point_verb.points.len() as u32,
            verb_start: point_verb.verbs.len() as u32,
        });
        let outline = outlines.get(glyph_id).unwrap_unreach();
        let bbox = glyph_metrics.bounds(glyph_id).unwrap_unreach();
        let advance = glyph_metrics.advance_width(glyph_id).unwrap_unreach();
        let settings = DrawSettings::unhinted(Size::unscaled(), LocationRef::default());

        bounds.push(BoundingBox::from(bbox.scale(scale)));
        point_verb.set_modifier(move |x| x * scale);
        outline.draw(settings, &mut point_verb).unwrap_unreach();

        // println!("char: {:?}", char::from_u32(char_id).unwrap(),);
        // println!("advance: {advance:?}");
        // println!("left bearing: {bearing:?}");
        // println!("bbox: {bbox:?}");

        glyph_info.insert(
            char::from_u32(char_id).unwrap_unreach(),
            GlyphInfo {
                glyph_id: index as u32,
                advance: advance * scale,
                bbox: bbox.scale(scale).into(),
            },
        );
    }

    // not a real glyph, but useful
    glyph_starts.push(GlyphStarts {
        point_start: point_verb.points.len() as u32,
        verb_start: point_verb.verbs.len() as u32,
    });

    let points_buffer = devices.make_storage_buffer("Glyph Points", &point_verb.points);
    let verbs_buffer = devices.make_storage_buffer("Glyph Verbs", &point_verb.verbs);
    let starts_buffer = devices.make_storage_buffer("Glyph Starts", &glyph_starts);
    let widths_buffer = devices.make_storage_buffer("Glyph Widths", &bounds);

    let layout = make_bind_group_layout(
        devices,
        "Glyph layout",
        [
            ShaderStages::VERTEX,
            ShaderStages::FRAGMENT,
            ShaderStages::FRAGMENT,
            ShaderStages::FRAGMENT,
        ],
    );

    let bind_group = devices.device.create_bind_group(&BindGroupDescriptor {
        label: Some("Glyph BG"),
        layout: &layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: widths_buffer.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 1,
                resource: points_buffer.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 2,
                resource: verbs_buffer.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 3,
                resource: starts_buffer.as_entire_binding(),
            },
        ],
    });

    (
        GpuGlyphData {
            points_buffer,
            verbs_buffer,
            starts_buffer,
            bounds_buffer: widths_buffer,
            bind_group,
            layout,
        },
        CpuGlyphData {
            glyph_info,
            baseline: -metrics.descent * scale,
        },
    )
}

pub struct GlyphInfo {
    pub glyph_id: u32,
    pub advance: f32,
    pub bbox: BoundingBox,
}

pub struct CpuGlyphData {
    baseline: f32,
    glyph_info: HashMap<char, GlyphInfo>,
}

pub struct GpuGlyphData {
    points_buffer: Buffer,
    verbs_buffer: Buffer,
    starts_buffer: Buffer,

    bounds_buffer: Buffer,

    layout: BindGroupLayout,
    bind_group: BindGroup,
}

#[repr(C)]
#[derive(Copy, Clone, Zeroable, Pod)]
pub struct BoundingBox {
    pub size: Vec2,
    pub offset: Vec2,
}

impl BoundingBox {
    pub const ZERO: Self = Self {
        size: Vec2::ZERO,
        offset: Vec2::ZERO,
    };

    pub fn x_min(&self) -> f32 {
        self.offset.x
    }

    pub fn y_min(&self) -> f32 {
        self.offset.y
    }

    pub fn x_max(&self) -> f32 {
        self.x_min() + self.size.x
    }

    pub fn y_max(&self) -> f32 {
        self.y_min() + self.size.y
    }

    pub fn is_zero(&self) -> bool {
        self.size == Vec2::ZERO
    }

    pub fn union(self, other: Self) -> Self {
        if self.is_zero() {
            other
        } else if other.is_zero() {
            self
        } else {
            let min = self.offset.min(other.offset);
            let max = (self.offset + other.size).max(self.offset + other.size);
            Self {
                offset: min,
                size: max - min,
            }
        }
    }
}

impl From<skrifa::metrics::BoundingBox> for BoundingBox {
    fn from(value: skrifa::metrics::BoundingBox) -> Self {
        let min = Vec2::new(value.x_min, value.y_min);
        let max = Vec2::new(value.x_max, value.y_max);
        Self {
            offset: min,
            size: max - min,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Zeroable, Pod)]
struct GlyphStarts {
    point_start: u32,
    verb_start: u32,
}

impl CpuGlyphData {
    pub fn baseline(&self) -> f32 {
        self.baseline
    }

    pub fn get_info(&self, char: char) -> Option<&GlyphInfo> {
        self.glyph_info.get(&char)
    }

    pub fn get_advance(&self, char: char) -> Option<f32> {
        self.get_info(char).map(|x| x.advance)
    }

    pub fn get_bearing(&self, char: char) -> Option<f32> {
        self.get_info(char).map(|x| x.bbox.offset.x)
    }

    pub fn get_id(&self, char: char) -> Option<u32> {
        self.get_info(char).map(|x| x.glyph_id)
    }

    pub fn layout<I: IntoIterator<Item = char>>(
        &self,
        text: I,
        size: f32,
        pos: Vec2,
    ) -> LayoutIter<I::IntoIter> {
        LayoutIter {
            glyph: self,
            chars: text.into_iter(),
            size: Vec2::splat(size),
            pos,
        }
    }
}

pub struct LayoutIter<'a, T> {
    glyph: &'a CpuGlyphData,
    chars: T,
    size: Vec2,
    pos: Vec2,
}

impl<'a, T: Iterator<Item = char>> Iterator for LayoutIter<'a, T> {
    type Item = GlyphInstance;

    fn next(&mut self) -> Option<Self::Item> {
        let char = self.chars.next()?;
        let char_info = self.glyph.get_info(char).unwrap();

        let result = GlyphInstance::new(self.pos, self.size, char_info.glyph_id);
        self.pos.x += char_info.advance * self.size.x;
        Some(result)
    }
}

impl GpuGlyphData {
    pub fn bind_group(&self) -> &BindGroup {
        &self.bind_group
    }
    pub fn layout(&self) -> &BindGroupLayout {
        &self.layout
    }
}
