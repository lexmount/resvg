// Copyright 2026 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![cfg(feature = "text")]

use std::sync::Arc;

fn text_weights(source: &str) -> Vec<u16> {
    let mut fontdb = usvg::fontdb::Database::new();
    fontdb.load_font_data(include_bytes!("../../resvg/tests/fonts/NotoSans-Regular.ttf").to_vec());
    let options = usvg::Options {
        font_family: "Noto Sans".to_owned(),
        fontdb: Arc::new(fontdb),
        ..usvg::Options::default()
    };
    let tree = usvg::Tree::from_str(source, &options).unwrap();
    let Some(usvg::Node::Text(text)) = tree.node_by_id("text") else {
        panic!("text must shape with the checked-in font");
    };
    text.chunks()
        .iter()
        .flat_map(|chunk| chunk.spans())
        .map(|span| span.font().weight())
        .collect()
}

#[test]
fn relative_weights_follow_css_font_weight_ranges() {
    // https://drafts.csswg.org/css-fonts-4/#relative-weights
    for (inherited, bolder, lighter) in [
        ("1", 400, 1),
        ("99", 400, 99),
        ("100", 400, 100),
        ("200", 400, 100),
        ("300", 400, 100),
        ("349.9", 400, 100),
        ("350", 700, 100),
        ("400", 700, 100),
        ("500", 700, 100),
        ("549.9", 700, 100),
        ("550", 900, 400),
        ("600", 900, 400),
        ("700", 900, 400),
        ("749.9", 900, 400),
        ("750", 900, 700),
        ("800", 900, 700),
        ("900", 900, 700),
        ("950", 950, 700),
        ("1000", 1000, 700),
    ] {
        for (relative, expected) in [("bolder", bolder), ("lighter", lighter)] {
            let source = format!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" width="90" height="60"
                        font-weight="{inherited}">
                    <text id="text" x="8" y="50" font-size="48" font-weight="{relative}">M</text>
                </svg>"#,
            );
            assert_eq!(
                text_weights(&source),
                [expected],
                "inherited {inherited} with {relative}",
            );
        }
    }
}

#[test]
fn nested_relative_weights_resolve_each_ancestor_once() {
    let source = r#"<svg xmlns="http://www.w3.org/2000/svg" width="90" height="60"
            font-weight="700">
        <g font-weight="lighter"><g>
            <text id="text" x="8" y="50" font-size="48" font-weight="bolder">M<tspan font-weight="lighter">M</tspan></text>
        </g></g>
    </svg>"#;
    assert_eq!(text_weights(source), [700, 400]);
}

#[test]
fn numeric_weights_accept_css_numbers_outside_the_nine_step_scale() {
    for (value, expected) in [("1", 1), ("349.9", 350), ("650", 650), ("1000", 1000)] {
        let source = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="90" height="60">
                <text id="text" x="8" y="50" font-size="48" font-weight="{value}">M</text>
            </svg>"#,
        );
        assert_eq!(text_weights(&source), [expected], "numeric weight {value}");
    }
}

#[test]
fn invalid_weights_keep_the_inherited_value() {
    for invalid in ["0", "1001", "-100", "NaN", "inf", "boldish"] {
        let source = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="90" height="60"
                    font-weight="700">
                <text id="text" x="8" y="50" font-size="48" font-weight="{invalid}">M</text>
            </svg>"#,
        );
        assert_eq!(text_weights(&source), [700], "invalid weight {invalid}");
    }
}
