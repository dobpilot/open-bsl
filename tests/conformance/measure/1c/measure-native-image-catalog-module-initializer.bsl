Попытка
    ПервыйОбъектПробы = Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент();
    ВторойОбъектПробы = Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент();
    Сообщить("image.native.catalog_module_initializer.first=" + Строка(ПервыйОбъектПробы.ПрочитатьПробы()));
    Сообщить("image.native.catalog_module_initializer.second=" + Строка(ВторойОбъектПробы.ПрочитатьПробы()));
    ПервыйОбъектПробы.УстановитьПробы(111);
    Сообщить("image.native.catalog_module_initializer.first_changed=" + Строка(ПервыйОбъектПробы.ПрочитатьПробы()));
    Сообщить("image.native.catalog_module_initializer.second_unchanged=" + Строка(ВторойОбъектПробы.ПрочитатьПробы()));
    ВторойОбъектПробы.УстановитьПробы(222);
    Сообщить("image.native.catalog_module_initializer.first_after_second=" + Строка(ПервыйОбъектПробы.ПрочитатьПробы()));
    Сообщить("image.native.catalog_module_initializer.second_changed=" + Строка(ВторойОбъектПробы.ПрочитатьПробы()));
    ТретийОбъектПробы = Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент();
    Сообщить("image.native.catalog_module_initializer.third=" + Строка(ТретийОбъектПробы.ПрочитатьПробы()));
Исключение
    Сообщить("image.native.catalog_module_initializer.failure=" + ОписаниеОшибки());
КонецПопытки;
