//! Достигнутая конфигурацией поверхность компоновки данных. Механизма
//! компоновки нет, поэтому каждая операция явно отказывает, а не возвращает
//! частичный результат.

use super::*;

#[derive(Debug)]
struct DataCompositionTemplateComposerValue;

#[derive(Debug)]
struct DataCompositionProcessorValue;

#[derive(Debug, Default)]
struct DataCompositionValueCollectionOutputProcessorValue {
    show_output_percent: Cell<bool>,
}

pub(crate) static DATA_COMPOSITION_SETTINGS_COMPOSER_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "КомпоновщикНастроекКомпоновкиДанных",
    type_display: "Data composition settings composer",
    type_names: &["DataCompositionSettingsComposer"],
};

pub(crate) static DATA_COMPOSITION_AVAILABLE_SETTINGS_SOURCE_TYPE: TypeDescriptor =
    TypeDescriptor {
        package: "bsl-orm",
        name: "ИсточникДоступныхНастроекКомпоновкиДанных",
        type_display: "Data composition available settings source",
        type_names: &["DataCompositionAvailableSettingsSource"],
    };

pub(crate) static DATA_COMPOSITION_DETAILS_PROCESS_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "ОбработкаРасшифровкиКомпоновкиДанных",
    type_display: "Data composition details processor",
    type_names: &["DataCompositionDetailsProcess"],
};

pub(crate) static DATA_COMPOSITION_TEMPLATE_COMPOSER_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "КомпоновщикМакетаКомпоновкиДанных",
    type_display: "Composer of data composition template",
    type_names: &["DataCompositionTemplateComposer"],
};

pub(crate) static DATA_COMPOSITION_PROCESSOR_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "ПроцессорКомпоновкиДанных",
    type_display: "Data composition processor",
    type_names: &["DataCompositionProcessor"],
};

pub(crate) static DATA_COMPOSITION_VALUE_COLLECTION_OUTPUT_PROCESSOR_TYPE: TypeDescriptor =
    TypeDescriptor {
        package: "bsl-orm",
        name: "ПроцессорВыводаРезультатаКомпоновкиДанныхВКоллекциюЗначений",
        type_display: "Processor of output of data composition result in values collection",
        type_names: &["DataCompositionResultValueCollectionOutputProcessor"],
    };

impl ObjectProtocol for DataCompositionTemplateComposerValue {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &DATA_COMPOSITION_TEMPLATE_COMPOSER_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        DATA_COMPOSITION_TEMPLATE_COMPOSER_METHODS
    }
}

impl ObjectProtocol for DataCompositionProcessorValue {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &DATA_COMPOSITION_PROCESSOR_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        DATA_COMPOSITION_PROCESSOR_METHODS
    }
}

impl ObjectProtocol for DataCompositionValueCollectionOutputProcessorValue {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &DATA_COMPOSITION_VALUE_COLLECTION_OUTPUT_PROCESSOR_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        DATA_COMPOSITION_VALUE_COLLECTION_OUTPUT_PROCESSOR_METHODS
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        DATA_COMPOSITION_VALUE_COLLECTION_OUTPUT_PROCESSOR_PROPERTIES
    }
}

pub(crate) fn new_data_composition_settings_composer(
    _: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "объектная модель настроек компоновки данных не реализована".into(),
    ))
}

pub(crate) fn new_data_composition_available_settings_source(
    _: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "объектная модель источника доступных настроек не реализована".into(),
    ))
}

pub(crate) fn new_data_composition_details_process(
    _: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "объектная модель обработки расшифровки компоновки данных не реализована".into(),
    ))
}

pub(crate) fn new_data_composition_schema(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "объектная модель схемы компоновки данных не реализована".into(),
    ))
}

pub(crate) fn new_data_composition_spreadsheet_output(
    _: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "вывод результата компоновки данных в табличный документ не реализован".into(),
    ))
}

pub(crate) fn new_data_composition_user_settings(
    _: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "объектная модель пользовательских настроек компоновки данных не реализована".into(),
    ))
}

pub(crate) fn new_data_composition_template_composer(
    _: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    Ok(Value::new_object(DataCompositionTemplateComposerValue))
}

fn compose_data_composition_template(
    _: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "движок компоновки макета данных не реализован".into(),
    ))
}

pub(crate) fn new_data_composition_processor(
    _: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    Ok(Value::new_object(DataCompositionProcessorValue))
}

fn data_composition_processing_unavailable(
    _: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "движок компоновки данных не реализован".into(),
    ))
}

pub(crate) fn new_data_composition_value_collection_output_processor(
    _: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    Ok(Value::new_object(
        DataCompositionValueCollectionOutputProcessorValue::default(),
    ))
}

fn data_composition_output_unavailable(
    _: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "движок вывода результата компоновки данных не реализован".into(),
    ))
}

fn show_output_percent(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    let output = receiver
        .downcast_ref::<DataCompositionValueCollectionOutputProcessorValue>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::Boolean(output.show_output_percent.get()))
}

fn set_show_output_percent(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    let output = receiver
        .downcast_ref::<DataCompositionValueCollectionOutputProcessorValue>()
        .ok_or(RtError::NotAnObject)?;
    let Value::Boolean(value) = value else {
        return Err(RtError::TypeError {
            expected: "Булево",
            op: "ОтображатьПроцентВывода",
        });
    };
    output.show_output_percent.set(value);
    Ok(())
}

pub(crate) static DATA_COMPOSITION_TEMPLATE_COMPOSER_METHODS: &[MethodDescriptor] =
    &[MethodDescriptor::new(
        &["Выполнить", "Execute"],
        Arity::range(2, 7),
        compose_data_composition_template,
    )];

pub(crate) static DATA_COMPOSITION_PROCESSOR_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["Инициализировать", "Initialize"],
        Arity::range(1, 6),
        data_composition_processing_unavailable,
    )
    .as_procedure(),
    MethodDescriptor::new(
        &["Сбросить", "Reset"],
        Arity::exact(0),
        data_composition_processing_unavailable,
    )
    .as_procedure(),
    MethodDescriptor::new(
        &["Следующий", "Next"],
        Arity::exact(0),
        data_composition_processing_unavailable,
    ),
];

pub(crate) static DATA_COMPOSITION_VALUE_COLLECTION_OUTPUT_PROCESSOR_METHODS:
    &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["УстановитьОбъект", "SetObject"],
        Arity::range(0, 1),
        data_composition_output_unavailable,
    )
    .as_procedure(),
    MethodDescriptor::new(
        &["Вывести", "Output"],
        Arity::range(1, 2),
        data_composition_output_unavailable,
    ),
    MethodDescriptor::new(
        &["ВывестиЭлемент", "OutputItem"],
        Arity::exact(1),
        data_composition_output_unavailable,
    )
    .as_procedure(),
    MethodDescriptor::new(
        &["НачатьВывод", "BeginOutput"],
        Arity::exact(0),
        data_composition_output_unavailable,
    )
    .as_procedure(),
    MethodDescriptor::new(
        &["ЗакончитьВывод", "EndOutput"],
        Arity::exact(0),
        data_composition_output_unavailable,
    ),
];

pub(crate) static DATA_COMPOSITION_VALUE_COLLECTION_OUTPUT_PROCESSOR_PROPERTIES:
    &[PropertyDescriptor] = &[PropertyDescriptor {
    names: &["ОтображатьПроцентВывода", "ShowOutputPercent"],
    get: show_output_percent,
    set: Some(set_show_output_percent),
}];
