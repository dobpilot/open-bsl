# Design

Native measure-fixed-map-iteration records zero iterations for an empty map, two key/value pairs for a nonempty snapshot, original values after source replacement/insertion, and rejection of pair value assignment. Use the immutable entries already stored by FixedMapObject and the existing BslObject::KeyValuePair. Keep the traversal index separate from public indexing. The control does not establish a portable ordering guarantee, so the requirement and tests compare entries rather than infer sort order.
