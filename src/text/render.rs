use crate::lexing::Span;
use crate::parsing::{AddOrSub, AstKind, AstNode};
use crate::utils::OptExt;
use crate::windowing::render::text::GlyphInstance;
use glam::Vec2;

use super::glyph_data::CpuGlyphData;

#[derive(Copy, Clone)]
pub struct AstNodeRenderTransform {
    pub offset: Vec2,
    pub scale: f32,
}

impl Default for AstNodeRenderTransform {
    fn default() -> Self {
        Self {
            offset: Vec2::ZERO,
            scale: 1.0,
        }
    }
}

impl AstNodeRenderTransform {
    pub fn apply_point(&self, to: Vec2) -> Vec2 {
        (to + self.offset) * self.scale
    }

    pub fn apply_glyph(&self, mut to: GlyphInstance) -> GlyphInstance {
        to.size *= self.scale;
        to.position += self.offset * self.scale;
        to
    }
}

#[derive(Debug)]
pub struct RenderNode {
    /// offset, relative to the parent
    offset: Vec2,
    bbox_size: Vec2,
    /// Joining text horizontally gets ugly without some space inbetween.
    /// This stores the space needed to the right of the current elem.
    /// It's kept if layed out horizontally.
    advance: f32,
    kind: RenderNodeKind,
}

#[derive(Debug)]
pub enum RenderNodeKind {
    OneGlyph(GlyphInstance),
    Text(Vec<GlyphInstance>),
    Layout(Vec<RenderNode>),
}

impl<'a> RenderNode {
    pub fn into_instances(self) -> Vec<GlyphInstance> {
        let mut nodes = match self.kind {
            RenderNodeKind::Text(layout) => layout,
            RenderNodeKind::OneGlyph(glyph) => vec![glyph],
            RenderNodeKind::Layout(nodes) => nodes
                .into_iter()
                .map(|node| node.into_instances())
                .reduce(|mut a, b| {
                    a.extend(b);
                    a
                })
                .unwrap_unreach(),
        };

        nodes
            .iter_mut()
            .for_each(|instance| instance.position += self.offset);
        nodes
    }

    pub fn text(glyph_data: &CpuGlyphData, text: &'a str) -> Self {
        let mut layout = Vec::new();
        let mut last_elem = None;

        let mut chars = text.chars().peekable();
        let first_char = *chars.peek().unwrap_unreach();
        let first_info = glyph_data.get_info(first_char).unwrap_unreach();
        let mut font_x_pos = -first_info.bbox.x_min();

        for char in chars {
            let info = glyph_data.get_info(char).unwrap_unreach();
            last_elem = Some((char, info));

            layout.push(GlyphInstance::new(
                Vec2::new(font_x_pos, glyph_data.baseline()),
                Vec2::ONE,
                info.glyph_id,
            ));

            font_x_pos += info.advance;
        }

        let (_, last_info) = last_elem.unwrap_unreach();
        let last_advance = last_info.advance;
        let font_last_x_pos = font_x_pos - last_advance;
        let rightmost = font_last_x_pos + last_info.bbox.x_max();

        Self {
            offset: Vec2::ZERO,
            bbox_size: Vec2::new(rightmost, 1.0),
            kind: RenderNodeKind::Text(layout),
            advance: last_advance - last_info.bbox.x_max(),
        }
    }

    pub fn one_glyph_stretched_y(glyph_data: &CpuGlyphData, goal_height: f32, char: char) -> Self {
        let info = glyph_data.get_info(char).unwrap_unreach();
        let y_stretch = goal_height / info.bbox.size.y;

        Self {
            offset: Vec2::ZERO,
            bbox_size: Vec2::new(info.bbox.size.x, goal_height),
            kind: RenderNodeKind::OneGlyph(GlyphInstance::new(
                Vec2::new(-info.bbox.x_min(), glyph_data.baseline() * y_stretch),
                Vec2::new(1.0, y_stretch),
                info.glyph_id,
            )),
            advance: 0.0,
        }
    }

    pub fn horizontal(nodes: Vec<Self>) -> Self {
        assert!(!nodes.is_empty(), "Horizontal cannot contain no nodes.");

        let mut offset = 0.0;
        let mut nodes = nodes;

        let mut last_advance = 0.0;
        for node in nodes.iter_mut() {
            node.offset = Vec2::new(offset, 0.0);
            offset += node.bbox_size.x + node.advance;
            last_advance = node.advance;
        }

        let max_y = nodes
            .iter()
            .map(|node| node.bbox_size.y)
            .reduce(f32::max)
            .unwrap_unreach();

        Self {
            offset: Vec2::ZERO,
            bbox_size: Vec2::new(offset - last_advance, max_y),
            kind: RenderNodeKind::Layout(nodes),
            advance: last_advance,
        }
    }

    pub fn vertical(nodes: Vec<Self>) -> Self {
        assert!(!nodes.is_empty(), "Vertical cannot contain no nodes.");

        let max_x = nodes
            .iter()
            .map(|node| node.bbox_size.x)
            .reduce(f32::max)
            .unwrap_unreach();

        let mut offset = 0.0;
        let mut nodes = nodes;
        for node in nodes.iter_mut() {
            node.offset = Vec2::new((max_x - node.bbox_size.x) / 2.0, offset);
            offset += node.bbox_size.y;
        }

        Self {
            offset: Vec2::ZERO,
            bbox_size: Vec2::new(max_x, offset),
            kind: RenderNodeKind::Layout(nodes),
            advance: 0.0,
        }
    }
}

pub struct AstNodeRenderContext<'a> {
    glyph_data: &'a CpuGlyphData,
}

impl<'a> AstNodeRenderContext<'a> {
    pub fn new(glyph_data: &'a CpuGlyphData) -> Self {
        Self { glyph_data }
    }

    pub fn render(&mut self, source: &str, node: &AstNode) -> RenderNode {
        match node.kind() {
            AstKind::Number(_) | AstKind::Identifier(_) => {
                RenderNode::text(self.glyph_data, node.span_as_str(source))
            }
            AstKind::Group(node) => {
                let render_node = self.render(source, node);
                let height = render_node.bbox_size.y;
                println!("Height: {height}");
                let left_paren = RenderNode::one_glyph_stretched_y(self.glyph_data, height, '(');
                let right_paren = RenderNode::one_glyph_stretched_y(self.glyph_data, height, ')');

                RenderNode::horizontal(vec![left_paren, render_node, right_paren])
            }
            AstKind::Frac { above, below } => {
                let above = self.render(source, above);
                let below = self.render(source, below);
                RenderNode::vertical(vec![below, above])
            }
            AstKind::LatexGroup(node) => self.render(source, node),
            AstKind::AddSub(items) => {
                let item_count = items.len();
                let mut items = items.iter();
                let (first_add_sub, first_item) = items.next().unwrap_unreach();
                let before_first_item = Span::new(node.span().from, first_item.span().from);

                let mut render_nodes = Vec::with_capacity(item_count * 2);

                match first_add_sub {
                    AddOrSub::Add => match before_first_item.len() {
                        0 => {}
                        1 => {
                            assert_eq!(before_first_item.select(source), "+");
                            render_nodes.push(RenderNode::text(self.glyph_data, "+"));
                        }
                        _ => unreachable!(),
                    },
                    AddOrSub::Sub => {
                        assert_eq!(before_first_item.select(source), "-");
                        render_nodes.push(RenderNode::text(self.glyph_data, "-"));
                    }
                }

                render_nodes.push(self.render(source, first_item));

                for (add_sub, item) in items {
                    let text = match add_sub {
                        AddOrSub::Add => "+",
                        AddOrSub::Sub => "-",
                    };
                    render_nodes.push(RenderNode::text(self.glyph_data, text));
                    render_nodes.push(self.render(source, item));
                }

                RenderNode::horizontal(render_nodes)
            }
            kind => todo!("Implement render node construction for {kind:?}"),
        }
    }
}
