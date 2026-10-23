Попытка
    ПервыйОбъектПробы = Обработки.Замеры.Создать();
    ВторойОбъектПробы = Обработки.Замеры.Создать();
    Сообщить("image.native.module_initializer_source.first=" + Строка(ПервыйОбъектПробы.ПрочитатьПробы()));
    Сообщить("image.native.module_initializer_source.second=" + Строка(ВторойОбъектПробы.ПрочитатьПробы()));
    ПервыйОбъектПробы.УстановитьПробы(111);
    Сообщить("image.native.module_initializer_source.first_changed=" + Строка(ПервыйОбъектПробы.ПрочитатьПробы()));
    Сообщить("image.native.module_initializer_source.second_unchanged=" + Строка(ВторойОбъектПробы.ПрочитатьПробы()));
    ВторойОбъектПробы.УстановитьПробы(222);
    Сообщить("image.native.module_initializer_source.first_after_second=" + Строка(ПервыйОбъектПробы.ПрочитатьПробы()));
    Сообщить("image.native.module_initializer_source.second_changed=" + Строка(ВторойОбъектПробы.ПрочитатьПробы()));
    ТретийОбъектПробы = Обработки.Замеры.Создать();
    Сообщить("image.native.module_initializer_source.third=" + Строка(ТретийОбъектПробы.ПрочитатьПробы()));
Исключение
    Сообщить("image.native.module_initializer_source.failure=" + ОписаниеОшибки());
КонецПопытки;
