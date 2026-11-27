//! Deterministic polygon geometry in integer micro-degrees (no floats).
//!
//! Coordinates are WGS84 longitude/latitude multiplied by 1 000 000 and
//! rounded by the import tool. All predicates use exact integer arithmetic,
//! so validation gives identical results on every platform.

use std::fmt;

/// Micro-degrees per degree.
pub const MICRO: i32 = 1_000_000;
/// Maximum absolute longitude in micro-degrees.
pub const MAX_LON: i32 = 180 * MICRO;
/// Maximum absolute latitude in micro-degrees.
pub const MAX_LAT: i32 = 90 * MICRO;

/// A position in micro-degrees.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Point {
    /// Longitude in micro-degrees, `-180e6..=180e6`.
    pub lon: i32,
    /// Latitude in micro-degrees, `-90e6..=90e6`.
    pub lat: i32,
}

impl Point {
    /// Creates a point from micro-degrees.
    pub const fn new(lon: i32, lat: i32) -> Self {
        Self { lon, lat }
    }
}

/// A closed linear ring: first point equals last point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ring(pub Vec<Point>);

/// A polygon with one exterior ring and optional holes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Polygon {
    /// Outer boundary.
    pub exterior: Ring,
    /// Holes (enclaves, lakes), each strictly inside the exterior.
    pub holes: Vec<Ring>,
}

/// One or more polygons forming a territory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultiPolygon(pub Vec<Polygon>);

/// Why a geometry is invalid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GeometryError {
    /// The multipolygon contains no polygon.
    Empty,
    /// A ring has fewer than 4 points (3 distinct + closing point).
    TooFewPoints,
    /// First and last point differ.
    NotClosed,
    /// A coordinate is outside the WGS84 range.
    OutOfRange(Point),
    /// Two consecutive points are identical.
    DuplicatePoint(Point),
    /// The ring encloses no area.
    ZeroArea,
    /// Two edges of the ring (or of the ring and another ring) cross or touch.
    SelfIntersection,
    /// A hole is not strictly inside its exterior ring.
    HoleOutside,
}

impl fmt::Display for GeometryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "empty geometry"),
            Self::TooFewPoints => write!(f, "ring has fewer than 4 points"),
            Self::NotClosed => write!(f, "ring is not closed"),
            Self::OutOfRange(p) => write!(f, "coordinate out of range: {p:?}"),
            Self::DuplicatePoint(p) => write!(f, "duplicate consecutive point: {p:?}"),
            Self::ZeroArea => write!(f, "ring has zero area"),
            Self::SelfIntersection => write!(f, "edges intersect"),
            Self::HoleOutside => write!(f, "hole not inside exterior"),
        }
    }
}

/// Sign of the cross product (b - a) x (c - a): 1 left turn, -1 right, 0 collinear.
fn orient(a: Point, b: Point, c: Point) -> i8 {
    let v = (i64::from(b.lon) - i64::from(a.lon)) * (i64::from(c.lat) - i64::from(a.lat))
        - (i64::from(b.lat) - i64::from(a.lat)) * (i64::from(c.lon) - i64::from(a.lon));
    v.signum() as i8
}

/// `true` when `p` lies on the closed segment `a..b`, given that the three are collinear.
fn on_segment(a: Point, b: Point, p: Point) -> bool {
    a.lon.min(b.lon) <= p.lon
        && p.lon <= a.lon.max(b.lon)
        && a.lat.min(b.lat) <= p.lat
        && p.lat <= a.lat.max(b.lat)
}

/// `true` when the closed segments `a..b` and `c..d` share at least one point.
fn segments_touch(a: Point, b: Point, c: Point, d: Point) -> bool {
    let (o1, o2, o3, o4) = (
        orient(a, b, c),
        orient(a, b, d),
        orient(c, d, a),
        orient(c, d, b),
    );
    if o1 != o2 && o3 != o4 && o1 != 0 && o2 != 0 && o3 != 0 && o4 != 0 {
        return true;
    }
    (o1 == 0 && on_segment(a, b, c))
        || (o2 == 0 && on_segment(a, b, d))
        || (o3 == 0 && on_segment(c, d, a))
        || (o4 == 0 && on_segment(c, d, b))
}

/// Axis-aligned bounding boxes of two segments overlap.
fn boxes_overlap(a: Point, b: Point, c: Point, d: Point) -> bool {
    a.lon.min(b.lon) <= c.lon.max(d.lon)
        && c.lon.min(d.lon) <= a.lon.max(b.lon)
        && a.lat.min(b.lat) <= c.lat.max(d.lat)
        && c.lat.min(d.lat) <= a.lat.max(b.lat)
}

impl Ring {
    /// Edges as point pairs (the closing point makes the ring explicit).
    fn edges(&self) -> impl Iterator<Item = (Point, Point)> + '_ {
        self.0.windows(2).map(|w| (w[0], w[1]))
    }

    /// Twice the signed area (positive = counter-clockwise).
    pub fn doubled_signed_area(&self) -> i128 {
        self.edges()
            .map(|(a, b)| {
                i128::from(a.lon) * i128::from(b.lat) - i128::from(b.lon) * i128::from(a.lat)
            })
            .sum()
    }

    /// Checks closure, coordinate range, duplicates, area and simplicity.
    pub fn validate(&self) -> Result<(), GeometryError> {
        let pts = &self.0;
        if pts.len() < 4 {
            return Err(GeometryError::TooFewPoints);
        }
        if pts.first() != pts.last() {
            return Err(GeometryError::NotClosed);
        }
        for p in pts {
            if p.lon.abs() > MAX_LON || p.lat.abs() > MAX_LAT {
                return Err(GeometryError::OutOfRange(*p));
            }
        }
        for w in pts.windows(2) {
            if w[0] == w[1] {
                return Err(GeometryError::DuplicatePoint(w[0]));
            }
        }
        if self.doubled_signed_area() == 0 {
            return Err(GeometryError::ZeroArea);
        }
        let edges: Vec<(Point, Point)> = self.edges().collect();
        let n = edges.len();
        for i in 0..n {
            for j in (i + 1)..n {
                let (a, b) = edges[i];
                let (c, d) = edges[j];
                let adjacent = j == i + 1 || (i == 0 && j == n - 1);
                if adjacent {
                    // Shared vertex is allowed; a reversal (spike) is not.
                    let (p, q, r) = if j == i + 1 { (a, b, d) } else { (c, a, b) };
                    if orient(p, q, r) == 0 && !on_segment(p, r, q) {
                        return Err(GeometryError::SelfIntersection);
                    }
                    continue;
                }
                if boxes_overlap(a, b, c, d) && segments_touch(a, b, c, d) {
                    return Err(GeometryError::SelfIntersection);
                }
            }
        }
        Ok(())
    }

    /// `true` when `p` is strictly inside the ring (not on an edge).
    pub fn contains_strictly(&self, p: Point) -> bool {
        let mut inside = false;
        for (a, b) in self.edges() {
            if orient(a, b, p) == 0 && on_segment(a, b, p) {
                return false;
            }
            // Ray casting to +lon with exact integer comparison.
            if (a.lat > p.lat) != (b.lat > p.lat) {
                let lhs =
                    (i64::from(p.lon) - i64::from(a.lon)) * (i64::from(b.lat) - i64::from(a.lat));
                let rhs =
                    (i64::from(b.lon) - i64::from(a.lon)) * (i64::from(p.lat) - i64::from(a.lat));
                let crosses = if b.lat > a.lat { lhs < rhs } else { lhs > rhs };
                if crosses {
                    inside = !inside;
                }
            }
        }
        inside
    }

    fn touches(&self, other: &Ring) -> bool {
        self.edges().any(|(a, b)| {
            other
                .edges()
                .any(|(c, d)| boxes_overlap(a, b, c, d) && segments_touch(a, b, c, d))
        })
    }
}

impl Polygon {
    /// Validates all rings and that every hole lies strictly inside the exterior.
    pub fn validate(&self) -> Result<(), GeometryError> {
        self.exterior.validate()?;
        for (k, hole) in self.holes.iter().enumerate() {
            hole.validate()?;
            if hole.touches(&self.exterior) || !self.exterior.contains_strictly(hole.0[0]) {
                return Err(GeometryError::HoleOutside);
            }
            if self.holes[..k].iter().any(|h| h.touches(hole)) {
                return Err(GeometryError::SelfIntersection);
            }
        }
        Ok(())
    }
}

impl MultiPolygon {
    /// Validates every polygon; an empty multipolygon is invalid.
    pub fn validate(&self) -> Result<(), GeometryError> {
        if self.0.is_empty() {
            return Err(GeometryError::Empty);
        }
        self.0.iter().try_for_each(Polygon::validate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ring(coords: &[(i32, i32)]) -> Ring {
        Ring(
            coords
                .iter()
                .map(|&(x, y)| Point::new(x * MICRO, y * MICRO))
                .collect(),
        )
    }

    fn square(x0: i32, y0: i32, s: i32) -> Ring {
        ring(&[
            (x0, y0),
            (x0 + s, y0),
            (x0 + s, y0 + s),
            (x0, y0 + s),
            (x0, y0),
        ])
    }

    #[test]
    fn valid_square() {
        assert_eq!(square(0, 0, 10).validate(), Ok(()));
        assert!(square(0, 0, 10).doubled_signed_area() > 0);
    }

    #[test]
    fn rejects_open_short_and_out_of_range() {
        assert_eq!(
            ring(&[(0, 0), (1, 0), (0, 0)]).validate(),
            Err(GeometryError::TooFewPoints)
        );
        assert_eq!(
            ring(&[(0, 0), (1, 0), (1, 1), (0, 1)]).validate(),
            Err(GeometryError::NotClosed)
        );
        assert!(matches!(
            ring(&[(0, 0), (181, 0), (1, 1), (0, 0)]).validate(),
            Err(GeometryError::OutOfRange(_))
        ));
    }

    #[test]
    fn rejects_duplicate_and_zero_area() {
        assert!(matches!(
            ring(&[(0, 0), (1, 0), (1, 0), (1, 1), (0, 0)]).validate(),
            Err(GeometryError::DuplicatePoint(_))
        ));
        assert_eq!(
            ring(&[(0, 0), (2, 0), (1, 0), (0, 0)]).validate(),
            Err(GeometryError::ZeroArea)
        );
    }

    #[test]
    fn rejects_bow_tie_and_spike() {
        let bow_tie = ring(&[(0, 0), (4, 4), (4, 0), (0, 2), (0, 0)]);
        assert_eq!(bow_tie.validate(), Err(GeometryError::SelfIntersection));
        let spike = ring(&[(0, 0), (4, 0), (4, 4), (4, 6), (4, 5), (0, 4), (0, 0)]);
        assert_eq!(spike.validate(), Err(GeometryError::SelfIntersection));
    }

    #[test]
    fn rejects_ring_touching_itself_at_vertex() {
        // Figure eight sharing vertex (2,2).
        let eight = ring(&[(0, 0), (2, 2), (4, 0), (4, 4), (2, 2), (0, 4), (0, 0)]);
        assert_eq!(eight.validate(), Err(GeometryError::SelfIntersection));
    }

    #[test]
    fn holes_must_be_inside() {
        let ok = Polygon {
            exterior: square(0, 0, 10),
            holes: vec![square(2, 2, 2)],
        };
        assert_eq!(ok.validate(), Ok(()));
        let outside = Polygon {
            exterior: square(0, 0, 10),
            holes: vec![square(20, 20, 2)],
        };
        assert_eq!(outside.validate(), Err(GeometryError::HoleOutside));
        let crossing = Polygon {
            exterior: square(0, 0, 10),
            holes: vec![square(8, 8, 4)],
        };
        assert_eq!(crossing.validate(), Err(GeometryError::HoleOutside));
    }

    #[test]
    fn point_in_ring() {
        let r = square(0, 0, 10);
        assert!(r.contains_strictly(Point::new(5 * MICRO, 5 * MICRO)));
        assert!(
            !r.contains_strictly(Point::new(10 * MICRO, 5 * MICRO)),
            "edge is not inside"
        );
        assert!(!r.contains_strictly(Point::new(11 * MICRO, 5 * MICRO)));
    }

    #[test]
    fn empty_multipolygon_invalid() {
        assert_eq!(MultiPolygon(vec![]).validate(), Err(GeometryError::Empty));
    }
}
