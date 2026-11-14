//! Outlines of the countries and lakes of the world from Natural Earth
//! (public domain, "Made with Natural Earth"), the map projections of the
//! map plot, and clipping of outlines to a rectangle.
//!
//! The outlines are compiled in from `crates/stamp-plot/data/*.bin`, made by
//! `scripts/naturalearth/convert.py`; its README describes the format.

use std::sync::OnceLock;

/// A longitude and latitude, in degrees.
pub type LonLat = (f64, f64);

/// The detail of the outlines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Resolution {
    /// Chosen by the extent of the map: 1:110m for maps wider than 90° of
    /// longitude, 1:50m for smaller areas.
    #[default]
    Auto,
    /// Natural Earth's 1:110 million scale outlines.
    Low,
    /// Natural Earth's 1:50 million scale outlines, simplified to 0.02°.
    Medium,
}

impl Resolution {
    pub const ALL: [Resolution; 3] = [Resolution::Auto, Resolution::Low, Resolution::Medium];

    pub fn name(self) -> &'static str {
        match self {
            Resolution::Auto => "Automatic",
            Resolution::Low => "1:110m (world)",
            Resolution::Medium => "1:50m (regions)",
        }
    }

    /// The outlines to draw for a map `lon_span` degrees wide.
    pub fn for_span(self, lon_span: f64) -> Resolution {
        match self {
            Resolution::Auto if lon_span > 90.0 => Resolution::Low,
            Resolution::Auto => Resolution::Medium,
            fixed => fixed,
        }
    }
}

/// The kinds of outline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    Countries,
    Lakes,
}

static NE_110M_COUNTRIES: &[u8] = include_bytes!("../data/ne_110m_countries.bin");
static NE_50M_COUNTRIES: &[u8] = include_bytes!("../data/ne_50m_countries.bin");
static NE_110M_LAKES: &[u8] = include_bytes!("../data/ne_110m_lakes.bin");
static NE_50M_LAKES: &[u8] = include_bytes!("../data/ne_50m_lakes.bin");

const MAGIC: &[u8] = b"STMAP1";
/// Longitudes are stored as `lon / 180 * SCALE`, latitudes as
/// `lat / 90 * SCALE`.
const SCALE: f64 = 32767.0;

/// Read the outlines from the format written by `convert.py`: open rings
/// (the first vertex is not repeated at the end) of longitudes and
/// latitudes.
pub fn decode(bytes: &[u8]) -> Result<Vec<Vec<LonLat>>, String> {
    let rest = bytes
        .strip_prefix(MAGIC)
        .ok_or_else(|| "not a STAMP map outline file".to_string())?;
    let mut position = 0usize;
    let mut varint = || -> Result<u64, String> {
        let mut value = 0u64;
        for shift in (0..64).step_by(7) {
            let byte = *rest
                .get(position)
                .ok_or_else(|| "map outline file is truncated".to_string())?;
            position += 1;
            value |= u64::from(byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
        }
        Err("bad number in map outline file".to_string())
    };
    let zigzag = |v: u64| (v >> 1) as i64 ^ -((v & 1) as i64);
    let rings = varint()? as usize;
    let mut out = Vec::with_capacity(rings.min(1 << 16));
    for _ in 0..rings {
        let count = varint()? as usize;
        let mut ring = Vec::with_capacity(count.min(1 << 20));
        let (mut x, mut y) = (0i64, 0i64);
        for _ in 0..count {
            x += zigzag(varint()?);
            y += zigzag(varint()?);
            ring.push((x as f64 / SCALE * 180.0, y as f64 / SCALE * 90.0));
        }
        out.push(ring);
    }
    Ok(out)
}

/// The outlines of a layer at a resolution (`Auto` gives the 1:110m ones),
/// read once.
pub fn outlines(layer: Layer, resolution: Resolution) -> &'static [Vec<LonLat>] {
    static CACHE: [OnceLock<Vec<Vec<LonLat>>>; 4] = [
        OnceLock::new(),
        OnceLock::new(),
        OnceLock::new(),
        OnceLock::new(),
    ];
    let medium = resolution == Resolution::Medium;
    let (index, bytes) = match (layer, medium) {
        (Layer::Countries, false) => (0, NE_110M_COUNTRIES),
        (Layer::Countries, true) => (1, NE_50M_COUNTRIES),
        (Layer::Lakes, false) => (2, NE_110M_LAKES),
        (Layer::Lakes, true) => (3, NE_50M_LAKES),
    };
    CACHE[index].get_or_init(|| decode(bytes).expect("the bundled map outlines are valid"))
}

/// A map projection, with the central meridian at 0°.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Projection {
    /// Equal Earth (Šavrič, Patterson and Jenny 2018): areas are true, so
    /// regions with many samples are not exaggerated near the poles.
    #[default]
    EqualEarth,
    /// Robinson (1974), a compromise for world maps.
    Robinson,
    /// Longitude and latitude as x and y (plate carrée).
    Equirectangular,
    /// Mercator, cut at 85° north and south.
    Mercator,
}

/// Mercator maps stop at this latitude.
pub const MERCATOR_LIMIT: f64 = 85.0;

/// Robinson's table (Snyder 1993, Flattening the Earth, table for the
/// Robinson projection): for every 5° of latitude, the length of the
/// parallel and its distance from the equator, relative to those at the
/// equator and the pole.
const ROBINSON: [(f64, f64); 19] = [
    (1.0000, 0.0000),
    (0.9986, 0.0620),
    (0.9954, 0.1240),
    (0.9900, 0.1860),
    (0.9822, 0.2480),
    (0.9730, 0.3100),
    (0.9600, 0.3720),
    (0.9427, 0.4340),
    (0.9216, 0.4958),
    (0.8962, 0.5571),
    (0.8679, 0.6176),
    (0.8350, 0.6769),
    (0.7986, 0.7346),
    (0.7597, 0.7903),
    (0.7186, 0.8435),
    (0.6732, 0.8936),
    (0.6213, 0.9394),
    (0.5722, 0.9761),
    (0.5322, 1.0000),
];

/// Interpolate Robinson's table at `lat` degrees (0 to 90) through the
/// cubic that passes through the four nearest rows (Lagrange; exact at the
/// rows).
fn robinson_row(lat: f64) -> (f64, f64) {
    let position = (lat.abs().min(90.0)) / 5.0;
    let i = (position.floor() as usize).min(17);
    // Four rows around the interval, kept inside the table.
    let first = i.saturating_sub(1).min(15);
    let t = position - first as f64;
    let mut out = (0.0, 0.0);
    for k in 0..4 {
        let mut weight = 1.0;
        for m in 0..4 {
            if m != k {
                weight *= (t - m as f64) / (k as f64 - m as f64);
            }
        }
        out.0 += weight * ROBINSON[first + k].0;
        out.1 += weight * ROBINSON[first + k].1;
    }
    out
}

impl Projection {
    pub const ALL: [Projection; 4] = [
        Projection::EqualEarth,
        Projection::Robinson,
        Projection::Equirectangular,
        Projection::Mercator,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Projection::EqualEarth => "Equal Earth",
            Projection::Robinson => "Robinson",
            Projection::Equirectangular => "Equirectangular",
            Projection::Mercator => "Mercator",
        }
    }

    /// Project a longitude and latitude (degrees) onto the plane of a
    /// sphere of radius 1, with y increasing northwards.
    pub fn forward(self, lon: f64, lat: f64) -> (f64, f64) {
        let lambda = lon.clamp(-180.0, 180.0).to_radians();
        let lat = lat.clamp(-90.0, 90.0);
        match self {
            Projection::Equirectangular => (lambda, lat.to_radians()),
            Projection::Mercator => {
                let phi = lat.clamp(-MERCATOR_LIMIT, MERCATOR_LIMIT).to_radians();
                (lambda, (std::f64::consts::FRAC_PI_4 + phi / 2.0).tan().ln())
            }
            Projection::EqualEarth => {
                const A1: f64 = 1.340264;
                const A2: f64 = -0.081106;
                const A3: f64 = 0.000893;
                const A4: f64 = 0.003796;
                let m = 3f64.sqrt() / 2.0;
                let theta = (m * lat.to_radians().sin()).asin();
                let t2 = theta * theta;
                let t6 = t2 * t2 * t2;
                let x = lambda * theta.cos()
                    / (m * (A1 + 3.0 * A2 * t2 + t6 * (7.0 * A3 + 9.0 * A4 * t2)));
                let y = theta * (A1 + A2 * t2 + t6 * (A3 + A4 * t2));
                (x, y)
            }
            Projection::Robinson => {
                let (length, distance) = robinson_row(lat);
                (0.8487 * length * lambda, 1.3523 * distance * lat.signum())
            }
        }
    }

    /// The latitudes the projection can show.
    pub fn lat_limits(self) -> (f64, f64) {
        match self {
            Projection::Mercator => (-MERCATOR_LIMIT, MERCATOR_LIMIT),
            _ => (-90.0, 90.0),
        }
    }
}

/// A rectangle, `x` from `left` to `right` and `y` from `bottom` to `top`
/// (in degrees, or in projected units).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub left: f64,
    pub right: f64,
    pub bottom: f64,
    pub top: f64,
}

impl Rect {
    pub fn contains(&self, (x, y): (f64, f64)) -> bool {
        x >= self.left && x <= self.right && y >= self.bottom && y <= self.top
    }
}

/// A point of an outline, and whether the edge that arrives at it (from the
/// point before it in the ring) is drawn. Edges along a clipping rectangle,
/// and Natural Earth's cuts along ±180° and the south pole, are filled but
/// not drawn.
pub type Vertex = ((f64, f64), bool);

/// The vertices of a ring of longitudes and latitudes, marking the edges
/// along ±180° and the south pole as not drawn.
pub fn mark_seams(ring: &[LonLat]) -> Vec<Vertex> {
    let seam = |a: LonLat, b: LonLat| {
        (a.0.abs() >= 179.999 && b.0.abs() >= 179.999 && a.0.signum() == b.0.signum())
            || (a.1 <= -89.99 && b.1 <= -89.99)
    };
    (0..ring.len())
        .map(|i| {
            let before = ring[(i + ring.len() - 1) % ring.len()];
            (ring[i], !seam(before, ring[i]))
        })
        .collect()
}

/// Clip a ring to a rectangle (Sutherland–Hodgman). The result may be
/// empty; edges added along the rectangle are marked as not drawn.
pub fn clip_ring(ring: &[Vertex], rect: &Rect) -> Vec<Vertex> {
    // Each side as: is a point inside, and where a segment crosses it.
    type Inside = fn(&Rect, (f64, f64)) -> bool;
    let sides: [(Inside, usize); 4] = [
        (|r, p| p.0 >= r.left, 0),
        (|r, p| p.0 <= r.right, 1),
        (|r, p| p.1 >= r.bottom, 2),
        (|r, p| p.1 <= r.top, 3),
    ];
    let crossing = |side: usize, a: (f64, f64), b: (f64, f64)| -> (f64, f64) {
        let (edge, vertical) = match side {
            0 => (rect.left, true),
            1 => (rect.right, true),
            2 => (rect.bottom, false),
            _ => (rect.top, false),
        };
        if vertical {
            let t = (edge - a.0) / (b.0 - a.0);
            (edge, a.1 + t * (b.1 - a.1))
        } else {
            let t = (edge - a.1) / (b.1 - a.1);
            (a.0 + t * (b.0 - a.0), edge)
        }
    };
    let mut points = ring.to_vec();
    for (inside, side) in sides {
        if points.is_empty() {
            break;
        }
        let mut out = Vec::with_capacity(points.len() + 4);
        for i in 0..points.len() {
            let (end, drawn) = points[i];
            let (start, _) = points[(i + points.len() - 1) % points.len()];
            match (inside(rect, start), inside(rect, end)) {
                (true, true) => out.push((end, drawn)),
                (false, true) => {
                    out.push((crossing(side, start, end), false));
                    out.push((end, drawn));
                }
                (true, false) => out.push((crossing(side, start, end), drawn)),
                (false, false) => {}
            }
        }
        points = out;
    }
    if points.len() < 3 {
        points.clear();
    }
    points
}

/// Split a polyline into the pieces inside a rectangle (Liang–Barsky).
pub fn clip_line(points: &[(f64, f64)], rect: &Rect) -> Vec<Vec<(f64, f64)>> {
    let mut pieces: Vec<Vec<(f64, f64)>> = Vec::new();
    let mut current: Vec<(f64, f64)> = Vec::new();
    for pair in points.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let (mut t0, mut t1) = (0.0f64, 1.0f64);
        let mut visible = true;
        for (p, q) in [
            (-dx, a.0 - rect.left),
            (dx, rect.right - a.0),
            (-dy, a.1 - rect.bottom),
            (dy, rect.top - a.1),
        ] {
            if p == 0.0 {
                if q < 0.0 {
                    visible = false;
                }
            } else {
                let r = q / p;
                if p < 0.0 {
                    t0 = t0.max(r);
                } else {
                    t1 = t1.min(r);
                }
            }
        }
        if !visible || t0 > t1 {
            if current.len() >= 2 {
                pieces.push(std::mem::take(&mut current));
            }
            current.clear();
            continue;
        }
        let start = (a.0 + t0 * dx, a.1 + t0 * dy);
        let end = (a.0 + t1 * dx, a.1 + t1 * dy);
        if current.last() != Some(&start) {
            if current.len() >= 2 {
                pieces.push(std::mem::take(&mut current));
            }
            current = vec![start];
        }
        current.push(end);
        if t1 < 1.0 {
            pieces.push(std::mem::take(&mut current));
        }
    }
    if current.len() >= 2 {
        pieces.push(current);
    }
    pieces
}

/// Add points along edges longer than `step` degrees, so that they follow
/// the curves projections give them. Added points keep the edge's
/// drawing.
pub fn densify(ring: &[Vertex], step: f64) -> Vec<Vertex> {
    let mut out = Vec::with_capacity(ring.len());
    for i in 0..ring.len() {
        let (start, _) = ring[(i + ring.len() - 1) % ring.len()];
        let (end, drawn) = ring[i];
        let length = (end.0 - start.0).abs().max((end.1 - start.1).abs());
        let parts = (length / step).ceil() as usize;
        for k in 1..parts {
            let t = k as f64 / parts as f64;
            out.push((
                (
                    start.0 + t * (end.0 - start.0),
                    start.1 + t * (end.1 - start.1),
                ),
                drawn,
            ));
        }
        out.push((end, drawn));
    }
    out
}

/// The drawn edges of a ring as polylines.
pub fn drawn_edges(ring: &[Vertex]) -> Vec<Vec<(f64, f64)>> {
    let n = ring.len();
    // Start after an edge that is not drawn, so that lines are not split
    // where the ring begins.
    let Some(start) = (0..n).find(|&i| !ring[i].1) else {
        let mut all: Vec<(f64, f64)> = ring.iter().map(|v| v.0).collect();
        if let Some(&first) = all.first() {
            all.push(first);
        }
        return vec![all];
    };
    let mut lines = Vec::new();
    let mut current: Vec<(f64, f64)> = vec![ring[start].0];
    for k in 1..=n {
        let (point, drawn) = ring[(start + k) % n];
        if drawn {
            current.push(point);
        } else {
            if current.len() >= 2 {
                lines.push(std::mem::take(&mut current));
            }
            current = vec![point];
        }
    }
    if current.len() >= 2 {
        lines.push(current);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_outlines_decode() {
        let world = outlines(Layer::Countries, Resolution::Low);
        assert_eq!(world.len(), 287);
        assert_eq!(world.iter().map(Vec::len).sum::<usize>(), 10_293);
        let detailed = outlines(Layer::Countries, Resolution::Medium);
        assert_eq!(detailed.len(), 1591);
        assert_eq!(outlines(Layer::Lakes, Resolution::Low).len(), 24);
        assert_eq!(outlines(Layer::Lakes, Resolution::Medium).len(), 110);
        for ring in world.iter().chain(detailed) {
            assert!(ring.len() >= 3);
            for &(lon, lat) in ring {
                assert!((-180.0..=180.0).contains(&lon) && (-90.0..=90.0).contains(&lat));
            }
        }
        // Known vertices of the 1:110m countries, within a quantisation
        // step: the north-western corner of the US-Canada border on the
        // 49th parallel, and the southern tip of Africa near Cape Agulhas.
        let near = |target: LonLat, tolerance: f64| {
            world.iter().flatten().any(|&(lon, lat)| {
                (lon - target.0).abs() <= tolerance && (lat - target.1).abs() <= tolerance
            })
        };
        assert!(near((-122.84, 49.0), 0.01));
        assert!(near((-120.0, 49.0), 0.01));
        // Natural Earth splits the land at ±180°.
        assert!(near((180.0, -16.067133), 0.01));
        assert!(near((-180.0, -84.71338), 0.01));
        // The land reaches the south pole (Antarctica) and the east and
        // west edges.
        let south = world
            .iter()
            .flatten()
            .map(|p| p.1)
            .fold(f64::INFINITY, f64::min);
        assert!((south + 90.0).abs() < 0.01, "{south}");
    }

    #[test]
    fn rings_survive_encoding() {
        // The format of convert.py, written by hand: two rings.
        let mut bytes = MAGIC.to_vec();
        let varint = |mut v: u64, out: &mut Vec<u8>| {
            while v >= 0x80 {
                out.push((v as u8 & 0x7f) | 0x80);
                v >>= 7;
            }
            out.push(v as u8);
        };
        let zigzag = |v: i64| ((v << 1) ^ (v >> 63)) as u64;
        let rings: [&[(i64, i64)]; 2] = [
            &[(0, 0), (32767, 0), (32767, 32767)],
            &[(-100, -200), (-32767, 5), (300, -32767), (12, 13)],
        ];
        varint(rings.len() as u64, &mut bytes);
        for ring in rings {
            varint(ring.len() as u64, &mut bytes);
            let (mut px, mut py) = (0, 0);
            for &(x, y) in ring {
                varint(zigzag(x - px), &mut bytes);
                varint(zigzag(y - py), &mut bytes);
                (px, py) = (x, y);
            }
        }
        let decoded = decode(&bytes).unwrap();
        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded[0], vec![(0.0, 0.0), (180.0, 0.0), (180.0, 90.0)]);
        assert_eq!(decoded[1].len(), 4);
        assert_eq!(decoded[1][1], (-180.0, 5.0 / 32767.0 * 90.0));
        assert_eq!(decoded[1][2], (300.0 / 32767.0 * 180.0, -90.0));
        assert!(decode(b"PNG").is_err());
        assert!(decode(&bytes[..bytes.len() - 1]).is_err());
    }

    fn close(a: (f64, f64), b: (f64, f64), tolerance: f64) -> bool {
        (a.0 - b.0).abs() <= tolerance && (a.1 - b.1).abs() <= tolerance
    }

    #[test]
    fn equal_earth_matches_published_values() {
        // Šavrič, Patterson and Jenny (2018), Int. J. Geogr. Inf. Sci. 33:
        // the map is 2.70663 units from the centre to each end of the
        // equator and 1.31736 to each pole, a width-to-height ratio of
        // about 2.05.
        let p = Projection::EqualEarth;
        assert!(close(p.forward(180.0, 0.0), (2.706_629_7, 0.0), 1e-6));
        assert!(close(p.forward(0.0, 90.0), (0.0, 1.317_362_7), 1e-6));
        assert!(close(p.forward(-180.0, -45.0), (-2.4, -0.6), 1.0));
        assert!(p.forward(-180.0, -45.0).0 < 0.0 && p.forward(-180.0, -45.0).1 < 0.0);
        // Equal areas: a 1° cell at the equator and at 60° N have areas in
        // the ratio of the sphere's (cos 60° roughly, exactly the ratio of
        // the sines).
        let cell = |lat: f64| {
            let (x0, y0) = p.forward(0.0, lat);
            let (x1, y1) = p.forward(1.0, lat + 1.0);
            let (x2, _) = p.forward(1.0, lat);
            let (x3, _) = p.forward(0.0, lat + 1.0);
            // A trapezoid: parallels are straight and horizontal.
            ((x2 - x0) + (x1 - x3)) / 2.0 * (y1 - y0)
        };
        let sphere = |lat: f64| (lat + 1.0f64).to_radians().sin() - lat.to_radians().sin();
        let ratio = cell(60.0) / cell(0.0);
        assert!((ratio - sphere(60.0) / sphere(0.0)).abs() < 1e-4, "{ratio}");
    }

    #[test]
    fn robinson_follows_its_table() {
        let p = Projection::Robinson;
        let pi = std::f64::consts::PI;
        // At the rows of the table the projection is exact.
        assert!(close(p.forward(180.0, 0.0), (0.8487 * pi, 0.0), 1e-12));
        assert!(close(
            p.forward(180.0, 90.0),
            (0.8487 * 0.5322 * pi, 1.3523),
            1e-12
        ));
        assert!(close(
            p.forward(-90.0, -45.0),
            (-0.8487 * 0.8962 * pi / 2.0, -1.3523 * 0.5571),
            1e-12
        ));
        // Between rows it lies between them.
        let (x, y) = p.forward(180.0, 47.5);
        assert!(x < 0.8487 * 0.8962 * pi && x > 0.8487 * 0.8679 * pi);
        assert!(y > 1.3523 * 0.5571 && y < 1.3523 * 0.6176);
    }

    #[test]
    fn cylindrical_projections() {
        let pi = std::f64::consts::PI;
        assert!(close(
            Projection::Equirectangular.forward(90.0, -45.0),
            (pi / 2.0, -pi / 4.0),
            1e-12
        ));
        // Mercator: y = ln tan(45° + φ/2); 85° is the limit.
        let (_, y) = Projection::Mercator.forward(0.0, 60.0);
        assert!((y - 1.316_957_9).abs() < 1e-6);
        assert_eq!(
            Projection::Mercator.forward(10.0, 89.0),
            Projection::Mercator.forward(10.0, 85.0)
        );
        assert_eq!(Projection::Mercator.lat_limits(), (-85.0, 85.0));
    }

    #[test]
    fn clipping_a_ring_that_crosses_the_box() {
        // A square from (0, 0) to (10, 10) against a box from x = 5.
        let ring = mark_seams(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]);
        let rect = Rect {
            left: 5.0,
            right: 20.0,
            bottom: -5.0,
            top: 20.0,
        };
        let clipped = clip_ring(&ring, &rect);
        let points: Vec<(f64, f64)> = clipped.iter().map(|v| v.0).collect();
        assert_eq!(points.len(), 4);
        for p in [(5.0, 0.0), (10.0, 0.0), (10.0, 10.0), (5.0, 10.0)] {
            assert!(points.contains(&p), "{points:?}");
        }
        // The edge along the box's side is filled but not drawn.
        let edges = drawn_edges(&clipped);
        assert_eq!(
            edges,
            vec![vec![(5.0, 0.0), (10.0, 0.0), (10.0, 10.0), (5.0, 10.0)]]
        );
        // Outside the box entirely, or inside it entirely.
        let away = Rect {
            left: 30.0,
            right: 40.0,
            bottom: 0.0,
            top: 1.0,
        };
        assert!(clip_ring(&ring, &away).is_empty());
        let around = Rect {
            left: -1.0,
            right: 11.0,
            bottom: -1.0,
            top: 11.0,
        };
        assert_eq!(clip_ring(&ring, &around), ring);
        assert_eq!(drawn_edges(&ring)[0].len(), 5);
    }

    #[test]
    fn seams_of_natural_earth_are_not_drawn() {
        // A strip of Antarctica: the edge along -90° and the cut at 180°.
        let ring = mark_seams(&[
            (170.0, -80.0),
            (180.0, -80.0),
            (180.0, -90.0),
            (170.0, -90.0),
        ]);
        let edges = drawn_edges(&ring);
        assert_eq!(
            edges,
            vec![vec![(170.0, -90.0), (170.0, -80.0), (180.0, -80.0)]]
        );
    }

    #[test]
    fn lines_are_clipped_into_pieces() {
        let rect = Rect {
            left: 0.0,
            right: 10.0,
            bottom: 0.0,
            top: 10.0,
        };
        let pieces = clip_line(&[(-5.0, 5.0), (5.0, 5.0), (15.0, 5.0)], &rect);
        assert_eq!(pieces, vec![vec![(0.0, 5.0), (5.0, 5.0), (10.0, 5.0)]]);
        let pieces = clip_line(&[(5.0, 5.0), (5.0, 15.0), (6.0, 15.0), (6.0, 5.0)], &rect);
        assert_eq!(
            pieces,
            vec![vec![(5.0, 5.0), (5.0, 10.0)], vec![(6.0, 10.0), (6.0, 5.0)]]
        );
        assert!(clip_line(&[(20.0, 20.0), (30.0, 30.0)], &rect).is_empty());
        let dense = densify(&mark_seams(&[(0.0, 0.0), (10.0, 0.0), (10.0, 1.0)]), 2.0);
        assert_eq!(dense.len(), 3 + 4 + 4);
    }

    #[test]
    fn resolution_follows_the_extent() {
        assert_eq!(Resolution::Auto.for_span(360.0), Resolution::Low);
        assert_eq!(Resolution::Auto.for_span(60.0), Resolution::Medium);
        assert_eq!(Resolution::Low.for_span(10.0), Resolution::Low);
    }
}
