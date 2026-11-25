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

    clip(&mut t0, &mut t1, -dx, x0 - min_x)
        && clip(&mut t0, &mut t1, dx, max_x - x0)
        && clip(&mut t0, &mut t1, -dy, y0 - min_y)
        && clip(&mut t0, &mut t1, dy, max_y - y0)
}

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
}
