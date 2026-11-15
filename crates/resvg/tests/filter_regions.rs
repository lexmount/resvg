use resvg::{tiny_skia, usvg};

fn parse(body: &str) -> usvg::Tree {
    let svg =
        format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64">{body}</svg>"#);
    usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap()
}

#[test]
fn object_bbox_filter_regions_reject_numeric_overflow_and_underflow() {
    for (filter, rect) in [
        (r#"x="-736" width="784""#, r#"width="1e38" height="100""#),
        (r#"x="736" width="784""#, r#"width="1e38" height="100""#),
        (
            r#"x="1" width="1""#,
            r#"x="2e38" width="1e38" height="100""#,
        ),
        (
            r#"x="0" y="0" width="1e-30" height="1""#,
            r#"width="1e-20" height="100""#,
        ),
    ] {
        let body = format!(
            r#"<defs><filter id="f" {filter}><feGaussianBlur stdDeviation="1"/></filter></defs><rect {rect} filter="url(#f)"/>"#
        );
        let tree = parse(&body);
        assert!(
            !tree.root().has_children(),
            "unrepresentable filter region must follow invalid-filter handling: {body}"
        );
    }
}

#[test]
fn primitive_bbox_regions_reject_overflow() {
    for body in [
        r#"<defs><filter id="f" filterUnits="userSpaceOnUse" primitiveUnits="objectBoundingBox" x="0" y="0" width="64" height="64"><feFlood x="-736" width="784" flood-color="blue"/></filter></defs><rect width="1e38" height="100" filter="url(#f)"/>"#,
        r#"<defs><filter id="f" filterUnits="userSpaceOnUse" primitiveUnits="objectBoundingBox" x="0" y="0" width="1e38" height="100"><feOffset x="0" width="784"/></filter></defs><rect width="20" height="20" filter="url(#f)"/>"#,
    ] {
        let tree = parse(body);
        assert!(
            !tree.root().has_children(),
            "invalid primitive region must not panic: {body}"
        );
    }
}

#[test]
fn css_filter_functions_handle_unrepresentable_mapped_regions() {
    let tree = parse(r#"<rect width="3e38" height="20" fill="red" filter="opacity(0.5)"/>"#);
    assert!(
        tree.root().has_children(),
        "invalid function region keeps the existing skip-function behavior"
    );
}

#[test]
fn finite_object_bbox_filters_still_affect_rendered_pixels() {
    let tree = parse(
        r#"<defs><filter id="f" x="0" y="0" width="1" height="1"><feFlood flood-color="blue"/></filter></defs><rect x="10" y="12" width="20" height="24" fill="red" filter="url(#f)"/>"#,
    );
    let mut pixmap = tiny_skia::Pixmap::new(64, 64).unwrap();
    resvg::render(
        &tree,
        tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    let pixel = pixmap.pixel(15, 15).unwrap();
    assert_eq!(
        (pixel.red(), pixel.green(), pixel.blue(), pixel.alpha()),
        (0, 0, 255, 255)
    );
    assert_eq!(pixmap.pixel(5, 5).unwrap().alpha(), 0);
}
