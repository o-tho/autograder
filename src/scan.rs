use crate::point::{find_circle, Point};
use image::{GrayImage, Luma};
use std::cmp::{max, min};

#[derive(Debug, Clone)]
pub struct Scan {
    pub image: GrayImage,
}

fn find_inner_boundary_points(
    c: Point,
    r: u32,
    image: &GrayImage,
    min_count: u32,
) -> Option<[Point; 3]> {
    let mut points = [Point { x: 0, y: 0 }; 3]; // Array to hold found points
    let (width, height) = image.dimensions();

    // Define direction vectors
    let directions = [
        (-1, -1), // up left
        (-1, 1),  // down left
        (1, 1),   // Down-right
    ];

    for (i, (dx, dy)) in directions.iter().enumerate() {
        let mut found_point: Option<Point> = None;
        let mut consecutive_black_pixels = 0;

        // Search in the specified direction
        for distance in 1..=(2 * r + min_count) {
            let new_x = (c.x as i32 + dx * distance as i32) as u32;
            let new_y = (c.y as i32 + dy * distance as i32) as u32;

            // Check if the new point is within the bounds of the image
            if new_x >= width || new_y >= height {
                break; // Out of bounds
            }

            // Get the pixel color
            let pixel = image.get_pixel(new_x, new_y);
            // Check if the pixel is "dark" (you may want to adjust this threshold)
            if is_dark(pixel) {
                consecutive_black_pixels += 1; // Increment count for consecutive dark pixels
                if found_point.is_none() {
                    found_point = Some(Point { x: new_x, y: new_y }); // Update found point
                }
            } else {
                // If a non-dark pixel is encountered, reset the count
                consecutive_black_pixels = 0;
                found_point = None;
            }

            // If we have found enough consecutive black pixels, we can stop
            if consecutive_black_pixels >= min_count {
                break; // Break once we have enough black pixels
            }
        }

        // If we found a valid point after checking the pixels
        if let Some(point) = found_point {
            points[i] = point; // Store the found point
        } else {
            return None; // Return None if any point is not found
        }
    }

    Some(points) // Return the array of found points
}

fn is_dark(pixel: &Luma<u8>) -> bool {
    pixel[0] == 0
}
impl Scan {
    fn blackness_around(&self, p: Point, r: u32) -> f64 {
        self.blackness(
            Point {
                x: p.x.saturating_sub(r),
                y: p.y.saturating_sub(r),
            },
            Point {
                x: p.x + r,
                y: p.y + r,
            },
        )
    }
    pub fn blackness(&self, p1: Point, p2: Point) -> f64 {
        let mut dark_pixels = 0;
        let mut total_pixels = 0;

        let x_min = min(p1.x, p2.x);
        let x_max = max(p1.x, p2.x);
        let y_min = min(p1.y, p2.y);
        let y_max = max(p1.y, p2.y);

        // Calculate ellipse center and radii
        let center_x = (x_min + x_max) as f64 / 2.0;
        let center_y = (y_min + y_max) as f64 / 2.0;
        let a = (x_max - x_min) as f64 / 2.0; // semi-major axis
        let b = (y_max - y_min) as f64 / 2.0; // semi-minor axis

        let (w, h) = self.image.dimensions();

        for x in x_min..x_max {
            for y in y_min..y_max {
                // Check if point (x,y) lies within the ellipse using the equation:
                // ((x-h)²/a²) + ((y-k)²/b²) ≤ 1
                // where (h,k) is the center point
                let normalized_x = (x as f64 - center_x) / a;
                let normalized_y = (y as f64 - center_y) / b;
                let in_ellipse = (normalized_x * normalized_x + normalized_y * normalized_y) <= 1.0;

                if in_ellipse && x < w && y < h {
                    let pixel = self.image.get_pixel(x, y);
                    if is_dark(pixel) {
                        dark_pixels += 1;
                    }
                    total_pixels += 1;
                }
            }
        }

        if total_pixels == 0 {
            0.0
        } else {
            (dark_pixels as f64) / (total_pixels as f64)
        }
    }

    fn find_white_spot_from_annulus(&self, start: Point, inner_radius: u32) -> Vec<Point> {
        let mut points = Vec::new();

        let topleft = Point {
            x: start.x.saturating_sub(4 * inner_radius / 3),
            y: start.y.saturating_sub(4 * inner_radius / 3),
        };
        let botright = Point {
            x: start.x + 4 * inner_radius / 3,
            y: start.y + 4 * inner_radius / 3,
        };

        // top line
        for x_new in topleft.x..botright.x {
            let newpoint = Point {
                x: x_new,
                y: topleft.y,
            };
            if self.blackness_around(newpoint, inner_radius / 10) < 0.01 {
                points.push(newpoint);
                break;
            }
        }

        // bottom line
        for x_new in topleft.x..botright.x {
            let newpoint = Point {
                x: x_new,
                y: botright.y,
            };
            if self.blackness_around(newpoint, inner_radius / 10) < 0.01 {
                points.push(newpoint);
                break;
            }
        }

        // left line
        for y_new in topleft.y..botright.y {
            let newpoint = Point {
                x: topleft.x,
                y: y_new,
            };
            if self.blackness_around(newpoint, inner_radius / 10) < 0.01 {
                points.push(newpoint);
                break;
            }
        }

        // right line
        for y_new in topleft.y..botright.y {
            let newpoint = Point {
                x: botright.x,
                y: y_new,
            };
            if self.blackness_around(newpoint, inner_radius / 10) < 0.1 {
                points.push(newpoint);
                break;
            }
        }

        points
    }
    #[cfg(target_arch = "wasm32")]
    pub fn real_centers_with_radius(
        &self,
        approx_centers: [Point; 3],
        approx_radius: u32,
    ) -> Option<([Point; 3], u32)> {
        let max_radius = ((approx_radius as f64) * 1.05).round() as u32;

        let real_centers: Vec<Point> = approx_centers
            .iter()
            .map(|p| self.real_center_fuzzy(*p, max_radius))
            .collect::<Option<Vec<Point>>>()?;

        let real_radii: Vec<f64> = real_centers
            .iter()
            .map(|c| {
                let boundary_points =
                    find_inner_boundary_points(*c, max_radius, &self.image, 3).unwrap();
                let distances = boundary_points.map(|p| c.distance(p) as f64);
                distances.iter().sum::<f64>() / 3.0
            })
            .collect();

        let average_radius = real_radii.iter().sum::<f64>() / real_radii.len() as f64;

        Some((
            [real_centers[0], real_centers[1], real_centers[2]],
            average_radius.round() as u32,
        ))
    }

    pub fn is_circle_center(&self, center: Point, radius: u32) -> bool {
        let directions: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

        !directions.iter().any(|d| {
            let x = center.x as i32 + d.0 * (5 * radius / 3) as i32;
            let y = center.y as i32 + d.1 * (5 * radius / 3) as i32;

            let p = Point {
                x: x as u32,
                y: y as u32,
            };

            self.blackness_around(p, radius / 5) < 0.6
        })
    }

    pub fn real_center_fuzzy(&self, approx_center: Point, max_radius: u32) -> Option<Point> {
        let fixes: [i32; 5] = [0, 1, -1, 2, -2];
        let real_centers: Option<Point> = fixes.iter().find_map(|&fix| {
            let p = approx_center;
            let y = p.y as i32 + fix * max_radius as i32;
            let y = y.max(0) as u32;
            self.real_center(Point { x: p.x, y }, max_radius)
        });
        real_centers
    }

    pub fn real_center(&self, approx_center: Point, max_radius: u32) -> Option<Point> {
        let a = Point {
            x: approx_center.x - max_radius / 4,
            y: approx_center.y - max_radius / 4,
        };
        let b = Point {
            x: approx_center.x + max_radius / 4,
            y: approx_center.y + max_radius / 4,
        };

        if self.blackness(a, b) >= 0.4 {
            // we actually are not inside the circle center but on the disc
            let points = self.find_white_spot_from_annulus(approx_center, max_radius);
            for p in points {
                let res = find_inner_boundary_points(p, max_radius, &self.image, max_radius / 4);
                if let Some(points) = res {
                    if let Some((center, radius)) = find_circle(points[0], points[1], points[2]) {
                        return match self.is_circle_center(center, radius) {
                            true => Some(center),
                            false => None,
                        };
                    }
                }
            }
            return None;
        }

        match find_inner_boundary_points(approx_center, max_radius, &self.image, max_radius / 10) {
            Some(points) => {
                if let Some((center, radius)) = find_circle(points[0], points[1], points[2]) {
                    match self.is_circle_center(center, radius) {
                        true => Some(center),
                        false => None,
                    }
                } else {
                    None
                }
            }
            None => None,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::image_helpers::binary_image_from_file;

    #[test]
    fn image_circle_center_easy() {
        let image = binary_image_from_file(&"tests/assets/c-47-47.png".to_string());
        let scan = Scan { image };

        let real_center = Point { x: 47, y: 47 };
        let test_center = Point { x: 40, y: 60 };
        let real_radius = 30;

        let res = scan
            .real_center(test_center, real_radius)
            .expect("could not find a center");

        assert!(real_center.distance(res) < 2);
    }

    #[test]
    fn circles_in_sample_bubblesheet() {
        let image = binary_image_from_file(&"tests/assets/example-ahmed.png".to_string());
        let scan = Scan { image };

        let real_centers = [
            Point { x: 173, y: 203 },
            Point { x: 1474, y: 204 },
            Point { x: 168, y: 2100 },
            Point { x: 1470, y: 2101 },
        ];
        let test_centers = [
            Point { x: 186, y: 210 },
            Point { x: 1461, y: 183 },
            Point { x: 212, y: 2073 },
            Point { x: 1481, y: 2099 },
        ];
        let real_radius = 35;

        for i in 0..3 {
            let res = scan
                .real_center(test_centers[i], real_radius)
                .expect("could not find a center");
            println!(
                "{} vs {} has distance {}",
                real_centers[i],
                res,
                real_centers[i].distance(res)
            );
            assert!(real_centers[i].distance(res) < 4);
        }
    }
}
