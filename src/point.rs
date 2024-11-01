use nalgebra::{Matrix2, Matrix3, Vector2};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Point {
    pub x: u32,
    pub y: u32,
}
#[derive(Debug, Clone, Copy)]
pub struct Transformation {
    pub m: Matrix2<f64>,
    pub t: Vector2<f64>,
}
fn round_to_u32(value: f64) -> u32 {
    value.round() as u32
}

// Function to find the center of the circle formed by three points
pub fn find_circle(p1: Point, p2: Point, p3: Point) -> Option<(Point, u32)> {
    let a = Vector2::new(p1.x as f64, p1.y as f64);
    let b = Vector2::new(p2.x as f64, p2.y as f64);
    let c = Vector2::new(p3.x as f64, p3.y as f64);

    let m = Matrix2::new(c.y - b.y, b.y - a.y, b.x - c.x, a.x - b.x).try_inverse()?;

    let st = m * Vector2::new((a.x - c.x) / 2.0, (a.y - c.y) / 2.0);
    let center_x = (a.x + b.x) / 2.0 + st.y * (a.y - b.y);
    let center_y = (a.y + b.y) / 2.0 + st.y * (b.x - a.x);

    let center = Point {
        x: round_to_u32(center_x),
        y: round_to_u32(center_y),
    };
    // Round the coordinates to the nearest u32 and return as a Point
    Some((center, center.distance(p1)))
}

fn point_from_vector(v: Vector2<f64>) -> Point {
    Point {
        x: v.x.round() as u32,
        y: v.y.round() as u32,
    }
}

pub fn affine_transformation(
    a1: Point,
    a2: Point,
    a3: Point,
    b1: Point,
    b2: Point,
    b3: Point,
) -> Option<Transformation> {
    let a_matrix = Matrix3::new(
        a1.x as f64,
        a2.x as f64,
        a3.x as f64,
        a1.y as f64,
        a2.y as f64,
        a3.y as f64,
        1.0,
        1.0,
        1.0,
    );
    let b_matrix = Matrix3::new(
        b1.x as f64,
        b2.x as f64,
        b3.x as f64,
        b1.y as f64,
        b2.y as f64,
        b3.y as f64,
        1.0,
        1.0,
        1.0,
    );

    let augmented_matrix = b_matrix * a_matrix.try_inverse()?;

    let m = augmented_matrix.fixed_view::<2, 2>(0, 0);
    let t = Vector2::new(augmented_matrix[(0, 2)], augmented_matrix[(1, 2)]);

    Some(Transformation { m: m.into(), t })
}

impl Point {
    pub fn distance(self, other: Point) -> u32 {
        let dx = (self.x as i64 - other.x as i64).pow(2);
        let dy = (self.y as i64 - other.y as i64).pow(2);
        let dist = ((dx + dy) as f64).sqrt();

        round_to_u32(dist)
    }

    fn to_vector(self) -> Vector2<f64> {
        Vector2::new(self.x as f64, self.y as f64)
    }
}

impl Transformation {
    pub fn apply(self, p: Point) -> Point {
        let v = p.to_vector();
        let res = self.m * v + self.t;
        point_from_vector(res)
    }
}

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y) // Format the Point as "(x, y)"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn circle_center_computation() {
        let x = Point { x: 4, y: 11 };
        let y = Point { x: 5, y: 12 };
        let z = Point { x: 11, y: 12 };
        let c = Point { x: 8, y: 8 };

        let res = find_circle(x, y, z).expect("could not compute center");
        assert!(c.distance(res.0) < 2);
        assert!(5 == res.1);
    }

    #[test]
    fn circle_center_collinear() {
        let x = Point { x: 1, y: 3 };
        let y = Point { x: 1001, y: 2003 };
        let z = Point { x: 2001, y: 4003 };

        let res = find_circle(x, y, z);
        assert!(res.is_none())
    }

    #[test]
    fn interpolate_affine_trafo() {
        let a: Matrix2<f64> = Matrix2::new(-3.0, 2.0, -7.0, 9.0);
        let b: Vector2<f64> = Vector2::new(2000.0, 4000.0);

        let origs = [
            Point { x: 1, y: 2 },
            Point { x: 18, y: 19 },
            Point { x: 11, y: 11 },
        ];
        let rngs = origs.map(|p| point_from_vector(a * p.to_vector() + b));

        let trafo = affine_transformation(origs[0], origs[1], origs[2], rngs[0], rngs[1], rngs[2])
            .expect("computation failed!");

        let interpolated = origs.map(|p| trafo.apply(p));

        for i in 0..3 {
            assert!(rngs[i].distance(interpolated[i]) < 2);
        }
    }
}
