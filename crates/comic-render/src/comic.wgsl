// The comic's pixels on the GPU: one workgroup per 16-pixel tile, one invocation per pixel. The same arithmetic as
// sdf.rs, in the same order: a capsule's and a convex fill's signed distance, a one-pixel coverage ramp, each layer's
// ink over its paper, layer over layer, packed to eight bits a channel by rounding half up.

struct Params {
    width: u32,
    height: u32,
    tiles_x: u32,
    tiles_n: u32,
    layers: u32,
    pad0: u32,
    pad1: u32,
    pad2: u32,
    clear: vec4<f32>,
    paper: vec4<f32>,
    ink: vec4<f32>,
};
struct Capsule { a: vec2<f32>, b: vec2<f32>, r: f32, pad: f32 };
struct Fill { start: u32, count: u32, pad0: u32, pad1: u32 };
struct Edge { p: vec2<f32>, n: vec2<f32> };
struct Tile { capsules: u32, capsule_count: u32, fills: u32, fill_count: u32 };

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> capsules: array<Capsule>;
@group(0) @binding(2) var<storage, read> fills: array<Fill>;
@group(0) @binding(3) var<storage, read> edges: array<Edge>;
@group(0) @binding(4) var<storage, read> tiles: array<Tile>;
@group(0) @binding(5) var<storage, read> items: array<u32>;
@group(0) @binding(6) var<storage, read_write> pixels: array<u32>;

fn sd_capsule(x: f32, y: f32, c: Capsule) -> f32 {
    let pax = x - c.a.x;
    let pay = y - c.a.y;
    let bax = c.b.x - c.a.x;
    let bay = c.b.y - c.a.y;
    let h = clamp((pax * bax + pay * bay) / max(bax * bax + bay * bay, 1e-6), 0.0, 1.0);
    let dx = pax - bax * h;
    let dy = pay - bay * h;
    return sqrt(dx * dx + dy * dy) - c.r;
}

fn sd_convex(x: f32, y: f32, f: Fill) -> f32 {
    var d = -1e9;
    for (var k = 0u; k < f.count; k = k + 1u) {
        let e = edges[f.start + k];
        d = max(d, (x - e.p.x) * e.n.x + (y - e.p.y) * e.n.y);
    }
    return d;
}

fn coverage(sd: f32) -> f32 {
    return clamp(0.5 - sd, 0.0, 1.0);
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) gid: vec3<u32>, @builtin(workgroup_id) wid: vec3<u32>) {
    if (gid.x >= params.width || gid.y >= params.height) {
        return;
    }
    let x = f32(gid.x) + 0.5;
    let y = f32(gid.y) + 0.5;
    let tile = wid.y * params.tiles_x + wid.x;
    var color = params.clear;
    for (var layer = 0u; layer < params.layers; layer = layer + 1u) {
        let t = tiles[layer * params.tiles_n + tile];
        if (t.capsule_count == 0u && t.fill_count == 0u) {
            continue;
        }
        var ink = 0.0;
        for (var i = 0u; i < t.capsule_count; i = i + 1u) {
            ink = max(ink, coverage(sd_capsule(x, y, capsules[items[t.capsules + i]])));
        }
        var fill = 0.0;
        for (var i = 0u; i < t.fill_count; i = i + 1u) {
            fill = max(fill, coverage(sd_convex(x, y, fills[items[t.fills + i]])));
        }
        let a = ink + fill * (1.0 - ink);
        let under = 1.0 - a;
        let rgb = params.ink.rgb * ink + params.paper.rgb * (fill * (1.0 - ink)) + color.rgb * under;
        color = vec4<f32>(rgb, a + color.a * under);
    }
    let q = vec4<u32>(floor(clamp(color, vec4<f32>(0.0), vec4<f32>(1.0)) * 255.0 + 0.5));
    pixels[gid.y * params.width + gid.x] = q.x | (q.y << 8u) | (q.z << 16u) | (q.w << 24u);
}
