// Copyright 2026 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![cfg(feature = "text")]

use std::sync::Arc;
use usvg::{ResolvedElementStyle, ResolvedStyleProperty as P, ResolvedStyleValue};

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

fn value(property: P, value: &str) -> ResolvedStyleValue {
    ResolvedStyleValue::parse(property, value).unwrap()
}

#[test]
fn host_font_and_fill_are_final_values_without_a_css_override() {
    let source = "<svg xmlns='http://www.w3.org/2000/svg' width='300' height='100'>\
        <style>text {font-size:8px !important;fill:blue !important}</style>\
        <text id='label' y='50' style='font-size:10px !important;fill:red !important;/*'>ABC</text></svg>";
    let xml = usvg::roxmltree::Document::parse(source).unwrap();
    let label = xml
        .descendants()
        .find(|node| node.has_tag_name("text"))
        .unwrap();
    let input = [ResolvedElementStyle {
        source_node_id: label.id().get(),
        values: vec![value(P::FontSize, "32px"), value(P::Fill, "green")],
    }];
    let opt = options();
    let tree = usvg::Tree::from_xmltree_with_resolved_styles(&xml, &opt, &input).unwrap();
    let usvg::Node::Text(text) = tree.node_by_id("label").unwrap() else {
        panic!("text")
    };
    assert_eq!(text.source_node_id(), Some(label.id().get()));
    let span = &text.chunks()[0].spans()[0];
    assert_eq!(span.font_size().get(), 32.0);
    assert!(
        matches!(span.fill().unwrap().paint(), usvg::Paint::Color(color) if *color == usvg::Color::new_rgb(0,128,0))
    );
    let plain = usvg::Tree::from_xmltree(&xml, &opt).unwrap();
    let usvg::Node::Text(plain_text) = plain.node_by_id("label").unwrap() else {
        panic!("text")
    };
    assert_ne!(plain_text.chunks()[0].spans()[0].font_size().get(), 32.0);
    assert!(label.attribute("style").unwrap().contains("fill:red"));
}

#[test]
fn source_styles_do_not_freeze_the_inheritance_of_use_instances() {
    let xml = usvg::roxmltree::Document::parse(
        "<svg xmlns='http://www.w3.org/2000/svg' width='200' height='100'>\
          <defs><text id='source' y='30'>ABC</text></defs>\
          <use href='#source' fill='red' font-size='12'/>\
          <use href='#source' fill='blue' font-size='24'/></svg>",
    )
    .unwrap();
    let source_id = xml
        .descendants()
        .find(|node| node.has_tag_name("text"))
        .unwrap()
        .id()
        .get();
    let tree = usvg::Tree::from_xmltree_with_resolved_styles(
        &xml,
        &options(),
        &[ResolvedElementStyle {
            source_node_id: source_id,
            values: vec![value(P::FontSize, "99px"), value(P::Fill, "green")],
        }],
    )
    .unwrap();
    fn visit(group: &usvg::Group, styles: &mut Vec<(f32, usvg::Color)>) {
        for node in group.children() {
            match node {
                usvg::Node::Group(group) => visit(group, styles),
                usvg::Node::Text(text) => {
                    let span = &text.chunks()[0].spans()[0];
                    let usvg::Paint::Color(color) = span.fill().unwrap().paint() else {
                        panic!("color")
                    };
                    styles.push((span.font_size().get(), *color));
                }
                _ => {}
            }
        }
    }
    let mut styles = Vec::new();
    visit(tree.root(), &mut styles);
    assert_eq!(
        styles,
        [
            (12.0, usvg::Color::new_rgb(255, 0, 0)),
            (24.0, usvg::Color::new_rgb(0, 0, 255))
        ]
    );
}

#[test]
fn style_sources_are_checked_and_values_cannot_contain_declarations() {
    let xml = usvg::roxmltree::Document::parse(
        "<svg xmlns='http://www.w3.org/2000/svg' width='1' height='1'/>",
    )
    .unwrap();
    let style = ResolvedElementStyle {
        source_node_id: xml.root_element().id().get(),
        values: vec![],
    };
    assert!(matches!(
        usvg::Tree::from_xmltree_with_resolved_styles(&xml, &options(), &[style.clone(), style]),
        Err(usvg::Error::InvalidResolvedStyleSource(_))
    ));
    for (property, text) in [
        (P::FontSize, "32px;fill:red"),
        (P::FontSize, "NaN"),
        (P::Fill, "red !important"),
        (P::Fill, "url(#paint)"),
        (P::StrokeWidth, "-3px"),
    ] {
        assert!(
            ResolvedStyleValue::parse(property, text).is_none(),
            "{property:?} {text}"
        );
    }
}
