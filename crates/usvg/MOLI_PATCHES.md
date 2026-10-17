# Moli's usvg 0.48.1 patch set

Fork base: upstream resvg's `usvg` 0.48.1 release, commit
`68b14c4c3bccdb60344c777406486b54c36ec1a4`.

The production source changes are migrated from Moli's vendored `usvg` 0.48.1.
They retain layout provenance that would otherwise be discarded:

- Original XML element indices on `Text` and `TextSpan`, independent of
  authored IDs. No source XML or live DOM attributes are modified.
- `Text::layouted_clusters()` exposes logical normalized byte ranges,
  typographic advance, font ascent/descent, bidi direction and final placement
  from the very same shaped clusters used to build paint glyphs.
- Advance is sampled after letter/word spacing but before textLength placement
  adjustments; glyph scaling remains in the final cluster transform.

These additions do not introduce a second text shaper or change paint output.
SVG DOM UTF-16 indexing, source-node association and immutable snapshot ownership
belong to Moli's consumer, not to an alternate layout algorithm in this patch.

`tests/text_geometry.rs` covers source association with duplicate IDs, nested
spans and the distinction between typographic advance and final placement.
Moli also tests the SVG DOM consumers and their rendering snapshots.

## Explicit host style input

`Tree::from_xmltree_with_resolved_styles` accepts a read-only table of original
XML element identities and validated presentation values. The closed property
enum and value parser accept individual computed atoms, not CSS declaration
lists, selectors or host callbacks. Values enter the existing SVG attribute
conversion after authored CSS has been evaluated, so the host's final cascade
does not require generated inline styles or `!important`.

The ordinary `from_data` and `from_xmltree` paths retain their original behavior.
This API does not resolve external resources or add another text shaper.

The table describes original XML elements. It deliberately does not copy a
source element's computed inherited values into generated `<use>` instances;
instances retain the backend's SVG attribute/stylesheet and use-parent
inheritance. Applying document CSS to instance trees requires a future
instance-aware host input and is not provided by this API. Resource-valued
paint is likewise left to the existing authored SVG resource path.

`tests/resolved_style.rs` covers host font/fill precedence, declaration
rejection, source validation, ordinary decoding and distinct inherited styles
on two `<use>` instances.
