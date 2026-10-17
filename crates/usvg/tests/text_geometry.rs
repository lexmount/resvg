// Copyright 2026 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![cfg(feature = "text")]

use std::sync::Arc;

fn options() -> usvg::Options<'static> {
    let mut fontdb = usvg::fontdb::Database::new();
    fontdb.load_font_data(include_bytes!("../../resvg/tests/fonts/NotoMono-Regular.ttf").to_vec());
    let font_family = fontdb.faces().next().unwrap().families[0].0.clone();
    usvg::Options {
        font_family,
        fontdb: Arc::new(fontdb),
        ..Default::default()
    }
}

fn text_nodes(tree: &usvg::Tree) -> Vec<&usvg::Text> {
    tree.root()
        .children()
        .iter()
        .filter_map(|node| match node {
            usvg::Node::Text(text) => Some(text.as_ref()),
            _ => None,
        })
        .collect()
}

#[test]
fn source_ranges_distinguish_nested_elements_with_duplicate_ids() {
    let source = "<svg xmlns='http://www.w3.org/2000/svg' width='300' height='100'>\
        <text id='duplicate' x='10' y='30'>A<tspan id='duplicate'>BC</tspan>D</text>\
        <text id='duplicate' x='10' y='60'>EF</text></svg>";
    let document = roxmltree::Document::parse(source).unwrap();
    let source_ids = document
        .descendants()
        .filter(|node| node.is_element() && matches!(node.tag_name().name(), "text" | "tspan"))
        .map(|node| node.id().get())
        .collect::<Vec<_>>();
    let tree = usvg::Tree::from_xmltree(&document, &options()).unwrap();
    let texts = text_nodes(&tree);

    assert_eq!(texts.len(), 2);
    assert_eq!(texts[0].source_node_id(), Some(source_ids[0]));
    assert_eq!(texts[1].source_node_id(), Some(source_ids[2]));
    let ranges = texts[0]
        .chunks()
        .iter()
        .flat_map(|chunk| chunk.spans())
        .map(|span| (span.source_node_id(), span.start()..span.end()))
        .collect::<Vec<_>>();
    assert_eq!(
        ranges,
        [
            (Some(source_ids[0]), 0..1),
            (Some(source_ids[1]), 1..3),
            (Some(source_ids[0]), 3..4),
        ]
    );
}
