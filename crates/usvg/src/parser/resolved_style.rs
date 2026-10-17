// Copyright 2026 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::str::FromStr;

use super::svgtree::AId;

/// The presentation properties accepted from a host style engine.
///
/// This closed set deliberately excludes selectors, declarations, DOM callbacks
/// and structural attributes such as `id`, `href` and path data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum ResolvedStyleProperty {
    Color,
    Display,
    Visibility,
    FontSize,
    FontFamily,
    FontWeight,
    FontStyle,
    Direction,
    FontKerning,
    LetterSpacing,
    WordSpacing,
    TextAnchor,
    Fill,
    Stroke,
    FillOpacity,
    StrokeOpacity,
    FillRule,
    ClipRule,
    StrokeWidth,
    StrokeDasharray,
    StrokeDashoffset,
    StrokeLinecap,
    StrokeLinejoin,
    StrokeMiterlimit,
    PaintOrder,
    ShapeRendering,
}

/// One validated, computed presentation value. It is not a CSS declaration.
#[derive(Clone, Debug)]
pub struct ResolvedStyleValue {
    property: ResolvedStyleProperty,
    value: Value,
}

#[derive(Clone, Debug)]
enum Value {
    Number(f64),
    Length(svgtypes::Length),
    Lengths(Vec<svgtypes::Length>),
    Color(svgtypes::Color),
    Families(Vec<svgtypes::FontFamily>),
    PaintOrder(svgtypes::PaintOrder),
    Keyword(&'static str),
}

impl ResolvedStyleValue {
    /// Admits a single computed value into the typed input protocol.
    ///
    /// Font and stroke lengths retain their explicit units. CSS-wide keywords,
    /// declarations and resource URLs are not accepted. Resource resolution
    /// remains with the SVG document's existing backend.
    pub fn parse(property: ResolvedStyleProperty, value: &str) -> Option<Self> {
        use ResolvedStyleProperty as P;
        if matches!(
            value,
            "inherit" | "initial" | "unset" | "revert" | "revert-layer"
        ) {
            return None;
        }
        let keyword = |values: &[&'static str]| {
            values
                .iter()
                .find(|v| **v == value)
                .copied()
                .map(Value::Keyword)
        };
        let number = || {
            let number = value.parse::<f64>().ok()?;
            number.is_finite().then_some(Value::Number(number))
        };
        let length = || {
            let length = svgtypes::Length::from_str(value).ok()?;
            length.number.is_finite().then_some(Value::Length(length))
        };
        let parsed = match property {
            P::Color => Value::Color(value.parse().ok()?),
            P::Fill | P::Stroke => {
                if let Some(value) = keyword(&["none", "context-fill", "context-stroke"]) {
                    value
                } else {
                    Value::Color(value.parse().ok()?)
                }
            }
            P::Display => {
                // usvg only distinguishes display:none from participating SVG
                // content. The host has already computed the display value.
                if value.is_empty()
                    || value.split_whitespace().any(|word| {
                        !matches!(
                            word,
                            "none"
                                | "contents"
                                | "block"
                                | "inline"
                                | "run-in"
                                | "list-item"
                                | "flow"
                                | "flow-root"
                                | "flex"
                                | "grid"
                                | "ruby"
                                | "table"
                                | "inline-block"
                                | "inline-table"
                                | "inline-flex"
                                | "inline-grid"
                                | "table-row-group"
                                | "table-header-group"
                                | "table-footer-group"
                                | "table-row"
                                | "table-cell"
                                | "table-column-group"
                                | "table-column"
                                | "table-caption"
                                | "ruby-base"
                                | "ruby-text"
                                | "ruby-base-container"
                                | "ruby-text-container"
                        )
                    })
                {
                    return None;
                }
                Value::Keyword(if value == "none" { "none" } else { "inline" })
            }
            P::Visibility => keyword(&["visible", "hidden", "collapse"])?,
            P::FontFamily => Value::Families(svgtypes::parse_font_families(value).ok()?),
            P::FontSize | P::StrokeWidth => {
                let parsed = length()?;
                if matches!(&parsed, Value::Length(length) if length.number < 0.0) {
                    return None;
                }
                parsed
            }
            P::FontWeight => match value {
                "normal" => Value::Number(400.0),
                "bold" => Value::Number(700.0),
                _ => number()?,
            },
            P::FontStyle => {
                if let Some(angle) = value.strip_prefix("oblique ") {
                    svgtypes::Angle::from_str(angle).ok()?;
                    Value::Keyword("oblique")
                } else {
                    keyword(&["normal", "italic", "oblique"])?
                }
            }
            P::Direction => keyword(&["ltr", "rtl"])?,
            P::FontKerning => keyword(&["auto", "normal", "none"])?,
            P::LetterSpacing | P::WordSpacing => {
                if value == "normal" {
                    Value::Number(0.0)
                } else {
                    length()?
                }
            }
            P::TextAnchor => keyword(&["start", "middle", "end"])?,
            P::FillOpacity | P::StrokeOpacity => {
                let number = value.parse::<f64>().ok()?;
                if !number.is_finite() || !(0.0..=1.0).contains(&number) {
                    return None;
                }
                Value::Number(number)
            }
            P::FillRule | P::ClipRule => keyword(&["nonzero", "evenodd"])?,
            P::StrokeDasharray => {
                let lengths = if value == "none" {
                    Vec::new()
                } else {
                    svgtypes::LengthListParser::from(value)
                        .collect::<Result<Vec<_>, _>>()
                        .ok()?
                };
                if lengths
                    .iter()
                    .any(|length| !length.number.is_finite() || length.number < 0.0)
                {
                    return None;
                }
                Value::Lengths(lengths)
            }
            P::StrokeDashoffset => length()?,
            P::StrokeLinecap => keyword(&["butt", "round", "square"])?,
            P::StrokeLinejoin => keyword(&["miter", "miter-clip", "round", "bevel", "arcs"])?,
            P::StrokeMiterlimit => number()?,
            P::PaintOrder => {
                let tokens = value.split_whitespace().collect::<Vec<_>>();
                if tokens.is_empty()
                    || tokens.len() > 3
                    || (value != "normal"
                        && tokens.iter().enumerate().any(|(i, token)| {
                            !matches!(*token, "fill" | "stroke" | "markers")
                                || tokens[..i].contains(token)
                        }))
                {
                    return None;
                }
                Value::PaintOrder(value.parse().ok()?)
            }
            P::ShapeRendering => {
                keyword(&["auto", "optimizeSpeed", "crispEdges", "geometricPrecision"])?
            }
        };
        Some(Self {
            property,
            value: parsed,
        })
    }

    /// Conservative size of this value's owned data, including backend atom
    /// conversion. Hosts can admit it before starting a backend layout.
    pub fn estimated_bytes(&self) -> usize {
        let extra = match &self.value {
            Value::Lengths(values) => values
                .capacity()
                .saturating_mul(std::mem::size_of::<svgtypes::Length>()),
            Value::Families(values) => {
                values.capacity() * std::mem::size_of::<svgtypes::FontFamily>()
                    + values
                        .iter()
                        .map(|family| match family {
                            svgtypes::FontFamily::Named(name) => name.capacity(),
                            _ => 0,
                        })
                        .sum::<usize>()
            }
            _ => 0,
        };
        std::mem::size_of::<Self>()
            .saturating_add(extra)
            .saturating_add(self.attribute().1.len().saturating_mul(2))
            .saturating_add(2 * std::mem::size_of::<super::svgtree::Attribute<'_>>())
    }

    pub(super) fn attribute(&self) -> (AId, String) {
        use ResolvedStyleProperty as P;
        let id = match self.property {
            P::Color => AId::Color,
            P::Display => AId::Display,
            P::Visibility => AId::Visibility,
            P::FontSize => AId::FontSize,
            P::FontFamily => AId::FontFamily,
            P::FontWeight => AId::FontWeight,
            P::FontStyle => AId::FontStyle,
            P::Direction => AId::Direction,
            P::FontKerning => AId::FontKerning,
            P::LetterSpacing => AId::LetterSpacing,
            P::WordSpacing => AId::WordSpacing,
            P::TextAnchor => AId::TextAnchor,
            P::Fill => AId::Fill,
            P::Stroke => AId::Stroke,
            P::FillOpacity => AId::FillOpacity,
            P::StrokeOpacity => AId::StrokeOpacity,
            P::FillRule => AId::FillRule,
            P::ClipRule => AId::ClipRule,
            P::StrokeWidth => AId::StrokeWidth,
            P::StrokeDasharray => AId::StrokeDasharray,
            P::StrokeDashoffset => AId::StrokeDashoffset,
            P::StrokeLinecap => AId::StrokeLinecap,
            P::StrokeLinejoin => AId::StrokeLinejoin,
            P::StrokeMiterlimit => AId::StrokeMiterlimit,
            P::PaintOrder => AId::PaintOrder,
            P::ShapeRendering => AId::ShapeRendering,
        };
        // The existing converter consumes individual SVG value atoms. This
        // conversion never creates declarations or enters the CSS cascade.
        let length = |v: &svgtypes::Length| {
            use svgtypes::LengthUnit as U;
            let unit = match v.unit {
                U::None => "",
                U::Em => "em",
                U::Ex => "ex",
                U::Px => "px",
                U::In => "in",
                U::Cm => "cm",
                U::Mm => "mm",
                U::Pt => "pt",
                U::Pc => "pc",
                U::Percent => "%",
            };
            format!("{}{unit}", v.number)
        };
        let value = match &self.value {
            Value::Number(v) => v.to_string(),
            Value::Length(v) => length(v),
            Value::Lengths(v) if v.is_empty() => "none".to_owned(),
            Value::Lengths(v) => v.iter().map(length).collect::<Vec<_>>().join(" "),
            Value::Color(v) => format!(
                "rgba({},{},{},{})",
                v.red,
                v.green,
                v.blue,
                f64::from(v.alpha) / 255.0
            ),
            Value::Families(v) => v
                .iter()
                .map(|family| match family {
                    svgtypes::FontFamily::Named(name) => {
                        format!("\"{}\"", name.replace('\\', "\\\\").replace('"', "\\\""))
                    }
                    other => other.to_string(),
                })
                .collect::<Vec<_>>()
                .join(","),
            Value::PaintOrder(v) => v
                .order
                .iter()
                .map(|v| match v {
                    svgtypes::PaintOrderKind::Fill => "fill",
                    svgtypes::PaintOrderKind::Stroke => "stroke",
                    svgtypes::PaintOrderKind::Markers => "markers",
                })
                .collect::<Vec<_>>()
                .join(" "),
            Value::Keyword(v) => (*v).to_owned(),
        };
        (id, value)
    }
}

/// Computed presentation values for one original XML element.
///
/// Values apply to that original element, after document CSS. They are not
/// broadcast onto generated `use` instances: those continue to resolve authored
/// SVG values and inheritance in their instance context. Host document CSS on
/// referenced instances requires a separate instance-aware style integration.
#[derive(Clone, Debug)]
pub struct ResolvedElementStyle {
    /// Original `roxmltree` element ID, independent of the authored `id`.
    pub source_node_id: u32,
    /// Computed values; duplicate properties are resolved in input order.
    pub values: Vec<ResolvedStyleValue>,
}
