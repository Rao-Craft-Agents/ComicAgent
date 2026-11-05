use comic_core::figure::{extent, place, skeleton};
use comic_core::geom::Vec2;
use comic_core::script::{Facing, Pose};

#[test]
fn every_pose_stands_in_its_box_on_its_feet() {
    for pose in Pose::ALL {
        let s = skeleton(pose);
        let points: Vec<(f32, f32)> = s.lines.iter().flatten().copied().collect();
        for &(x, y) in &points {
            assert!(
                (-0.5..=0.9).contains(&x) && (-0.01..=1.0).contains(&y),
                "{pose:?} {x} {y}"
            );
        }
        let lowest = points.iter().map(|p| p.1).fold(f32::MIN, f32::max);
        assert!(lowest > 0.97, "{pose:?} stands on the ground: {lowest}");
        assert!(s.head.1 - s.radius >= -0.01, "{pose:?}");
        let (lo, hi) = extent(pose);
        assert!(lo <= s.head.0 - s.radius && hi >= s.head.0 + s.radius);
    }
    assert!(skeleton(Pose::HoldingSign).sign.is_some());
    assert!(skeleton(Pose::Sitting).desk_height.is_some());
}

#[test]
fn facing_left_mirrors_about_the_feet() {
    let foot = Vec2::new(100.0, 300.0);
    let right = place(Pose::Pointing, Facing::Right, foot, 160.0);
    let left = place(Pose::Pointing, Facing::Left, foot, 160.0);
    assert!((right.head.x - foot.x + (left.head.x - foot.x)).abs() < 1e-3);
    assert!((right.bounds.right() - foot.x - (foot.x - left.bounds.x)).abs() < 1e-3);
    assert!((right.bounds.bottom() - foot.y).abs() < 1e-3);
    assert!((right.radius - 0.11 * 160.0).abs() < 1e-3);
}
