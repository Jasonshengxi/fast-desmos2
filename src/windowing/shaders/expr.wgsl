struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) tex_pos: vec2<f32>,
    @interpolate(flat) @location(1) glyph_index: u32,
}

struct Transform {
    scale: vec2<f32>,
    offset: vec2<f32>,
    bbox_expansion: vec2<f32>,
}

struct BoundingBox {
    size: vec2<f32>,
    pos: vec2<f32>,
}

struct InstanceData {
    pos: vec2<f32>,
    size: vec2<f32>,
    index: u32,
}


@group(0) @binding(0)
var<uniform> transform: Transform;

@group(1) @binding(0)
var<storage, read> inst_data: array<InstanceData>;

@group(2) @binding(0)
var<storage, read> bounding_boxes: array<BoundingBox>;

@vertex
fn vs_main(
    @builtin(vertex_index) v_ind: u32,
    @builtin(instance_index) i_ind: u32,
) -> VertexOutput {
    let inst: InstanceData = inst_data[i_ind];
    let bbox = bounding_boxes[inst.index];

    let box_min = bbox.pos             - transform.bbox_expansion;
    let box_max = bbox.pos + bbox.size + transform.bbox_expansion;

    var vo: VertexOutput;
    
    let corner: vec2<f32> = vec2<f32>(
        select(box_min.x, box_max.x, (v_ind & 1) > 0),
        select(box_min.y, box_max.y, (v_ind & 2) > 0),
    );
    let inst_corner: vec2<f32> = corner * inst.size + inst.pos;
    let pos: vec2<f32> = inst_corner * transform.scale + transform.offset;
    
    vo.pos = vec4<f32>(pos, 0.0, 1.0);
    vo.tex_pos = corner;
    vo.glyph_index = inst.index;

    return vo;
}

struct GlyphStarts {
    point_start: u32,
    verb_start: u32,
}

fn is_between(x: f32, min: f32, max: f32) -> bool {
    return (x > min) && (x < max);
}

struct IntersectResult {
    intersects: u32,
    closest: f32,
    row_intersects: u32,
}

fn infinity() -> f32 {
    let a: f32 = 1.0;
    let b: f32 = 0.0;
    return a / b;
}

fn line_sdf(pos: vec2<f32>, a: vec2<f32>, b: vec2<f32>) -> f32 {
    let pa = pos - a;
    let ba = b - a;
    let h = saturate(dot(pa, ba) / dot(ba, ba));
    return distance(pa, ba * h);
}

fn dot2(v: vec2<f32>) -> f32 {
    return dot(v, v);
}

fn quad_sdf(pos: vec2<f32>, A: vec2<f32>, B: vec2<f32>, C: vec2<f32>) -> f32 {
    let a = B - A;
    let b = A - 2.0 * B + C;
    let c = 2.0 * a;
    let d = A - pos;
    
    let kk = 1.0 / dot(b, b);
    let kx = kk * dot(a, b);
    let ky = kk * (2.0 * dot(a, a) + dot(d, b)) / 3.0;
    let kz = kk * dot(d, a);
    
    let p = ky - kx * kx;
    let p3 = p * p * p;
    let q = kx * (2.0 * kx * kx - 3.0 * ky) + kz;
    let h = q * q + 4.0 * p3;
    if (h >= 0.0) {
        let sh = sqrt(h);
        let x = (vec2<f32>(sh, -sh) - q) / 2.0;
        let uv = sign(x) * pow(abs(x), vec2<f32>(1.0 / 3.0));
        let t = saturate(uv.x + uv.y - kx);
        return length(d + (c + b * t) * t);
    } else {
        let z = sqrt(-p);
        let v = acos(q / (p * z * 2.0)) / 3.0;
        let m = cos(v);
        let n = sin(v) * 1.732050808;
        let t = saturate(vec3<f32>(m + m, -n - m, n - m) * z - kx);
        return sqrt(min(
            dot2(d + (c + b * t.x) * t.x),
            dot2(d + (c + b * t.y) * t.y),
        ));
    }
}

fn line_intersects(pos: vec2<f32>, p1: vec2<f32>, p2: vec2<f32>) -> IntersectResult {
	if (p1.y == p2.y) {
		if (pos.y == p1.y) {
			let inter_x = min(p1.x, p2.y);
			if (pos.x < inter_x) {
				return IntersectResult(1u, inter_x, 1u);
			} else {
				return IntersectResult(0u, infinity(), 0u);
			}
		} else {
			return IntersectResult(0u, infinity(), 0u);
		}
	}

    let t = (pos.y - p1.y) / (p2.y - p1.y);
    if (is_between(t, 0.0, 1.0)) {
        let x = p1.x + t * (p2.x - p1.x);
        if (x > pos.x) {
            return IntersectResult(1u, x, 1u);
        } else {
            return IntersectResult(0u, infinity(), 1u);
        }
    } else {
        return IntersectResult(0u, infinity(), 0u);
    }
}

fn quad_bounds_check(pos: vec2<f32>, p1: vec2<f32>, p2: vec2<f32>) -> bool {
    let y_min = min(p1.y, p2.y);
    let y_max = max(p1.y, p2.y);

    if (is_between(pos.y, y_min, y_max)) {
        return true;
    }
    
    let t_y_tip = p1.y / (p1.y + p2.y);

    if (!is_between(t_y_tip, 0.0, 1.0)) {
        return false;
    }

    // since the control point has been offset to 0,
    // f(t) = (1 - t)^2 * p1 + t^2 * p2
    let y_tip = (1.0 - t_y_tip) * (1.0 - t_y_tip) * p1.y + t_y_tip * t_y_tip * p2.y;

    var y_min_bound = y_min;
    var y_max_bound = y_max;
    if (y_tip > y_max_bound) {
        y_max_bound = y_tip;
    } else if (y_tip < y_min_bound) {
        y_min_bound = y_tip;
    } else {
        return false;
    }

    let in_bounds = is_between(pos.y, y_min_bound, y_max_bound);
    return in_bounds;
}

fn quad_intersects(Pos: vec2<f32>, P1: vec2<f32>, cp: vec2<f32>, P2: vec2<f32>) -> IntersectResult {
    let p1 = cp - P1;
    let p2 = cp - P2;
    let pos = cp - Pos;
    
    if (!quad_bounds_check(pos, p1, p2)) {
        return IntersectResult(0u, infinity(), 0u);
    }
    
    let y_sum = p1.y + p2.y;
    
    let discrim = pos.y * y_sum - p1.y * p2.y;
    if (discrim < 0.0) {
        return IntersectResult(0u, infinity(), 0u);
    }
    
    let const_term = p1.y / y_sum;
    let var_term = sqrt(discrim) / y_sum;
    
    let t1 = const_term + var_term;
    let t2 = const_term - var_term;
    
    var result: u32 = 0u;
    var intersect: f32 = infinity();
    var row_intersect: u32 = 0u;
    
    if (is_between(t1, 0.0, 1.0)) {
        let s1 = 1.0 - t1;
        let x = s1 * s1 * P1.x + 2 * t1 * s1 * cp.x + t1 * t1 * P2.x;
        if (x > Pos.x) {
            result += 1u;
            intersect = min(intersect, x);
        }
        row_intersect += 1u;
    }
    if (is_between(t2, 0.0, 1.0)) {
        let s2 = 1.0 - t2;
        let x = s2 * s2 * P1.x + 2 * t2 * s2 * cp.x + t2 * t2 * P2.x;
        
        if (x > Pos.x) {
            result += 1u;
            intersect = min(intersect, x);
        }
        row_intersect += 1u;
    }
    
    return IntersectResult(result, intersect, row_intersect);
}

@group(2) @binding(1)
var<storage, read> glyph_points: array<vec2<f32>>;
@group(2) @binding(2)
var<storage, read> glyph_verbs: array<u32>;
@group(2) @binding(3)
var<storage, read> glyph_starts: array<GlyphStarts>;


@fragment
fn fs_main(vo: VertexOutput) -> @location(0) vec4<f32> {
    let glyph_start = glyph_starts[vo.glyph_index];
    let glyph_end = glyph_starts[vo.glyph_index + 1];
    
    let px_width = dpdxCoarse(vo.tex_pos.x);
    
    let pos = vo.tex_pos;
    
    var total_intersects: u32 = 0u;
    var total_row: u32 = 0u;
    var min_dist: f32 = infinity();
//    var min_intersect: f32 = infinity();
    var last_close: u32 = glyph_start.point_start;
    var point_index: u32 = glyph_start.point_start;
    for (var verb_index: u32 = glyph_start.verb_start; verb_index < glyph_end.verb_start; verb_index++) {
        let verb = glyph_verbs[verb_index];
        switch (verb) {
            case 0u: {
                // MoveTo
                point_index += 1u;
            }
            case 1u: {
                // LineTo
                let start = glyph_points[point_index - 1u];
                let end   = glyph_points[point_index];
                
                let result = line_intersects(pos, start, end);
                total_intersects += result.intersects;
                total_row += result.row_intersects;
//                min_intersect = min(min_intersect, result.closest);
                min_dist = min(min_dist, line_sdf(pos, start, end));
                
                point_index += 1u;
            }
            case 2u: {
                // QuadTo
                let start   = glyph_points[point_index - 1u];
                let control = glyph_points[point_index];
                let end     = glyph_points[point_index + 1u];
                
                let result = quad_intersects(pos, start, control, end);
                total_intersects += result.intersects;
                total_row += result.row_intersects;
//                min_intersect = min(min_intersect, result.closest);
                min_dist = min(min_dist, quad_sdf(pos, start, control, end));
                
                point_index += 2u;
            }
            case 3u: {
                // Close
                let start = glyph_points[point_index - 1];
                let end   = glyph_points[last_close];
                last_close = point_index;

                let result = line_intersects(pos, start, end);
                total_intersects += result.intersects;
                total_row += result.row_intersects;
//                min_intersect = min(min_intersect, result.closest);
                min_dist = min(min_dist, line_sdf(pos, start, end));

//                point_index += 1u;
            }
            default: {
                return vec4<f32>(1.0, 0.0, 0.0, 0.5);
            }
        }
    }
    
    let had_error = (total_row % 2) > 0;
    let raw_inside = (total_intersects % 2) > 0;
    let inside = select(raw_inside, !raw_inside, had_error);
    
    let signed_dist = select(min_dist, -min_dist, inside);
    let pixel_dist = signed_dist / px_width;

    let c = saturate(-pixel_dist / 2.0 + 0.5);
    if (c == 0.0) {
        discard;
    }
    
    let color_on = select(
        vec4<f32>(1.0, 1.0, 1.0, 0.5), 
        vec4<f32>(1.0, 0.0, 0.0, 0.5), 
        had_error
    );
    let color_off = vec4<f32>(0.0, 0.0, 0.0, 0.5);
    
    return (1.0 - c) * color_off + c * color_on;
}
