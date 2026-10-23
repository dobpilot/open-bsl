# Design

Use the SDK owner-only complete optional GUID list, resolving section names from existing acquired descriptors once. Unknown and known-empty lists remain distinct. Do not parse Config serialization again in open-bsl and do not include nested row attributes in the object receiver context.

Reuse EngineBuilder::configuration_property_names and existing object property dispatch. Standard properties retain their callbacks. Until runtime section types and rows are measured and implemented, reads and writes of acquired sections raise an explicit named unavailable-section error. This is an implementation boundary, not a platform semantics claim.

Native XML identifies Состав as 6581ac1f-11ef-42b8-9f3b-590bb7546e63 and its nested Пользователь as ce952e88-35b7-4506-81ac-d7081e50a650. Preserve independent XML/Config evidence in fixtures.

The SDK read-only comparison identifies root collection `932159f9-95b2-4e76-a8dd-8849fe5c5ded` and verifies all 313 section headers against native XML across 296 catalogs, including 151 known-empty collections. The owner-only API is `ConfigDescriptor::catalog_tabular_sections: Option<Vec<Guid>>`; it promises ordered complete membership and uses `None` for unknown, incomplete or duplicate membership. Section header selection must follow the measured header structure, not the first recursive descriptor, since nested row fields share the descriptor shape.
