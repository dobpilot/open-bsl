//! Достигнутые объявления собственного менеджера плана обмена.

use crate::*;

// Сигнатура позволяет компилировать неисполняемые ветки оригинального
// модуля менеджера. Объект узла хост пока не создаёт даже в памяти.
pub(super) static METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(&["СоздатьУзел", "CreateNode"], Arity::exact(0), create_node),
    // Нативная проба подтверждает тот же пустой UUID и собственный тип
    // владельца, которые уже хранит общая ссылка конфигурации.
    MethodDescriptor::new(
        &["ПустаяСсылка", "EmptyRef"],
        Arity::exact(0),
        document_empty_ref,
    ),
    MethodDescriptor::new(&["ЭтотУзел", "ThisNode"], Arity::exact(0), this_node),
    MethodDescriptor::new(
        &["ПолучитьСсылку", "GetRef"],
        Arity::range(0, 1),
        configuration_get_ref,
    ),
];

fn create_node(_: &dyn ObjectProtocol, _: &[Value], _: &mut CallContext<'_>) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "создание узла плана обмена не поддерживается этим хостом".into(),
    ))
}

fn this_node(_: &dyn ObjectProtocol, _: &[Value], _: &mut CallContext<'_>) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "текущий узел плана обмена не приобретён хостом".into(),
    ))
}
