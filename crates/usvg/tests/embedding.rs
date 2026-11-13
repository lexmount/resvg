use std::sync::Arc;

use usvg::{ElementStyleOverrides, ElementStyleProperty, ObjectGeometry, Options, Tree};

fn rect(value: &ObjectGeometry) -> [f32; 4] {
    let b = value.bounding_box;
    [b.x(), b.y(), b.width(), b.height()]
}

#[test]
fn final_styles_do_not_change_author_selector_matching() {
    let xml = usvg::roxmltree::Document::parse(r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50"><style>rect[style] { transform: matrix(1,0,0,1,40,0); fill:blue !important }</style><rect width="20" height="20" fill="red"/></svg>"#).unwrap();
    let source = xml
        .descendants()
        .find(|node| node.has_tag_name("rect"))
        .unwrap()
        .id();
    let mut styles = ElementStyleOverrides::new();
    styles.insert(
        source,
        Arc::from([ElementStyleProperty::new("fill", "red")]),
    );
    let mut tree = Tree::from_xmltree(
        &xml,
        &Options {
            element_styles: Some(Arc::new(styles)),
            collect_object_geometry: true,
            ..Options::default()
        },
    )
    .unwrap();
    let value = tree
        .take_object_geometry()
        .into_iter()
        .find(|value| value.source == source)
        .unwrap();
    assert_eq!(rect(&value), [0.0, 0.0, 20.0, 20.0]);
    assert_eq!(value.transform.tx, 0.0);
    let usvg::Node::Path(path) = &tree.root().children()[0] else {
        panic!("path");
    };
    assert!(matches!(
        path.fill().unwrap().paint(),
        usvg::Paint::Color(usvg::Color {
            red: 255,
            green: 0,
            blue: 0
        })
    ));
    assert!(
        !xml.descendants()
            .find(|node| node.id() == source)
            .unwrap()
            .has_attribute("style")
    );
}

#[test]
fn final_styles_override_important_declarations_after_the_cascade() {
    let xml = usvg::roxmltree::Document::parse(r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50"><style>rect { fill:blue !important }</style><rect width="20" height="20"/></svg>"#).unwrap();
    let source = xml
        .descendants()
        .find(|node| node.has_tag_name("rect"))
        .unwrap()
        .id();
    let mut styles = ElementStyleOverrides::new();
    styles.insert(
        source,
        Arc::from([ElementStyleProperty::new("fill", "red")]),
    );
    let tree = Tree::from_xmltree(
        &xml,
        &Options {
            element_styles: Some(Arc::new(styles)),
            ..Options::default()
        },
    )
    .unwrap();
    let usvg::Node::Path(path) = &tree.root().children()[0] else {
        panic!("path");
    };
    assert!(matches!(
        path.fill().unwrap().paint(),
        usvg::Paint::Color(usvg::Color {
            red: 255,
            green: 0,
            blue: 0
        })
    ));
}

#[test]
fn clipped_geometry_survives_for_both_the_shape_and_its_parent() {
    let xml = usvg::roxmltree::Document::parse(r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><defs><clipPath id="empty"/></defs><g id="g" transform="translate(5 7)"><rect id="r" x="10" y="20" width="30" height="40" clip-path="url(#empty)"/></g></svg>"##).unwrap();
    let mut tree = Tree::from_xmltree(
        &xml,
        &Options {
            collect_object_geometry: true,
            ..Options::default()
        },
    )
    .unwrap();
    assert!(
        tree.node_by_id("r").is_none(),
        "painting still discards the clipped shape"
    );
    let values = tree.take_object_geometry();
    for id in ["r", "g"] {
        let source = xml
            .descendants()
            .find(|node| node.attribute("id") == Some(id))
            .unwrap()
            .id();
        let value = values.iter().find(|value| value.source == source).unwrap();
        assert_eq!(rect(value), [10.0, 20.0, 30.0, 40.0]);
        assert_eq!([value.transform.tx, value.transform.ty], [5.0, 7.0]);
    }
}

#[test]
fn nested_viewport_and_duplicate_author_ids_keep_xml_identity() {
    let xml = usvg::roxmltree::Document::parse(r#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100"><svg id="nested" x="10" y="20" width="100" height="80" overflow="visible"><rect id="same" x="2" y="3" width="30" height="40"/><rect id="same" x="40" y="3" width="10" height="40"/></svg></svg>"#).unwrap();
    let mut tree = Tree::from_xmltree(
        &xml,
        &Options {
            collect_object_geometry: true,
            ..Options::default()
        },
    )
    .unwrap();
    let values = tree.take_object_geometry();
    let nodes = xml
        .descendants()
        .filter(|node| node.attribute("id") == Some("same"))
        .collect::<Vec<_>>();
    assert_eq!(nodes.len(), 2);
    for (node, expected) in nodes
        .iter()
        .zip([[2.0, 3.0, 30.0, 40.0], [40.0, 3.0, 10.0, 40.0]])
    {
        let value = values
            .iter()
            .find(|value| value.source == node.id())
            .unwrap();
        assert_eq!(rect(value), expected);
        assert_eq!([value.transform.tx, value.transform.ty], [10.0, 20.0]);
    }
    let nested = xml
        .descendants()
        .find(|node| node.attribute("id") == Some("nested"))
        .unwrap()
        .id();
    assert_eq!(
        rect(values.iter().find(|value| value.source == nested).unwrap()),
        [2.0, 3.0, 48.0, 40.0]
    );
}

#[test]
fn use_instances_do_not_overwrite_the_original_source_geometry() {
    let xml = usvg::roxmltree::Document::parse(r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50"><rect id="shared" width="20" height="20"/><use id="instance" href="#shared" x="30"/></svg>"##).unwrap();
    let mut tree = Tree::from_xmltree(
        &xml,
        &Options {
            collect_object_geometry: true,
            ..Options::default()
        },
    )
    .unwrap();
    let values = tree.take_object_geometry();
    let shared = xml
        .descendants()
        .find(|node| node.attribute("id") == Some("shared"))
        .unwrap()
        .id();
    let original = values
        .iter()
        .filter(|value| value.source == shared)
        .collect::<Vec<_>>();
    assert_eq!(original.len(), 1);
    assert_eq!(original[0].transform.tx, 0.0);
    let instance = xml
        .descendants()
        .find(|node| node.attribute("id") == Some("instance"))
        .unwrap()
        .id();
    let value = values
        .iter()
        .find(|value| value.source == instance)
        .unwrap();
    assert_eq!(
        value.bounding_box.transform(value.transform).unwrap().x(),
        30.0
    );
}
