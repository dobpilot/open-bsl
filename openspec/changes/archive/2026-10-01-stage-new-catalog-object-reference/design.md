# Design

Full vendor documentation `GetNewObjectRef2291.html` and `SetNewObjectRef1610.html` under `platform-syntax-help:syntax-context:objects/catalog125/catalog126/object130/methods/` confirms zero and one argument respectively. The setter stages a reference for a future write. Uniqueness is checked during writing, not by this setter.

Store the staged value on the existing CatalogObject, independently of its persisted Ref property, using the same acquired owned reference type. No new runtime dependency, resolver branch, thread-local storage, database mutation or speculative uniqueness query. Keep existing method-table dispatch and instance isolation.

Before runtime implementation, measure the default value, repeated reads, own empty and filled references, invalid argument types and availability in the object-module body. Record the exact native behavior; do not infer coercions or preselection access from other properties. Record persistence remains a separate unfinished part of the overall session-runtime plan.

Native 8.3.27.2342 produced 20 rows in `measure-catalog-new-reference.platform.txt`. Both methods reject access in the object-module body. After selection the default is an own nil reference; own empty/filled values are accepted. Undefined, Null, number, string, boolean, UUID, array, object, other catalog references and document references are rejected while preserving the prior staged value. Staging does not alter Ref and does not change another object's default. English method aliases resolve to the same methods. No record was written by the probe.
