# Moli's usvg 0.48.1 patch set

Fork base: upstream resvg's `usvg` 0.48.1 release, commit
`68b14c4c3bccdb60344c777406486b54c36ec1a4`.

The production source changes are migrated from Moli's vendored `usvg` 0.48.1.
They retain layout provenance that would otherwise be discarded:

- Original XML element indices on `Text` and `TextSpan`, independent of
  authored IDs. No source XML or live DOM attributes are modified.

These additions do not introduce a second text shaper or change paint output.
SVG DOM UTF-16 indexing, source-node association and immutable snapshot ownership
belong to Moli's consumer, not to an alternate layout algorithm in this patch.

`tests/text_geometry.rs` covers source association with duplicate IDs and nested
spans.
Moli also tests the SVG DOM consumers and their rendering snapshots.
