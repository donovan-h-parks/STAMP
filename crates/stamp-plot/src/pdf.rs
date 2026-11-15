//! PDF export, written directly from a figure.
//!
//! The page has the figure's size in points and draws what [`crate::svg`]
//! draws, shape for shape: the same geometry, colours, opacity, dashes and
//! text positions. Text stays text, set in the figure's bundled font, which
//! is embedded (only the glyphs the figure uses) as a CID-keyed TrueType
//! font with a ToUnicode map, so that it can be selected, searched and
//! copied.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::io::Write as _;

use crate::figure::{Color, Figure, HAlign, Shape, Stroke};
use crate::text::PlotFont;

/// Render a figure as a single-page PDF document.
pub fn to_pdf(figure: &Figure) -> Vec<u8> {
    let font = figure.font;
    let face = font.face();
    let mut glyphs = Glyphs::default();
    let content = content_stream(figure, face, &mut glyphs);

    let mut pdf = Writer::new();
    let catalog = pdf.reserve();
    let pages = pdf.reserve();
    let page = pdf.reserve();
    let contents = pdf.reserve();
    let info = pdf.reserve();
    let type0 = (!glyphs.used.is_empty()).then(|| pdf.reserve());

    pdf.object(catalog, &format!("<< /Type /Catalog /Pages {pages} 0 R >>"));
    pdf.object(
        pages,
        &format!("<< /Type /Pages /Kids [{page} 0 R] /Count 1 >>"),
    );

    let mut resources = String::new();
    if let Some(type0) = type0 {
        let _ = write!(resources, " /Font << /F1 {type0} 0 R >>");
    }
    if !glyphs.alphas.is_empty() {
        resources.push_str(" /ExtGState <<");
        for &(stroke, alpha) in &glyphs.alphas {
            let _ = write!(
                resources,
                " /{} << /{} {} >>",
                alpha_name(stroke, alpha),
                if stroke { "CA" } else { "ca" },
                num(f64::from(alpha) / 255.0)
            );
        }
        resources.push_str(" >>");
    }
    pdf.object(
        page,
        &format!(
            "<< /Type /Page /Parent {pages} 0 R /MediaBox [0 0 {} {}] /Resources <<{resources} >> /Contents {contents} 0 R >>",
            num(figure.width),
            num(figure.height)
        ),
    );
    pdf.stream(contents, "", content.as_bytes());
    pdf.object(
        info,
        &format!(
            "<< /Producer (STAMP {}) /Creator (STAMP {}) >>",
            env!("CARGO_PKG_VERSION"),
            env!("CARGO_PKG_VERSION")
        ),
    );
    if let Some(type0) = type0 {
        write_font(&mut pdf, type0, font, face, &glyphs.used);
    }
    pdf.finish(catalog, info)
}

/// Numbers as PDF writes them: plain decimals, at a precision that keeps
/// files small without visible error.
fn num(value: f64) -> String {
    if !value.is_finite() {
        return "0".to_string();
    }
    let rounded = (value * 1000.0).round() / 1000.0;
    if rounded == 0.0 {
        "0".to_string()
    } else {
        rounded.to_string()
    }
}

fn channels(color: Color) -> String {
    format!(
        "{} {} {}",
        num(f64::from(color.r) / 255.0),
        num(f64::from(color.g) / 255.0),
        num(f64::from(color.b) / 255.0)
    )
}

/// The name of the graphics state that sets the fill (or stroke) opacity.
fn alpha_name(stroke: bool, alpha: u8) -> String {
    format!("{}{alpha}", if stroke { "S" } else { "F" })
}

/// What the content stream needs from the rest of the document: the glyphs
/// its text uses (with a character for each, for the ToUnicode map) and the
/// opacities it sets.
#[derive(Default)]
struct Glyphs {
    used: BTreeMap<u16, char>,
    alphas: BTreeSet<(bool, u8)>,
}

/// The graphics state last set in the content stream, so that it is set
/// again only when it changes.
struct State {
    fill: Option<Color>,
    stroke: Option<Color>,
    width: Option<f64>,
    dash: Option<Option<(f64, f64)>>,
}

impl State {
    fn set_fill(&mut self, out: &mut String, glyphs: &mut Glyphs, color: Color) {
        let last = self.fill.replace(color);
        if last.map(|c| (c.r, c.g, c.b)) != Some((color.r, color.g, color.b)) {
            let _ = writeln!(out, "{} rg", channels(color));
        }
        if last.map(|c| c.a) != Some(color.a) && (last.is_some() || color.a != 255) {
            glyphs.alphas.insert((false, color.a));
            let _ = writeln!(out, "/{} gs", alpha_name(false, color.a));
        }
    }

    fn set_stroke(&mut self, out: &mut String, glyphs: &mut Glyphs, stroke: &Stroke) {
        let color = stroke.color;
        let last = self.stroke.replace(color);
        if last.map(|c| (c.r, c.g, c.b)) != Some((color.r, color.g, color.b)) {
            let _ = writeln!(out, "{} RG", channels(color));
        }
        if last.map(|c| c.a) != Some(color.a) && (last.is_some() || color.a != 255) {
            glyphs.alphas.insert((true, color.a));
            let _ = writeln!(out, "/{} gs", alpha_name(true, color.a));
        }
        if self.width != Some(stroke.width) {
            self.width = Some(stroke.width);
            let _ = writeln!(out, "{} w", num(stroke.width));
        }
        // SVG draws a solid line for dashes it cannot draw, which PDF
        // viewers may refuse.
        let dash = stroke
            .dash
            .filter(|&(dash, gap)| dash >= 0.0 && gap >= 0.0 && dash + gap > 0.0);
        if self.dash != Some(dash) {
            self.dash = Some(dash);
            match dash {
                Some((dash, gap)) => {
                    let _ = writeln!(out, "[{} {}] 0 d", num(dash), num(gap));
                }
                None => out.push_str("[] 0 d\n"),
            }
        }
    }
}

/// A stroke that draws something: SVG draws nothing for a width of zero,
/// where PDF would draw the thinnest line the device can.
fn visible(stroke: Option<&Stroke>) -> Option<&Stroke> {
    stroke.filter(|s| s.width > 0.0 && s.width.is_finite())
}

/// Set the colours of a path just described and paint it.
fn paint(
    out: &mut String,
    state: &mut State,
    glyphs: &mut Glyphs,
    fill: Option<Color>,
    stroke: Option<&Stroke>,
    even_odd: bool,
) {
    let stroke = visible(stroke);
    if let Some(fill) = fill {
        state.set_fill(out, glyphs, fill);
    }
    if let Some(stroke) = stroke {
        state.set_stroke(out, glyphs, stroke);
    }
    let operator = match (fill.is_some(), stroke.is_some(), even_odd) {
        (true, true, false) => "B",
        (true, true, true) => "B*",
        (true, false, false) => "f",
        (true, false, true) => "f*",
        (false, true, _) => "S",
        (false, false, _) => "n",
    };
    out.push_str(operator);
    out.push('\n');
}

fn path(out: &mut String, points: &[(f64, f64)], close: bool) {
    for (i, (x, y)) in points.iter().enumerate() {
        let _ = writeln!(
            out,
            "{} {} {}",
            num(*x),
            num(*y),
            if i == 0 { "m" } else { "l" }
        );
    }
    if close {
        out.push_str("h\n");
    }
}

/// The drawing operators of the page. They work in the figure's own
/// coordinates (y downwards), which the first operator maps to the page.
fn content_stream(figure: &Figure, face: &ttf_parser::Face<'_>, glyphs: &mut Glyphs) -> String {
    let mut out = String::new();
    let mut state = State {
        fill: None,
        stroke: None,
        width: None,
        dash: None,
    };
    let _ = writeln!(out, "1 0 0 -1 0 {} cm", num(figure.height));
    // SVG's limit on mitred joins (PDF's default is 10).
    out.push_str("4 M\n");
    let background = Color {
        a: 255,
        ..figure.background
    };
    state.set_fill(&mut out, glyphs, background);
    let _ = writeln!(out, "0 0 {} {} re f", num(figure.width), num(figure.height));

    for shape in &figure.shapes {
        match shape {
            Shape::Line { points, stroke } => {
                if points.len() < 2 || visible(Some(stroke)).is_none() {
                    continue;
                }
                path(&mut out, points, false);
                paint(&mut out, &mut state, glyphs, None, Some(stroke), false);
            }
            Shape::Rect {
                x,
                y,
                width,
                height,
                fill,
                stroke,
            } => {
                // SVG draws no rectangle without a positive size.
                if !(*width > 0.0 && *height > 0.0) {
                    continue;
                }
                let _ = writeln!(
                    out,
                    "{} {} {} {} re",
                    num(*x),
                    num(*y),
                    num(*width),
                    num(*height)
                );
                paint(&mut out, &mut state, glyphs, *fill, stroke.as_ref(), false);
            }
            Shape::Circle {
                cx,
                cy,
                radius,
                fill,
                stroke,
            } => {
                if radius.is_nan() || *radius <= 0.0 {
                    continue;
                }
                circle(&mut out, *cx, *cy, *radius);
                paint(&mut out, &mut state, glyphs, *fill, stroke.as_ref(), false);
            }
            Shape::Polygon {
                points,
                fill,
                stroke,
            } => {
                if points.len() < 2 {
                    continue;
                }
                path(&mut out, points, true);
                paint(&mut out, &mut state, glyphs, *fill, stroke.as_ref(), false);
            }
            Shape::Area { rings, fill, .. } => {
                if rings.is_empty() {
                    continue;
                }
                for ring in rings {
                    path(&mut out, ring, true);
                }
                paint(&mut out, &mut state, glyphs, Some(*fill), None, true);
            }
            Shape::Text { x, y, text, style } => {
                if text.is_empty() || style.size.is_nan() || style.size <= 0.0 {
                    continue;
                }
                let units = f64::from(face.units_per_em());
                let mut hex = String::with_capacity(text.len() * 4);
                let mut advance = 0u32;
                for c in text.chars() {
                    // As the SVG: control characters become spaces.
                    let c = if c.is_control() { ' ' } else { c };
                    let glyph = face.glyph_index(c).unwrap_or(ttf_parser::GlyphId(0));
                    advance += u32::from(face.glyph_hor_advance(glyph).unwrap_or(0));
                    glyphs.used.entry(glyph.0).or_insert(c);
                    let _ = write!(hex, "{:04X}", glyph.0);
                }
                let width = f64::from(advance) / units * style.size;
                let dx = match style.h_align {
                    HAlign::Left => 0.0,
                    HAlign::Center => -width / 2.0,
                    HAlign::Right => -width,
                };
                let offset = style.baseline_offset_in(figure.font);
                // Rotation is counter-clockwise on the page, about the
                // anchor; the alignment offsets are in the text's own
                // (rotated) frame, as in the SVG. The text matrix also
                // turns the glyphs upright again in the flipped page.
                let (sin, cos) = style.rotation.to_radians().sin_cos();
                let ox = x + dx * cos + offset * sin;
                let oy = y - dx * sin + offset * cos;
                state.set_fill(&mut out, glyphs, style.color);
                let _ = writeln!(
                    out,
                    "BT /F1 {} Tf {} {} {} {} {} {} Tm <{hex}> Tj ET",
                    num(style.size),
                    num(cos),
                    num(-sin),
                    num(-sin),
                    num(-cos),
                    num(ox),
                    num(oy)
                );
            }
        }
    }
    out
}

/// A circle as four Bézier curves.
fn circle(out: &mut String, cx: f64, cy: f64, r: f64) {
    const KAPPA: f64 = 0.552_284_749_830_793_4;
    let k = r * KAPPA;
    let _ = writeln!(out, "{} {} m", num(cx + r), num(cy));
    let quarter = |out: &mut String, c1: (f64, f64), c2: (f64, f64), end: (f64, f64)| {
        let _ = writeln!(
            out,
            "{} {} {} {} {} {} c",
            num(c1.0),
            num(c1.1),
            num(c2.0),
            num(c2.1),
            num(end.0),
            num(end.1)
        );
    };
    quarter(out, (cx + r, cy + k), (cx + k, cy + r), (cx, cy + r));
    quarter(out, (cx - k, cy + r), (cx - r, cy + k), (cx - r, cy));
    quarter(out, (cx - r, cy - k), (cx - k, cy - r), (cx, cy - r));
    quarter(out, (cx + k, cy - r), (cx + r, cy - k), (cx + r, cy));
    out.push_str("h\n");
}

/// Write the font objects, the first as `type0`.
fn write_font(
    pdf: &mut Writer,
    type0: usize,
    font: PlotFont,
    face: &ttf_parser::Face<'_>,
    used: &BTreeMap<u16, char>,
) {
    let cid_font = pdf.reserve();
    let descriptor = pdf.reserve();
    let file = pdf.reserve();
    let to_unicode = pdf.reserve();

    let mut keep: BTreeSet<u16> = used.keys().copied().collect();
    keep.insert(0);
    let (data, subset) = match subset(face, &keep) {
        Some(data) => (data, true),
        None => (font.data().to_vec(), false),
    };
    let postscript = face
        .names()
        .into_iter()
        .find(|name| name.name_id == ttf_parser::name_id::POST_SCRIPT_NAME)
        .and_then(|name| name.to_string())
        .unwrap_or_else(|| font.bundled_family().replace(' ', ""));
    let base_font = if subset {
        format!("{}+{postscript}", subset_tag(&keep))
    } else {
        postscript
    };

    let units = f64::from(face.units_per_em());
    let scale = |v: f64| num(v * 1000.0 / units);

    // Widths of the glyphs used, in runs of consecutive glyph ids.
    let mut widths = String::new();
    let mut last: Option<u16> = None;
    for &glyph in used.keys() {
        let advance = f64::from(
            face.glyph_hor_advance(ttf_parser::GlyphId(glyph))
                .unwrap_or(0),
        );
        if last.is_some_and(|l| l + 1 == glyph) {
            let _ = write!(widths, " {}", scale(advance));
        } else {
            if last.is_some() {
                widths.push(']');
            }
            let _ = write!(widths, " {glyph} [{}", scale(advance));
        }
        last = Some(glyph);
    }
    if last.is_some() {
        widths.push(']');
    }

    pdf.object(
        type0,
        &format!(
            "<< /Type /Font /Subtype /Type0 /BaseFont /{base_font} /Encoding /Identity-H /DescendantFonts [{cid_font} 0 R] /ToUnicode {to_unicode} 0 R >>"
        ),
    );
    pdf.object(
        cid_font,
        &format!(
            "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /{base_font} /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor {descriptor} 0 R /DW 0 /W [{widths} ] /CIDToGIDMap /Identity >>"
        ),
    );

    // Symbolic, as fonts addressed by glyph id are; serif and fixed-width
    // where they are.
    let mut flags = 4;
    if font == PlotFont::TimesNewRoman {
        flags |= 2;
    }
    if face.is_monospaced() {
        flags |= 1;
    }
    let bbox = face.global_bounding_box();
    let cap_height = face.capital_height().map_or(units * 0.73, f64::from);
    pdf.object(
        descriptor,
        &format!(
            "<< /Type /FontDescriptor /FontName /{base_font} /Flags {flags} /FontBBox [{} {} {} {}] /ItalicAngle {} /Ascent {} /Descent {} /CapHeight {} /StemV 80 /FontFile2 {file} 0 R >>",
            scale(f64::from(bbox.x_min)),
            scale(f64::from(bbox.y_min)),
            scale(f64::from(bbox.x_max)),
            scale(f64::from(bbox.y_max)),
            num(f64::from(face.italic_angle())),
            scale(f64::from(face.ascender())),
            scale(f64::from(face.descender())),
            scale(cap_height),
        ),
    );
    pdf.stream(file, &format!(" /Length1 {}", data.len()), &data);
    pdf.stream(to_unicode, "", to_unicode_cmap(used).as_bytes());
}

/// A tag naming a subset, six capital letters derived from the glyphs it
/// holds.
fn subset_tag(glyphs: &BTreeSet<u16>) -> String {
    // FNV-1a
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for glyph in glyphs {
        for byte in glyph.to_be_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
    }
    (0..6)
        .map(|i| char::from(b'A' + ((hash >> (i * 8)) % 26) as u8))
        .collect()
}

/// The map from glyph ids back to the characters they show.
fn to_unicode_cmap(used: &BTreeMap<u16, char>) -> String {
    let entries: Vec<(u16, char)> = used
        .iter()
        .filter(|(&glyph, _)| glyph != 0)
        .map(|(&glyph, &c)| (glyph, c))
        .collect();
    let mut out = String::from(
        "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n\
         /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n\
         /CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n\
         1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n",
    );
    // At most 100 entries to a block.
    for block in entries.chunks(100) {
        let _ = writeln!(out, "{} beginbfchar", block.len());
        for (glyph, c) in block {
            let mut units = [0u16; 2];
            let unicode: String = c
                .encode_utf16(&mut units)
                .iter()
                .map(|u| format!("{u:04X}"))
                .collect();
            let _ = writeln!(out, "<{glyph:04X}> <{unicode}>");
        }
        out.push_str("endbfchar\n");
    }
    out.push_str(
        "endcmap\nCMapName currentdict /defineresource /CMap defineresource pop\nend\nend\n",
    );
    out
}

/// The tables a TrueType font embedded in a PDF needs: outlines, metrics
/// and hinting.
const SUBSET_TABLES: [&[u8; 4]; 10] = [
    b"OS/2", b"cvt ", b"fpgm", b"glyf", b"head", b"hhea", b"hmtx", b"loca", b"maxp", b"prep",
];

/// A copy of a TrueType font with the outlines of every glyph but `keep`
/// (and the glyphs they are built from) removed. Glyph ids stay as they
/// are, so the subset is addressed exactly as the whole font. `None` if the
/// font is not of the expected form; the whole font is embedded then.
fn subset(face: &ttf_parser::Face<'_>, keep: &BTreeSet<u16>) -> Option<Vec<u8>> {
    let raw = face.raw_face();
    let table = |tag: &[u8; 4]| raw.table(ttf_parser::Tag::from_bytes(tag));
    let head = table(b"head")?;
    let loca = table(b"loca")?;
    let glyf = table(b"glyf")?;
    let maxp = table(b"maxp")?;
    let u16_at = |data: &[u8], at: usize| -> Option<u16> {
        Some(u16::from_be_bytes(data.get(at..at + 2)?.try_into().ok()?))
    };
    let count = usize::from(u16_at(maxp, 4)?);
    let long = match u16_at(head, 50)? {
        0 => false,
        1 => true,
        _ => return None,
    };
    let offset = |glyph: usize| -> Option<usize> {
        if long {
            let at = glyph * 4;
            Some(u32::from_be_bytes(loca.get(at..at + 4)?.try_into().ok()?) as usize)
        } else {
            Some(usize::from(u16_at(loca, glyph * 2)?) * 2)
        }
    };
    let outline = |glyph: usize| -> Option<&[u8]> { glyf.get(offset(glyph)?..offset(glyph + 1)?) };

    // The glyphs kept, with the components of composite glyphs.
    let mut kept = vec![false; count];
    let mut pending: Vec<usize> = keep.iter().map(|&g| usize::from(g)).collect();
    while let Some(glyph) = pending.pop() {
        if glyph >= count || kept[glyph] {
            continue;
        }
        kept[glyph] = true;
        let data = outline(glyph)?;
        if data.len() < 10 || (u16_at(data, 0)? as i16) >= 0 {
            continue;
        }
        let mut at = 10;
        loop {
            let flags = u16_at(data, at)?;
            pending.push(usize::from(u16_at(data, at + 2)?));
            at += 4 + if flags & 0x0001 != 0 { 4 } else { 2 };
            at += if flags & 0x0008 != 0 {
                2
            } else if flags & 0x0040 != 0 {
                4
            } else if flags & 0x0080 != 0 {
                8
            } else {
                0
            };
            if flags & 0x0020 == 0 {
                break;
            }
        }
    }

    let mut new_glyf = Vec::new();
    let mut new_loca = Vec::with_capacity((count + 1) * 4);
    for (glyph, &kept) in kept.iter().enumerate() {
        new_loca.extend_from_slice(&(new_glyf.len() as u32).to_be_bytes());
        if kept {
            new_glyf.extend_from_slice(outline(glyph)?);
            while new_glyf.len() % 4 != 0 {
                new_glyf.push(0);
            }
        }
    }
    new_loca.extend_from_slice(&(new_glyf.len() as u32).to_be_bytes());

    let mut new_head = head.to_vec();
    // Long offsets in `loca`; the checksum adjustment is set at the end.
    new_head
        .get_mut(50..52)?
        .copy_from_slice(&1u16.to_be_bytes());
    new_head.get_mut(8..12)?.copy_from_slice(&[0; 4]);

    let mut tables: Vec<(&[u8; 4], Vec<u8>)> = Vec::new();
    for tag in SUBSET_TABLES {
        let data = match tag {
            b"glyf" => std::mem::take(&mut new_glyf),
            b"loca" => std::mem::take(&mut new_loca),
            b"head" => std::mem::take(&mut new_head),
            _ => match table(tag) {
                Some(data) => data.to_vec(),
                None => continue,
            },
        };
        tables.push((tag, data));
    }
    Some(write_sfnt(&tables))
}

fn checksum(data: &[u8]) -> u32 {
    data.chunks(4).fold(0u32, |sum, chunk| {
        let mut word = [0u8; 4];
        word[..chunk.len()].copy_from_slice(chunk);
        sum.wrapping_add(u32::from_be_bytes(word))
    })
}

/// A TrueType font file of the given tables, which are in order of tag.
fn write_sfnt(tables: &[(&[u8; 4], Vec<u8>)]) -> Vec<u8> {
    let count = tables.len() as u16;
    let power = 1u16 << (15 - count.max(1).leading_zeros()) as u16;
    let search_range = power * 16;
    let entry_selector = power.trailing_zeros() as u16;
    let range_shift = count * 16 - search_range;

    let mut out = Vec::new();
    out.extend_from_slice(&0x0001_0000u32.to_be_bytes());
    for value in [count, search_range, entry_selector, range_shift] {
        out.extend_from_slice(&value.to_be_bytes());
    }
    let mut offset = 12 + 16 * tables.len();
    let mut head_at = None;
    for (tag, data) in tables {
        if *tag == b"head" {
            head_at = Some(offset);
        }
        out.extend_from_slice(*tag);
        out.extend_from_slice(&checksum(data).to_be_bytes());
        out.extend_from_slice(&(offset as u32).to_be_bytes());
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        offset += data.len().next_multiple_of(4);
    }
    for (_, data) in tables {
        out.extend_from_slice(data);
        out.resize(out.len().next_multiple_of(4), 0);
    }
    if let Some(at) = head_at {
        let adjustment = 0xB1B0_AFBAu32.wrapping_sub(checksum(&out));
        out[at + 8..at + 12].copy_from_slice(&adjustment.to_be_bytes());
    }
    out
}

/// Builds the file: numbered objects, then the cross-reference table that
/// gives the position of each.
struct Writer {
    out: Vec<u8>,
    offsets: Vec<usize>,
}

impl Writer {
    fn new() -> Writer {
        Writer {
            // The comment of bytes above 127 marks the file as binary.
            out: b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec(),
            offsets: Vec::new(),
        }
    }

    /// The number of a new object, to be written later.
    fn reserve(&mut self) -> usize {
        self.offsets.push(0);
        self.offsets.len()
    }

    fn object(&mut self, id: usize, body: &str) {
        self.offsets[id - 1] = self.out.len();
        let _ = write!(self.out, "{id} 0 obj\n{body}\nendobj\n");
    }

    /// A compressed stream; `extra` adds entries to its dictionary.
    fn stream(&mut self, id: usize, extra: &str, data: &[u8]) {
        let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::best());
        let _ = encoder.write_all(data);
        let compressed = encoder.finish().unwrap_or_default();
        self.offsets[id - 1] = self.out.len();
        let _ = write!(
            self.out,
            "{id} 0 obj\n<< /Length {} /Filter /FlateDecode{extra} >>\nstream\n",
            compressed.len()
        );
        self.out.extend_from_slice(&compressed);
        self.out.extend_from_slice(b"\nendstream\nendobj\n");
    }

    fn finish(mut self, root: usize, info: usize) -> Vec<u8> {
        let xref = self.out.len();
        let _ = write!(
            self.out,
            "xref\n0 {}\n0000000000 65535 f \n",
            self.offsets.len() + 1
        );
        for offset in &self.offsets {
            // Each entry is 20 bytes, its line end included.
            let _ = writeln!(self.out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            self.out,
            "trailer\n<< /Size {} /Root {root} 0 R /Info {info} 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            self.offsets.len() + 1
        );
        self.out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::{TextStyle, VAlign};
    use std::io::Read as _;

    fn sample() -> Figure {
        let mut figure = Figure::new(200.0, 100.0);
        figure.rect(0.0, 0.0, 36.0, 36.0, Some(Color::RED), None);
        figure.line(
            &[(10.0, 90.0), (190.0, 90.0)],
            Stroke::dashed(Color::BLACK, 1.5, 4.0, 2.0),
        );
        figure.circle(
            50.0,
            50.0,
            5.0,
            Some(Color {
                r: 0,
                g: 0,
                b: 255,
                a: 128,
            }),
            Some(Stroke::solid(Color::BLACK, 0.5)),
        );
        figure.area(
            vec![vec![(100.0, 10.0), (140.0, 10.0), (140.0, 40.0)]],
            Color::rgb(0, 128, 0),
        );
        figure.text(
            100.0,
            60.0,
            "p − 0.05 µm α",
            TextStyle::new(12.0).align(HAlign::Center, VAlign::Middle),
        );
        figure.text(20.0, 80.0, "Rotated", TextStyle::new(8.0).rotated(90.0));
        figure
    }

    /// The objects of a PDF, by number, checking that the cross-reference
    /// table gives the position of each.
    fn objects(pdf: &[u8]) -> BTreeMap<usize, Vec<u8>> {
        let text = String::from_utf8_lossy(pdf);
        assert!(pdf.starts_with(b"%PDF-1.7\n"));
        assert!(text.ends_with("%%EOF\n"));
        let start: usize = text
            .rsplit_once("startxref\n")
            .unwrap()
            .1
            .lines()
            .next()
            .unwrap()
            .parse()
            .unwrap();
        assert!(pdf[start..].starts_with(b"xref\n0 "));
        let xref = String::from_utf8_lossy(&pdf[start..]);
        let mut lines = xref.lines().skip(1);
        let count: usize = lines
            .next()
            .unwrap()
            .split(' ')
            .nth(1)
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!(lines.next().unwrap(), "0000000000 65535 f ");
        let trailer = xref.split_once("trailer\n").unwrap().1;
        assert!(trailer.contains(&format!("/Size {count} ")));
        assert!(trailer.contains("/Root 1 0 R"));
        let mut objects = BTreeMap::new();
        for id in 1..count {
            let line = lines.next().unwrap();
            // Entries are exactly 20 bytes, line end included.
            assert_eq!(line.len(), 19, "{line:?}");
            assert!(line.ends_with(" 00000 n "));
            let offset: usize = line[..10].parse().unwrap();
            let header = format!("{id} 0 obj\n");
            assert!(pdf[offset..].starts_with(header.as_bytes()), "object {id}");
            let body = &pdf[offset + header.len()..];
            let end = body.windows(7).position(|w| w == b"endobj\n").unwrap();
            objects.insert(id, body[..end].to_vec());
        }
        objects
    }

    /// The decompressed data of a stream object.
    fn stream_data(object: &[u8]) -> Vec<u8> {
        let start = object.windows(7).position(|w| w == b"stream\n").unwrap() + 7;
        let end = object
            .windows(10)
            .rposition(|w| w == b"\nendstream")
            .unwrap();
        let mut data = Vec::new();
        flate2::read::ZlibDecoder::new(&object[start..end])
            .read_to_end(&mut data)
            .unwrap();
        data
    }

    #[test]
    fn writes_a_well_formed_pdf() {
        let pdf = to_pdf(&sample());
        let objects = objects(&pdf);
        let find = |needle: &str| {
            objects
                .values()
                .find(|o| String::from_utf8_lossy(o).contains(needle))
                .unwrap_or_else(|| panic!("no object with {needle}"))
        };
        assert!(String::from_utf8_lossy(find("/Type /Page ")).contains("/MediaBox [0 0 200 100]"));
        assert!(String::from_utf8_lossy(find("/Subtype /Type0")).contains("/Encoding /Identity-H"));
        assert!(String::from_utf8_lossy(find("/CIDFontType2")).contains("/CIDToGIDMap /Identity"));
        // Opacity of the circle's fill, but not of its outline.
        assert!(String::from_utf8_lossy(find("/ExtGState")).contains("/F128 << /ca 0.502 >>"));

        let page = String::from_utf8_lossy(find("/Type /Page "));
        let contents: usize = page
            .split_once("/Contents ")
            .unwrap()
            .1
            .split(' ')
            .next()
            .unwrap()
            .parse()
            .unwrap();
        let content = String::from_utf8(stream_data(&objects[&contents])).unwrap();
        assert!(content.starts_with("1 0 0 -1 0 100 cm\n"));
        assert!(content.contains("[4 2] 0 d\n1.5 w") || content.contains("1.5 w\n[4 2] 0 d"));
        assert!(content.contains("/F128 gs"));
        // The area is filled by the even-odd rule.
        assert!(content.contains("\nf*\n"));
        // The rotated label runs up the page.
        assert!(
            content.contains("BT /F1 8 Tf 0 -1 -1 0 20 80 Tm"),
            "{content}"
        );

        // The ToUnicode map gives back the characters, including those
        // outside Latin-1.
        let face = PlotFont::Arial.face();
        let glyph = |c: char| face.glyph_index(c).unwrap().0;
        let cmap = objects
            .values()
            .map(|o| String::from_utf8_lossy(&stream_data_or_empty(o)).into_owned())
            .find(|s| s.contains("beginbfchar"))
            .unwrap();
        for (c, unicode) in [('−', "2212"), ('µ', "00B5"), ('α', "03B1"), ('p', "0070")] {
            assert!(
                cmap.contains(&format!("<{:04X}> <{unicode}>", glyph(c))),
                "{c}: {cmap}"
            );
            assert!(content.contains(&format!("{:04X}", glyph(c))));
        }
    }

    fn stream_data_or_empty(object: &[u8]) -> Vec<u8> {
        if object.windows(7).any(|w| w == b"stream\n") {
            stream_data(object)
        } else {
            Vec::new()
        }
    }

    #[test]
    fn text_is_centred_by_the_width_of_its_glyphs() {
        let mut figure = Figure::new(100.0, 100.0);
        figure.text(
            50.0,
            50.0,
            "Ag",
            TextStyle::new(10.0).align(HAlign::Right, VAlign::Baseline),
        );
        let pdf = to_pdf(&figure);
        let objects = objects(&pdf);
        let content = objects
            .values()
            .map(|o| String::from_utf8_lossy(&stream_data_or_empty(o)).into_owned())
            .find(|s| s.contains(" Tm "))
            .unwrap();
        let x = 50.0 - PlotFont::Arial.width("Ag", 10.0);
        assert!(
            content.contains(&format!("1 0 0 -1 {} 50 Tm", num(x))),
            "{content}"
        );
    }

    #[test]
    fn a_figure_without_text_has_no_font() {
        let mut figure = Figure::new(10.0, 20.0);
        figure.rect(
            1.0,
            1.0,
            2.0,
            2.0,
            None,
            Some(Stroke::solid(Color::BLACK, 1.0)),
        );
        let pdf = to_pdf(&figure);
        let objects = objects(&pdf);
        assert_eq!(objects.len(), 5);
        assert!(!String::from_utf8_lossy(&pdf).contains("/Font"));
    }

    /// The embedded font holds the glyphs used (and the parts of composite
    /// glyphs) and no others, under their own ids.
    #[test]
    fn fonts_are_subset() {
        for font in PlotFont::ALL {
            let face = font.face();
            let glyph = |c: char| face.glyph_index(c).unwrap();
            // "é" is a composite glyph in some of the fonts.
            let keep: BTreeSet<u16> = [0, glyph('A').0, glyph('é').0, glyph('−').0]
                .into_iter()
                .collect();
            let data = subset(face, &keep).unwrap();
            assert!(data.len() < font.data().len() / 4, "{font:?}");
            let small = ttf_parser::Face::parse(&data, 0).unwrap();
            assert_eq!(small.number_of_glyphs(), face.number_of_glyphs());
            for c in ['A', 'é', '−'] {
                let mut sink = Sink(0);
                assert!(
                    small.outline_glyph(glyph(c), &mut sink).is_some(),
                    "{font:?} {c}"
                );
                let mut whole = Sink(0);
                face.outline_glyph(glyph(c), &mut whole);
                assert_eq!(sink.0, whole.0);
            }
            let mut sink = Sink(0);
            assert!(small.outline_glyph(glyph('B'), &mut sink).is_none());
            assert_eq!(
                small.glyph_hor_advance(glyph('B')),
                face.glyph_hor_advance(glyph('B'))
            );
            assert_eq!(checksum(&data), 0xB1B0_AFBA);
        }
    }

    struct Sink(usize);

    impl ttf_parser::OutlineBuilder for Sink {
        fn move_to(&mut self, _: f32, _: f32) {
            self.0 += 1;
        }
        fn line_to(&mut self, _: f32, _: f32) {
            self.0 += 1;
        }
        fn quad_to(&mut self, _: f32, _: f32, _: f32, _: f32) {
            self.0 += 1;
        }
        fn curve_to(&mut self, _: f32, _: f32, _: f32, _: f32, _: f32, _: f32) {
            self.0 += 1;
        }
        fn close(&mut self) {
            self.0 += 1;
        }
    }

    #[test]
    fn subset_tags_are_six_capitals() {
        let tag = subset_tag(&[0, 5, 9].into_iter().collect());
        assert_eq!(tag.len(), 6);
        assert!(tag.chars().all(|c| c.is_ascii_uppercase()));
    }
}
