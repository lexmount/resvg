#![cfg(feature = "text")]

use std::sync::Arc;

fn options() -> usvg::Options<'static> {
    let mut fonts = usvg::fontdb::Database::new();
    fonts.load_font_data(include_bytes!("../../resvg/tests/fonts/NotoSans-Regular.ttf").to_vec());
    fonts.load_font_data(
        include_bytes!("../../resvg/tests/fonts/NotoColorEmojiCBDT.subset.ttf").to_vec(),
    );
    usvg::Options {
        fontdb: Arc::new(fonts),
        font_family: "Noto Sans".into(),
        collect_object_geometry: true,
        ..Default::default()
    }
}

#[test]
fn shaped_text_cells_share_element_metrics_and_transforms() {
    for attributes in ["", "transform=\"matrix(2 0 0 2 7 11)\"", "rotate=\"45\""] {
        let source = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="500" height="200"><text id="t" x="20" y="80" font-size="20" {attributes}>An SVG text</text></svg>"#
        );
        let xml = usvg::roxmltree::Document::parse(&source).unwrap();
        let mut tree = usvg::Tree::from_xmltree(&xml, &options()).unwrap();
        let usvg::Node::Text(text) = tree.node_by_id("t").unwrap() else {
            panic!("text");
        };
        let expected = text.abs_bounding_box();
        let geometry = tree.take_object_geometry();
        let cells = geometry
            .iter()
            .filter(|cell| cell.text_range.is_some())
            .collect::<Vec<_>>();
        assert_eq!(cells.len(), 11);
        let bounds = cells
            .iter()
            .map(|cell| cell.bounding_box.transform(cell.transform).unwrap())
            .reduce(|left, right| {
                let x = left.x().min(right.x());
                let y = left.y().min(right.y());
                usvg::Rect::from_xywh(
                    x,
                    y,
                    left.right().max(right.right()) - x,
                    left.bottom().max(right.bottom()) - y,
                )
                .unwrap()
            })
            .unwrap();
        for (actual, expected) in [bounds.x(), bounds.y(), bounds.width(), bounds.height()]
            .into_iter()
            .zip([
                expected.x(),
                expected.y(),
                expected.width(),
                expected.height(),
            ])
        {
            assert!(
                (actual - expected).abs() < 0.001,
                "{attributes}: {bounds:?} != {expected:?}"
            );
        }
    }
}

#[test]
fn original_text_identity_survives_whitespace_utf16_and_tspans() {
    let source = r#"<svg xmlns="http://www.w3.org/2000/svg" width="500" height="100"><text x="20" y="60">  A  😀<tspan>BC</tspan> Z  </text></svg>"#;
    let xml = usvg::roxmltree::Document::parse(source).unwrap();
    let sources = xml
        .descendants()
        .filter(|node| node.is_text())
        .map(|node| node.id())
        .collect::<Vec<_>>();
    let mut tree = usvg::Tree::from_xmltree(&xml, &options()).unwrap();
    let geometry = tree.take_object_geometry();
    let observed = sources
        .iter()
        .map(|source| {
            geometry
                .iter()
                .filter(|cell| cell.source == *source)
                .map(|cell| {
                    let range = cell.text_range.unwrap();
                    (range.start, range.end)
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        observed,
        [
            vec![(2, 3), (3, 5), (5, 7)],
            vec![(0, 1), (1, 2)],
            vec![(0, 1), (1, 2)]
        ]
    );
    assert!(
        geometry
            .iter()
            .filter(|cell| cell.text_range.is_some())
            .all(|cell| cell.bounding_box.width() > 0.0 && cell.bounding_box.height() > 0.0)
    );
}

#[test]
fn paint_only_parse_does_not_retain_source_geometry() {
    let mut options = options();
    options.collect_object_geometry = false;
    let mut tree = usvg::Tree::from_str(r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><text x="10" y="50">abc</text></svg>"#, &options).unwrap();
    assert!(!tree.root().children().is_empty());
    assert!(tree.take_object_geometry().is_empty());
}

#[test]
fn long_text_keeps_the_final_source_offsets_and_unclipped_cell_geometry() {
    let count = 20_000;
    let source = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><text x="20" y="60">{}</text></svg>"#,
        "M".repeat(count)
    );
    let mut tree = usvg::Tree::from_str(&source, &options()).unwrap();
    let cells = tree
        .take_object_geometry()
        .into_iter()
        .filter(|cell| cell.text_range.is_some())
        .collect::<Vec<_>>();
    assert_eq!(cells.len(), count);
    let first = cells.first().unwrap();
    let last = cells.last().unwrap();
    let range = last.text_range.unwrap();
    assert_eq!((range.start, range.end), (count - 1, count));
    assert_eq!(first.source, last.source);
    let first = first.bounding_box.transform(first.transform).unwrap();
    let last = last.bounding_box.transform(last.transform).unwrap();
    assert!(last.x() > first.x() + 100_000.0);
    assert!(last.width() > 0.0 && last.height() > 0.0);
}
