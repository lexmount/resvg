// Copyright 2026 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::{collections::HashMap, sync::Arc};

use crate::{BBox, Rect, Transform};

/// A final style value supplied by an embedding document after its cascade.
#[derive(Clone, Debug)]
pub struct ElementStyleProperty {
    /// A CSS presentation property name.
    pub name: String,
    /// Its final serialized value, with shared owned storage.
    pub value: roxmltree::StringStorage<'static>,
}

impl ElementStyleProperty {
    /// Creates an owned value whose storage is shared when profiles are cloned.
    pub fn new(name: impl Into<String>, value: &str) -> Self {
        Self {
            name: name.into(),
            value: roxmltree::StringStorage::new_owned(value),
        }
    }
}

/// Overrides are keyed by the original XML node, never an injected attribute.
/// Profiles and their string values can be shared by many equal elements.
pub type ElementStyleOverrides = HashMap<roxmltree::NodeId, Arc<[ElementStyleProperty]>>;

/// Source object geometry, independent of clipping and paint-tree retention.
#[derive(Clone, Copy, Debug)]
pub struct ObjectGeometry {
    /// Identity in the original XML document, including anonymous elements.
    pub source: roxmltree::NodeId,
    /// Unclipped object bounds in the element's user coordinate system.
    pub bounding_box: Rect,
    /// Maps those coordinates into the outer SVG viewport.
    pub transform: Transform,
}

#[derive(Clone, Default)]
pub(crate) struct GeometryCollector {
    pub(crate) objects: Vec<ObjectGeometry>,
    frames: Vec<Vec<(Rect, Transform)>>,
}

impl GeometryCollector {
    pub(crate) fn begin(&mut self) {
        self.frames.push(Vec::new());
    }

    pub(crate) fn finish(
        &mut self,
        source: Option<roxmltree::NodeId>,
        own: Option<Rect>,
        transform: Transform,
    ) {
        let children = self.frames.pop().expect("balanced SVG geometry scope");
        let bounds = if children.is_empty() {
            own
        } else if let Some(inverse) = transform.invert() {
            children
                .iter()
                .fold(BBox::default(), |bounds, (rect, child_transform)| {
                    rect.transform(inverse.pre_concat(*child_transform))
                        .map_or(bounds, |rect| bounds.expand(rect))
                })
                .to_rect()
        } else {
            // A singular transform has no local inverse. Retain the local
            // paint bounds when available; its final quad remains singular.
            own
        };
        if let Some(bounding_box) = bounds {
            if let Some(source) = source {
                self.objects.push(ObjectGeometry {
                    source,
                    bounding_box,
                    transform,
                });
            }
            if let Some(parent) = self.frames.last_mut() {
                parent.push((bounding_box, transform));
            }
        }
    }
}
