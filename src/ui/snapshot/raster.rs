//! A software stand-in for `egui_glow`'s painter: the triangles egui
//! tessellates, coloured the way the GPU colours them, into a buffer of
//! pixels that can be written out as a picture.
//!
//! Every rule here is one `egui_glow` 0.28 follows, copied because a picture
//! that differs from the screen by a rounding sends whoever compares it after
//! a difference that is not there:
//!
//! - Vertex colours are premultiplied sRGB, interpolated across the triangle
//!   in gamma space: the shader takes them as they come, divided by 255.
//! - Textures are uploaded as `SRGB8_ALPHA8`, so the hardware filters a sample
//!   in linear light, and the shader converts it back to gamma before it
//!   multiplies it by the vertex colour.
//! - That product is blended `ONE, ONE_MINUS_SRC_ALPHA` (and
//!   `ONE_MINUS_DST_ALPHA, ONE` for alpha) into an 8-bit framebuffer that is
//!   not sRGB, so blending happens in gamma space too.
//! - A pixel is covered when its centre is inside the triangle, and a centre
//!   lying exactly on an edge belongs to the triangle that edge is the top or
//!   the left of. That rule is what lets two triangles sharing an edge cover
//!   each pixel once - and egui's feathered meshes are nothing but shared
//!   edges, so without it every seam would be blended twice and show as a
//!   line.
//! - The clip rectangle is `egui_glow`'s scissor: each side scaled to
//!   physical pixels, rounded, and clamped to the framebuffer.

use std::collections::HashMap;
use std::sync::OnceLock;

use egui::epaint::{ImageDelta, Primitive, Vertex};
use egui::{
    ClippedPrimitive, ImageData, Mesh, Rect, TextureFilter, TextureId, TextureOptions,
    TextureWrapMode, TexturesDelta,
};

/// Steps per pixel a vertex is snapped to before the coverage test: the eight
/// bits of sub-pixel precision GPUs rasterise with. Past this point the test
/// is exact integer arithmetic, which is what makes the top-left rule hold -
/// done in floating point, the same pixel centre can come out on both sides of
/// an edge two triangles share.
const SUBPIXEL: i64 = 256;
/// A pixel's centre, in the same steps.
const HALF: i64 = SUBPIXEL / 2;

/// One texture, as `egui_glow` holds it.
struct Texture {
    width: usize,
    height: usize,
    /// Premultiplied sRGBA: the very bytes `egui_glow` uploads.
    texels: Vec<[u8; 4]>,
    options: TextureOptions,
}

/// Every texture egui has handed over, kept up to date from each frame's
/// `TexturesDelta`.
///
/// One store for every window, as `egui_glow` keeps one painter for all of
/// them: the font atlas the Settings window draws with is the one the main
/// window filled in, and a delta taken by one window's frame is part of the
/// other's.
#[derive(Default)]
pub(crate) struct Textures {
    textures: HashMap<TextureId, Texture>,
}

impl Textures {
    /// Takes in what a frame created or changed. Called before that frame is
    /// painted.
    pub(crate) fn set(&mut self, delta: &TexturesDelta) {
        for (id, image) in &delta.set {
            self.set_one(*id, image);
        }
    }

    /// Lets go of what a frame freed. Called after that frame is painted, as
    /// `egui_glow` does: the frame that frees a texture may still draw with it.
    pub(crate) fn free(&mut self, delta: &TexturesDelta) {
        for id in &delta.free {
            self.textures.remove(id);
        }
    }

    fn set_one(&mut self, id: TextureId, delta: &ImageDelta) {
        let ([width, height], texels): ([usize; 2], Vec<[u8; 4]>) = match &delta.image {
            ImageData::Color(image) => (
                image.size,
                image.pixels.iter().map(|c| c.to_array()).collect(),
            ),
            // The atlas holds coverage, which `egui_glow` turns into white of
            // that coverage - through egui's own gamma curve - on the way up.
            ImageData::Font(image) => (
                image.size,
                image.srgba_pixels(None).map(|c| c.to_array()).collect(),
            ),
        };
        let Some([x, y]) = delta.pos else {
            let texture = Texture {
                width,
                height,
                texels,
                options: delta.options,
            };
            self.textures.insert(id, texture);
            return;
        };
        // A patch of a texture that was never created is an error the driver
        // shrugs off, and so is it here.
        let Some(texture) = self.textures.get_mut(&id) else {
            return;
        };
        texture.options = delta.options;
        for (row, patch) in texels.chunks_exact(width.max(1)).enumerate() {
            let to_y = y + row;
            if to_y >= texture.height {
                break;
            }
            let fits = patch.len().min(texture.width.saturating_sub(x));
            if fits == 0 {
                continue;
            }
            let start = to_y * texture.width + x;
            texture.texels[start..start + fits].copy_from_slice(&patch[..fits]);
        }
    }

    fn get(&self, id: TextureId) -> Option<&Texture> {
        self.textures.get(&id)
    }
}

/// What the hardware reads each value of an `SRGB8_ALPHA8` colour channel as:
/// linear light.
fn linear_from_srgb() -> &'static [f32; 256] {
    static TABLE: OnceLock<[f32; 256]> = OnceLock::new();
    TABLE.get_or_init(|| {
        std::array::from_fn(|value| {
            let c = value as f32 / 255.0;
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        })
    })
}

/// The shader's `srgb_gamma_from_linear`, word for word.
fn gamma_from_linear(value: f32) -> f32 {
    if value < 0.003_130_8 {
        value * 12.92
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    }
}

/// Where texel `index` of a row or column `size` long is read from, once the
/// wrap mode has had its say.
fn wrap(index: i64, size: usize, mode: TextureWrapMode) -> usize {
    let size = size as i64;
    let index = match mode {
        TextureWrapMode::ClampToEdge => index.clamp(0, size - 1),
        TextureWrapMode::Repeat => index.rem_euclid(size),
        TextureWrapMode::MirroredRepeat => {
            let at = index.rem_euclid(2 * size);
            if at < size {
                at
            } else {
                2 * size - 1 - at
            }
        }
    };
    index as usize
}

impl Texture {
    fn texel(&self, x: i64, y: i64) -> [u8; 4] {
        let x = wrap(x, self.width, self.options.wrap_mode);
        let y = wrap(y, self.height, self.options.wrap_mode);
        self.texels[y * self.width + x]
    }

    /// What `texture2D` returns at `uv`, already converted back to gamma the
    /// way the shader converts it: RGB through linear light and back, alpha
    /// untouched, since an `SRGB8_ALPHA8` texture's alpha is linear.
    fn sample(&self, uv: [f32; 2], filter: TextureFilter) -> [f32; 4] {
        let linear = linear_from_srgb();
        let (width, height) = (self.width as f32, self.height as f32);
        let mut sum = [0.0_f32; 4];
        let mut add = |texel: [u8; 4], weight: f32| {
            for (channel, value) in sum.iter_mut().zip(&texel[..3]) {
                *channel += weight * linear[usize::from(*value)];
            }
            sum[3] += weight * f32::from(texel[3]) / 255.0;
        };
        match filter {
            TextureFilter::Nearest => {
                let texel = self.texel(
                    (uv[0] * width).floor() as i64,
                    (uv[1] * height).floor() as i64,
                );
                add(texel, 1.0);
            }
            TextureFilter::Linear => {
                // Texel centres sit on the halves, so a coordinate on one
                // reads that texel alone.
                let x = uv[0] * width - 0.5;
                let y = uv[1] * height - 0.5;
                let (left, top) = (x.floor(), y.floor());
                let (fx, fy) = (x - left, y - top);
                let (left, top) = (left as i64, top as i64);
                for (dx, dy, weight) in [
                    (0, 0, (1.0 - fx) * (1.0 - fy)),
                    (1, 0, fx * (1.0 - fy)),
                    (0, 1, (1.0 - fx) * fy),
                    (1, 1, fx * fy),
                ] {
                    if weight > 0.0 {
                        add(self.texel(left + dx, top + dy), weight);
                    }
                }
            }
        }
        [
            gamma_from_linear(sum[0]),
            gamma_from_linear(sum[1]),
            gamma_from_linear(sum[2]),
            sum[3],
        ]
    }
}

/// The pixels `egui_glow` lets a primitive touch: `[x0, x1)` by `[y0, y1)`.
#[derive(Clone, Copy)]
struct Scissor {
    x0: i64,
    y0: i64,
    x1: i64,
    y1: i64,
}

/// A triangle's corner, in sub-pixel steps, with what is interpolated from it.
#[derive(Clone, Copy)]
struct Corner {
    x: i64,
    y: i64,
    /// Premultiplied, gamma-space, 0 to 1: the shader's `v_rgba_in_gamma`.
    colour: [f32; 4],
    uv: [f32; 2],
}

impl Corner {
    fn of(vertex: &Vertex, pixels_per_point: f32) -> Self {
        let snap = |v: f32| (f64::from(v) * f64::from(pixels_per_point) * SUBPIXEL as f64).round();
        Corner {
            x: snap(vertex.pos.x) as i64,
            y: snap(vertex.pos.y) as i64,
            colour: vertex.color.to_array().map(|c| f32::from(c) / 255.0),
            uv: [vertex.uv.x, vertex.uv.y],
        }
    }
}

/// One edge of a triangle wound so that its inside is where every edge
/// function is positive.
#[derive(Clone, Copy)]
struct Edge {
    from_x: i64,
    from_y: i64,
    dx: i64,
    dy: i64,
    /// What the edge function must reach for a pixel centre to be inside: 0
    /// on a top or a left edge, where a centre exactly on the edge counts, and
    /// 1 on any other, where it does not.
    bias: i64,
}

impl Edge {
    fn new(from: &Corner, to: &Corner) -> Self {
        let (dx, dy) = (to.x - from.x, to.y - from.y);
        // With y growing downwards and this winding, a left edge is walked
        // upwards and a top edge, flat, from left to right.
        let top_left = dy < 0 || (dy == 0 && dx > 0);
        Edge {
            from_x: from.x,
            from_y: from.y,
            dx,
            dy,
            bias: if top_left { 0 } else { 1 },
        }
    }

    /// Twice the area of the triangle this edge makes with `(x, y)`: positive
    /// on the inside, and the barycentric weight of the corner opposite the
    /// edge once divided by the whole triangle's.
    fn at(&self, x: i64, y: i64) -> i64 {
        self.dx * (y - self.from_y) - self.dy * (x - self.from_x)
    }

    /// The columns on the pixel row whose centre is at `y` that lie on the
    /// inside of this edge, as an inclusive range that may be empty. Solved
    /// exactly rather than tested pixel by pixel, so a thin diagonal costs
    /// what it covers and not its bounding box.
    fn span(&self, y: i64) -> (i64, i64) {
        // at(x) = k - dy * x, and pixel column c has its centre at
        // x = (2c + 1) * HALF.
        let k = self.dx * (y - self.from_y) + self.dy * self.from_x;
        match self.dy.signum() {
            0 if k >= self.bias => (i64::MIN, i64::MAX),
            0 => (1, 0),
            1 => {
                let odd = (k - self.bias).div_euclid(self.dy * HALF);
                (i64::MIN, (odd - 1).div_euclid(2))
            }
            _ => {
                let odd = ceil_div(self.bias - k, -self.dy * HALF);
                (ceil_div(odd - 1, 2), i64::MAX)
            }
        }
    }
}

/// `a / b` rounded up, for a positive `b`.
fn ceil_div(a: i64, b: i64) -> i64 {
    -(-a).div_euclid(b)
}

/// A framebuffer to paint egui's primitives into.
pub(crate) struct Canvas {
    width: usize,
    height: usize,
    pixels: Vec<[u8; 4]>,
}

/// A colour channel as an 8-bit framebuffer stores it.
fn unorm8(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

impl Canvas {
    /// A framebuffer `width` by `height` physical pixels, cleared to `clear` -
    /// gamma-space and premultiplied, as eframe hands its clear colour to
    /// `glClearColor`.
    pub(crate) fn new(width: usize, height: usize, clear: [f32; 4]) -> Self {
        Canvas {
            width,
            height,
            pixels: vec![clear.map(unorm8); width * height],
        }
    }

    /// Paints one frame's tessellated primitives, in order. Returns what could
    /// not be painted - a mesh whose texture never arrived, a paint callback -
    /// so a picture with a hole in it says why.
    pub(crate) fn paint(
        &mut self,
        primitives: &[ClippedPrimitive],
        pixels_per_point: f32,
        textures: &Textures,
    ) -> Vec<String> {
        let mut missed = Vec::new();
        for primitive in primitives {
            let scissor = self.scissor(primitive.clip_rect, pixels_per_point);
            if scissor.x0 >= scissor.x1 || scissor.y0 >= scissor.y1 {
                continue;
            }
            match &primitive.primitive {
                Primitive::Mesh(mesh) => match textures.get(mesh.texture_id) {
                    Some(texture) => self.mesh(mesh, texture, pixels_per_point, scissor),
                    None => missed.push(format!("texture {:?}", mesh.texture_id)),
                },
                Primitive::Callback(_) => missed.push("a paint callback".to_owned()),
            }
        }
        missed
    }

    /// The picture, opaque. A window without transparency is shown by its
    /// colour alone, whatever alpha the blending left behind.
    pub(crate) fn into_image(self) -> image::RgbaImage {
        let mut bytes = Vec::with_capacity(self.pixels.len() * 4);
        for [r, g, b, _] in self.pixels {
            bytes.extend_from_slice(&[r, g, b, 255]);
        }
        image::RgbaImage::from_raw(self.width as u32, self.height as u32, bytes)
            .expect("a buffer of exactly width * height pixels")
    }

    /// `egui_glow`'s `set_clip_rect`.
    fn scissor(&self, clip: Rect, pixels_per_point: f32) -> Scissor {
        let (width, height) = (self.width as i64, self.height as i64);
        // `as` saturates, so an unbounded clip rectangle comes out as the
        // whole framebuffer, as it does through `egui_glow`'s `as i32`.
        let px = |v: f32| (pixels_per_point * v).round() as i64;
        let x0 = px(clip.min.x).clamp(0, width);
        let y0 = px(clip.min.y).clamp(0, height);
        Scissor {
            x0,
            y0,
            x1: px(clip.max.x).clamp(x0, width),
            y1: px(clip.max.y).clamp(y0, height),
        }
    }

    fn mesh(&mut self, mesh: &Mesh, texture: &Texture, pixels_per_point: f32, scissor: Scissor) {
        for triangle in mesh.indices.as_chunks::<3>().0 {
            let corner = |i: u32| Corner::of(&mesh.vertices[i as usize], pixels_per_point);
            self.triangle(
                [
                    corner(triangle[0]),
                    corner(triangle[1]),
                    corner(triangle[2]),
                ],
                texture,
                scissor,
            );
        }
    }

    fn triangle(&mut self, [a, mut b, mut c]: [Corner; 3], texture: &Texture, clip: Scissor) {
        let mut area = (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
        if area == 0 {
            return;
        }
        // egui winds its triangles both ways; turned round here so the inside
        // is always where the edge functions are positive.
        if area < 0 {
            std::mem::swap(&mut b, &mut c);
            area = -area;
        }
        let low_x = a.x.min(b.x).min(c.x);
        let high_x = a.x.max(b.x).max(c.x);
        let low_y = a.y.min(b.y).min(c.y);
        let high_y = a.y.max(b.y).max(c.y);
        // The pixels whose centres could be inside at all, within the scissor.
        let first_col = ceil_div(low_x - HALF, SUBPIXEL).max(clip.x0);
        let last_col = (high_x - HALF).div_euclid(SUBPIXEL).min(clip.x1 - 1);
        let first_row = ceil_div(low_y - HALF, SUBPIXEL).max(clip.y0);
        let last_row = (high_y - HALF).div_euclid(SUBPIXEL).min(clip.y1 - 1);
        if first_col > last_col || first_row > last_row {
            return;
        }

        // Each edge weighs the corner opposite it.
        let edges = [Edge::new(&b, &c), Edge::new(&c, &a), Edge::new(&a, &b)];
        let corners = [a, b, c];
        let filter = filter_for(texture, &corners, area);
        // Most of what egui draws is flat colour on the atlas's one white
        // texel: read once, not once a pixel.
        let constant = (a.uv == b.uv && b.uv == c.uv).then(|| texture.sample(a.uv, filter));
        let whole = area as f64;

        for row in first_row..=last_row {
            let y = row * SUBPIXEL + HALF;
            let (mut from, mut to) = (first_col, last_col);
            for edge in &edges {
                let (low, high) = edge.span(y);
                from = from.max(low);
                to = to.min(high);
            }
            for col in from..=to {
                let x = col * SUBPIXEL + HALF;
                let weights = edges.map(|edge| (edge.at(x, y) as f64 / whole) as f32);
                let mut colour = [0.0_f32; 4];
                let mut uv = [0.0_f32; 2];
                for (corner, weight) in corners.iter().zip(weights) {
                    for (out, value) in colour.iter_mut().zip(corner.colour) {
                        *out += weight * value;
                    }
                    uv[0] += weight * corner.uv[0];
                    uv[1] += weight * corner.uv[1];
                }
                let texel = constant.unwrap_or_else(|| texture.sample(uv, filter));
                let fragment = std::array::from_fn(|i| colour[i] * texel[i]);
                self.blend(col as usize, row as usize, fragment);
            }
        }
    }

    /// `ONE, ONE_MINUS_SRC_ALPHA` for colour and `ONE_MINUS_DST_ALPHA, ONE`
    /// for alpha, into eight bits a channel.
    fn blend(&mut self, x: usize, y: usize, source: [f32; 4]) {
        let pixel = &mut self.pixels[y * self.width + x];
        let target = pixel.map(|c| f32::from(c) / 255.0);
        let keep = 1.0 - source[3];
        *pixel = [
            unorm8(source[0] + target[0] * keep),
            unorm8(source[1] + target[1] * keep),
            unorm8(source[2] + target[2] * keep),
            unorm8(source[3] * (1.0 - target[3]) + target[3]),
        ];
    }
}

/// Which of the texture's two filters the hardware would use across this
/// triangle: magnification where a texel covers at least a pixel, and
/// minification where it covers less. The texture coordinates are affine
/// across a triangle, so the answer is the same at every pixel of it.
fn filter_for(texture: &Texture, corners: &[Corner; 3], area: i64) -> TextureFilter {
    let options = texture.options;
    if options.magnification == options.minification {
        return options.magnification;
    }
    let [a, b, c] = corners;
    // How far each corner's weight moves for one pixel across and one down.
    let per_pixel = SUBPIXEL as f32 / area as f32;
    let across = [
        -((c.y - b.y) as f32),
        -((a.y - c.y) as f32),
        -((b.y - a.y) as f32),
    ];
    let down = [(c.x - b.x) as f32, (a.x - c.x) as f32, (b.x - a.x) as f32];
    let derivative = |step: [f32; 3], axis: usize, size: usize| {
        corners
            .iter()
            .zip(step)
            .map(|(corner, s)| corner.uv[axis] * s * per_pixel)
            .sum::<f32>()
            * size as f32
    };
    let along = |step| {
        egui::vec2(
            derivative(step, 0, texture.width),
            derivative(step, 1, texture.height),
        )
        .length()
    };
    if along(across).max(along(down)) <= 1.0 {
        options.magnification
    } else {
        options.minification
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{pos2, Color32, ColorImage};

    /// Textures holding the atlas's white texel, the way the font texture
    /// starts out, so a flat colour can be painted.
    fn white() -> Textures {
        let mut textures = Textures::default();
        let image = ColorImage::new([4, 4], Color32::WHITE);
        let mut delta = TexturesDelta::default();
        delta.set.push((
            TextureId::default(),
            ImageDelta::full(image, TextureOptions::LINEAR),
        ));
        textures.set(&delta);
        textures
    }

    fn quad(min: egui::Pos2, max: egui::Pos2, colour: Color32) -> ClippedPrimitive {
        let mut mesh = Mesh::default();
        mesh.add_colored_rect(Rect::from_min_max(min, max), colour);
        ClippedPrimitive {
            clip_rect: Rect::EVERYTHING,
            primitive: Primitive::Mesh(mesh),
        }
    }

    fn pixel(canvas: &Canvas, x: usize, y: usize) -> [u8; 4] {
        canvas.pixels[y * canvas.width + x]
    }

    /// Two triangles sharing a diagonal: every pixel of the square is painted
    /// once, so a half-transparent colour comes out the same everywhere -
    /// painted twice along the seam, the diagonal would show as a darker line.
    #[test]
    fn a_rectangle_split_into_two_triangles_covers_every_pixel_exactly_once() {
        let mut canvas = Canvas::new(8, 8, [0.0, 0.0, 0.0, 1.0]);
        let half = Color32::from_rgba_premultiplied(100, 0, 0, 128);
        let missed = canvas.paint(&[quad(pos2(1.0, 1.0), pos2(7.0, 7.0), half)], 1.0, &white());
        assert!(missed.is_empty());
        for y in 0..8 {
            for x in 0..8 {
                let inside = (1..7).contains(&x) && (1..7).contains(&y);
                let expected = if inside { 100 } else { 0 };
                assert_eq!(pixel(&canvas, x, y)[0], expected, "pixel {x},{y}");
            }
        }
    }

    /// A one-point line drawn down the middle of a pixel column, which is
    /// where egui puts a crisp line, lands on that column and on no other.
    #[test]
    fn a_rectangle_one_pixel_wide_lands_on_exactly_one_column() {
        let mut canvas = Canvas::new(6, 4, [0.0, 0.0, 0.0, 1.0]);
        canvas.paint(
            &[quad(pos2(2.0, 0.0), pos2(3.0, 4.0), Color32::WHITE)],
            1.0,
            &white(),
        );
        for x in 0..6 {
            let expected = if x == 2 { 255 } else { 0 };
            assert_eq!(pixel(&canvas, x, 1)[1], expected, "column {x}");
        }
    }

    /// At twice the scale a point is two pixels, and the scissor follows.
    #[test]
    fn the_clip_rectangle_is_cut_on_whole_physical_pixels() {
        let mut canvas = Canvas::new(8, 8, [0.0, 0.0, 0.0, 1.0]);
        let mut primitive = quad(pos2(0.0, 0.0), pos2(4.0, 4.0), Color32::WHITE);
        primitive.clip_rect = Rect::from_min_max(pos2(1.0, 1.0), pos2(3.0, 3.0));
        canvas.paint(&[primitive], 2.0, &white());
        for x in 0..8 {
            let expected = if (2..6).contains(&x) { 255 } else { 0 };
            assert_eq!(pixel(&canvas, x, 3)[2], expected, "column {x}");
        }
    }

    /// A texel is read back as the colour it was uploaded as when it is
    /// sampled at its centre - the round trip through linear light must not
    /// move it.
    #[test]
    fn a_texel_read_at_its_centre_comes_back_unchanged() {
        let mut textures = Textures::default();
        let mut image = ColorImage::new([2, 1], Color32::BLACK);
        image.pixels[1] = Color32::from_rgb(10, 128, 250);
        let mut delta = TexturesDelta::default();
        let id = TextureId::User(7);
        delta
            .set
            .push((id, ImageDelta::full(image, TextureOptions::LINEAR)));
        textures.set(&delta);
        let texel = textures
            .get(id)
            .unwrap()
            .sample([0.75, 0.5], TextureFilter::Linear);
        let bytes = texel.map(unorm8);
        assert_eq!(bytes, [10, 128, 250, 255]);
    }

    /// Between two texels the hardware blends in linear light, so halfway
    /// from black to white is lighter than the 128 a blend in gamma would
    /// give.
    #[test]
    fn halfway_between_two_texels_is_blended_in_linear_light() {
        let mut textures = Textures::default();
        let mut image = ColorImage::new([2, 1], Color32::BLACK);
        image.pixels[1] = Color32::WHITE;
        let mut delta = TexturesDelta::default();
        let id = TextureId::User(1);
        delta
            .set
            .push((id, ImageDelta::full(image, TextureOptions::LINEAR)));
        textures.set(&delta);
        let texel = textures
            .get(id)
            .unwrap()
            .sample([0.5, 0.5], TextureFilter::Linear);
        assert_eq!(unorm8(texel[0]), 188);
        assert_eq!(unorm8(texel[3]), 255);
    }

    /// A patch lands where it was put, and leaves the rest of the texture as
    /// it was: the font atlas grows by patches, a glyph at a time.
    #[test]
    fn a_partial_update_changes_only_its_own_texels() {
        let mut textures = Textures::default();
        let id = TextureId::User(3);
        let mut delta = TexturesDelta::default();
        delta.set.push((
            id,
            ImageDelta::full(
                ColorImage::new([3, 2], Color32::BLACK),
                TextureOptions::NEAREST,
            ),
        ));
        delta.set.push((
            id,
            ImageDelta::partial(
                [1, 1],
                ColorImage::new([2, 1], Color32::WHITE),
                TextureOptions::NEAREST,
            ),
        ));
        textures.set(&delta);
        let texels = &textures.get(id).unwrap().texels;
        let white = Color32::WHITE.to_array();
        let black = Color32::BLACK.to_array();
        assert_eq!(texels, &[black, black, black, black, white, white]);
    }
}
