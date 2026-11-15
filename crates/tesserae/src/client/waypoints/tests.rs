use tesserae_core::marker::MarkerKind;

use super::*;

fn marks_with(points: &[(&str, f32, f32)]) -> Marks {
    let mut marks = Marks {
        kinds: MarkerKinds {
            kinds: vec![MarkerKind {
                name: "t:home".into(),
                icon: Icon::Home,
                color: [200, 40, 40, 255],
            }],
            death: None,
        },
        ..Marks::default()
    };
    let mut list = Waypoints::default();
    for (name, x, z) in points {
        list.push(name, "t:home", *x, *z, false);
    }
    marks.set_list(list);
    marks
}

fn pixel(pixels: &[u8], width: u32, x: u32, y: u32) -> [u8; 3] {
    let i = ((y * width + x) * 4) as usize;
    [pixels[i], pixels[i + 1], pixels[i + 2]]
}

#[test]
fn every_icon_draws_something_inside_its_cells() {
    for icon in Icon::ALL {
        let (side, pixels) = icon_pixels(icon, [9, 9, 9, 255], 1);
        assert_eq!(side, 7);
        assert!(pixels.chunks(4).any(|p| p[3] == 255), "{icon:?} is blank");
    }
}

#[test]
fn a_minimap_mark_inside_the_square_is_drawn_whole() {
    let marks = marks_with(&[("a", 20.0, 0.0)]);
    let on = minimap_marks(&marks, [0, 0], 100);
    assert_eq!(on.len(), 1);
    assert!(!on[0].dimmed);
    assert_eq!(on[0].at, Vec2::new(60.0, 50.0));
}

#[test]
fn a_minimap_mark_beyond_the_edge_rests_on_it_dimmed_along_its_bearing() {
    let marks = marks_with(&[("far", 2000.0, 0.0)]);
    let on = minimap_marks(&marks, [0, 0], 100);
    assert!(on[0].dimmed);
    assert_eq!(on[0].at.y, 50.0, "still level with the Player");
    assert!(on[0].at.x < 100.0 && on[0].at.x > 90.0, "{:?}", on[0].at);
    let mut pixels = vec![0u8; 100 * 100 * 4];
    paint_marks(&mut pixels, 100, 100, &on);
    let ring = pixel(&pixels, 100, on[0].at.x as u32 + 2, on[0].at.y as u32);
    assert!(
        ring[0] > 0 && ring[0] < 200,
        "dimmed, not full colour: {ring:?}"
    );
}

#[test]
fn the_view_leaves_out_what_is_off_screen() {
    let marks = marks_with(&[("near", 10.0, 0.0), ("far", 5000.0, 0.0)]);
    let on = view_marks(&marks, Vec2::ZERO, 2.0, Vec2::new(640.0, 480.0));
    assert_eq!(on.len(), 1);
}

#[test]
fn a_selected_mark_is_ringed_in_white() {
    let mut marks = marks_with(&[("a", 0.0, 0.0)]);
    marks.select(Some(1));
    let on = view_marks(&marks, Vec2::ZERO, 2.0, Vec2::new(100.0, 100.0));
    let mut pixels = vec![0u8; 100 * 100 * 4];
    paint_marks(&mut pixels, 100, 100, &on);
    assert_eq!(pixel(&pixels, 100, 58, 50), [255, 255, 255]);
}

#[test]
fn an_undeclared_kind_falls_back_to_a_grey_pin() {
    let marks = marks_with(&[]);
    assert_eq!(marks.look("gone:kind"), UNKNOWN_KIND);
}

#[test]
fn the_bearing_runs_clockwise_from_north() {
    let w = |x_m, z_m| Waypoint {
        id: 1,
        name: "w".into(),
        kind: "t:home".into(),
        x_m,
        z_m,
        death: false,
    };
    let (d, north) = from_player(&w(0.0, -30.0), Vec2::ZERO);
    assert_eq!((d, north), (30.0, 0.0));
    assert_eq!(from_player(&w(10.0, 0.0), Vec2::ZERO).1, 90.0);
}

#[test]
fn the_form_never_rests_on_the_death_kind() {
    let mut marks = marks_with(&[]);
    marks.kinds = MarkerKinds {
        kinds: vec![
            MarkerKind {
                name: "t:grave".into(),
                icon: Icon::Death,
                color: [1, 1, 1, 255],
            },
            MarkerKind {
                name: "t:home".into(),
                icon: Icon::Home,
                color: [2, 2, 2, 255],
            },
        ],
        death: Some(0),
    };
    let mut draft = Draft::default();
    marks.settle(&mut draft);
    assert_eq!(draft.kind, 1);
}
