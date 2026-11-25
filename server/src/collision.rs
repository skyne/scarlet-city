use crate::protocol::Obstacle;

/// Vertical slabs collide only when both 2D footprints overlap **and**
/// height intervals `[z_min, z_max]` overlap.
pub fn intervals_overlap(a_min: f32, a_max: f32, b_min: f32, b_max: f32) -> bool {
    a_min < b_max && b_min < a_max
}

pub fn circle_hits_aabb(cx: f32, cy: f32, radius: f32, x: f32, y: f32, w: f32, d: f32) -> bool {
    let nearest_x = cx.clamp(x, x + w);
    let nearest_y = cy.clamp(y, y + d);
    let dx = cx - nearest_x;
    let dy = cy - nearest_y;
    dx * dx + dy * dy < radius * radius
}

pub fn volume_hits_obstacle(
    x: f32,
    y: f32,
    radius: f32,
    z_min: f32,
    z_max: f32,
    obs: &Obstacle,
) -> bool {
    intervals_overlap(z_min, z_max, obs.z_min, obs.z_max)
        && circle_hits_aabb(x, y, radius, obs.x, obs.y, obs.width, obs.depth)
}

pub fn blocked_by_map(
    x: f32,
    y: f32,
    radius: f32,
    z_min: f32,
    z_max: f32,
    obstacles: &[Obstacle],
) -> bool {
    obstacles
        .iter()
        .any(|o| volume_hits_obstacle(x, y, radius, z_min, z_max, o))
}

/// 2D segment vs AABB. Returns the entry and exit parameters on the segment.
pub fn segment_aabb_span(
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    x: f32,
    y: f32,
    w: f32,
    d: f32,
) -> Option<(f32, f32)> {
    let min_x = x;
    let min_y = y;
    let max_x = x + w;
    let max_y = y + d;

    let mut t0 = 0.0_f32;
    let mut t1 = 1.0_f32;
    let dx = x1 - x0;
    let dy = y1 - y0;

    let clip = |t0: &mut f32, t1: &mut f32, p: f32, q: f32| -> bool {
        if p.abs() < f32::EPSILON {
            return q >= 0.0;
        }
        let r = q / p;
        if p < 0.0 {
            if r > *t1 {
                return false;
            }
            if r > *t0 {
                *t0 = r;
            }
        } else {
            if r < *t0 {
                return false;
            }
            if r < *t1 {
                *t1 = r;
            }
        }
        true
    };

    if clip(&mut t0, &mut t1, -dx, x0 - min_x)
        && clip(&mut t0, &mut t1, dx, max_x - x0)
        && clip(&mut t0, &mut t1, -dy, y0 - min_y)
        && clip(&mut t0, &mut t1, dy, max_y - y0)
        && t0 <= t1
    {
        Some((t0, t1))
    } else {
        None
    }
}

/// 2D segment vs AABB, used with a height band for altitude-aware LOS.
pub fn segment_hits_aabb(
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    x: f32,
    y: f32,
    w: f32,
    d: f32,
) -> bool {
    segment_aabb_span(x0, y0, x1, y1, x, y, w, d).is_some()
}

/// Parameter span where `z(t)` lies in `[z_min, z_max]`. Unbounded when `z` is constant.
fn altitude_span(z0: f32, z1: f32, z_min: f32, z_max: f32) -> Option<(f32, f32)> {
    let dz = z1 - z0;
    if dz.abs() <= 1.0e-5 {
        if z0 >= z_min && z0 <= z_max {
            Some((f32::NEG_INFINITY, f32::INFINITY))
        } else {
            None
        }
    } else {
        let mut ta = (z_min - z0) / dz;
        let mut tb = (z_max - z0) / dz;
        if ta > tb {
            std::mem::swap(&mut ta, &mut tb);
        }
        Some((ta, tb))
    }
}

/// Earliest `t` in `[0, 1]` where the horizontal span and the altitude span overlap.
fn earliest_t(horizontal: Option<(f32, f32)>, vertical: Option<(f32, f32)>) -> Option<f32> {
    let (h0, h1) = horizontal?;
    let (v0, v1) = vertical?;
    if h1 < 0.0 || h0 > 1.0 || v1 < 0.0 || v0 > 1.0 {
        return None;
    }
    let t0 = h0.max(v0).max(0.0);
    let t1 = h1.min(v1).min(1.0);
    if t0 <= t1 + 1.0e-4 {
        Some(t0.clamp(0.0, 1.0))
    } else {
        None
    }
}

/// Entry `t` along the segment if it meets the obstacle footprint while `z(t)` is inside `[z_min, z_max]`.
pub fn segment_obstacle_t(
    x0: f32,
    y0: f32,
    z0: f32,
    x1: f32,
    y1: f32,
    z1: f32,
    obs: &Obstacle,
) -> Option<f32> {
    earliest_t(
        segment_aabb_span(x0, y0, x1, y1, obs.x, obs.y, obs.width, obs.depth),
        altitude_span(z0, z1, obs.z_min, obs.z_max),
    )
}

/// 2D segment vs circle. Returns entry and exit parameters (may lie outside `[0, 1]`).
pub fn segment_circle_span(
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    cx: f32,
    cy: f32,
    radius: f32,
) -> Option<(f32, f32)> {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let fx = x0 - cx;
    let fy = y0 - cy;
    let a = dx * dx + dy * dy;
    let b = 2.0 * (fx * dx + fy * dy);
    let c = fx * fx + fy * fy - radius * radius;
    if a <= 1.0e-8 {
        return if c <= 0.0 { Some((0.0, 1.0)) } else { None };
    }
    let disc = b * b - 4.0 * a * c;
    if disc < 0.0 {
        return None;
    }
    let s = disc.sqrt();
    let inv = 0.5 / a;
    Some(((-b - s) * inv, (-b + s) * inv))
}

/// Entry `t` where the segment meets a vertical cylinder.
pub fn segment_cylinder_t(
    x0: f32,
    y0: f32,
    z0: f32,
    x1: f32,
    y1: f32,
    z1: f32,
    cx: f32,
    cy: f32,
    radius: f32,
    z_min: f32,
    z_max: f32,
) -> Option<f32> {
    earliest_t(
        segment_circle_span(x0, y0, x1, y1, cx, cy, radius),
        altitude_span(z0, z1, z_min, z_max),
    )
}

/// 2D segment vs extruded obstacles, clipped to a height band.
#[allow(dead_code)]
pub fn los_blocked(
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    z_min: f32,
    z_max: f32,
    obstacles: &[Obstacle],
) -> bool {
    obstacles.iter().any(|o| {
        intervals_overlap(z_min, z_max, o.z_min, o.z_max)
            && segment_hits_aabb(x0, y0, x1, y1, o.x, o.y, o.width, o.depth)
    })
}

const EYE_HEIGHT: f32 = 1.7;

/// 2.5D sightline: a 2D ray vs extruded AABB. If the observer's eye (`z + 1.7`)
/// **or** the target's altitude is above an obstacle's `z_max`, that volume
/// does not block (flyers see over sheds; rooftop targets peek over low walls).
pub fn sightline_blocked(
    ox: f32,
    oy: f32,
    oz: f32,
    tx: f32,
    ty: f32,
    tz: f32,
    obstacles: &[Obstacle],
) -> bool {
    let eye_z = oz + EYE_HEIGHT;
    obstacles.iter().any(|o| {
        if eye_z > o.z_max || tz > o.z_max {
            return false;
        }
        segment_hits_aabb(ox, oy, tx, ty, o.x, o.y, o.width, o.depth)
    })
}

pub fn in_vision_radius(ox: f32, oy: f32, tx: f32, ty: f32, radius: f32) -> bool {
    let dx = tx - ox;
    let dy = ty - oy;
    dx * dx + dy * dy <= radius * radius
}

pub fn point_in_rect(px: f32, py: f32, x: f32, y: f32, w: f32, d: f32) -> bool {
    px >= x && px <= x + w && py >= y && py <= y + d
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Obstacle;

    fn cube(z_min: f32, z_max: f32) -> Obstacle {
        Obstacle {
            id: "t".into(),
            x: 0.0,
            y: 0.0,
            width: 10.0,
            depth: 10.0,
            z_min,
            z_max,
        }
    }

    #[test]
    fn flyer_clears_low_roof() {
        let shed = cube(0.0, 3.0);
        assert!(volume_hits_obstacle(5.0, 5.0, 1.0, 0.0, 1.8, &shed));
        assert!(!volume_hits_obstacle(5.0, 5.0, 1.0, 4.0, 5.2, &shed));
    }

    #[test]
    fn under_bridge_is_open() {
        let deck = cube(4.0, 5.5);
        assert!(!volume_hits_obstacle(5.0, 5.0, 0.45, 0.0, 1.8, &deck));
        assert!(volume_hits_obstacle(5.0, 5.0, 1.0, 4.2, 5.4, &deck));
    }

    #[test]
    fn ground_los_blocked_by_cube() {
        let wall = cube(0.0, 6.0);
        assert!(sightline_blocked(-2.0, 5.0, 0.0, 12.0, 5.0, 0.0, &[wall.clone()]));
    }

    #[test]
    fn flyer_sees_over_low_building() {
        let shed = cube(0.0, 3.0);
        assert!(!sightline_blocked(-2.0, 5.0, 4.0, 12.0, 5.0, 0.0, &[shed.clone()]));
        assert!(!sightline_blocked(-2.0, 5.0, 0.0, 12.0, 5.0, 4.0, &[shed]));
    }

    #[test]
    fn street_shot_enters_shed_footprint() {
        let shed = cube(0.0, 3.0);
        let t = segment_obstacle_t(-2.0, 5.0, 1.0, 12.0, 5.0, 1.0, &shed).unwrap();
        assert!((t - (2.0 / 14.0)).abs() < 1.0e-3, "t={t}");
    }

    #[test]
    fn shot_above_roof_misses_shed() {
        let shed = cube(0.0, 3.0);
        assert!(segment_obstacle_t(-2.0, 5.0, 6.0, 12.0, 5.0, 6.0, &shed).is_none());
    }

    #[test]
    fn cylinder_entry_is_before_center() {
        let t = segment_cylinder_t(10.0, 10.0, 1.0, 30.0, 10.0, 1.0, 15.0, 10.0, 0.45, 0.0, 1.8)
            .unwrap();
        let x = 10.0 + (30.0 - 10.0) * t;
        assert!((x - (15.0 - 0.45)).abs() < 1.0e-3, "x={x}");
    }
}
