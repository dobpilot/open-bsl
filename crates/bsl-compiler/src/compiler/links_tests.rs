use super::compile_links;
use bsl_rt::{LibraryKey, LibraryRequirement};
use bsl_sema::ResolvedLink;

#[test]
fn object_method_link_uses_the_first_folded_spelling_in_the_shared_name_table() {
    let links = [ResolvedLink::ObjectMethod {
        library: LibraryKey::new("test-component"),
        type_name: "ТипОбъекта",
        method: "получить".to_string(),
    }];
    let requirements = [LibraryRequirement::new("test-component", "1")];
    let names = ["типобъекта".to_string(), "Получить".to_string()];

    let compiled = compile_links(&links, &requirements, &names);
    assert!(matches!(
        compiled.as_slice(),
        [bsl_bytecode::LinkEntry::ObjectMethod {
            library: 0,
            type_name: 0,
            method: 1,
        }]
    ));
}
