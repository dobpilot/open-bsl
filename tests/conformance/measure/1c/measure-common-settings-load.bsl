// PLATFORM-ONLY: собственные записи общих настроек в одноразовой базе.
Процедура М(Ид, Значение)
    Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "common-settings-load-1");
Попытка
    ОтсутствующаяНастройка = ХранилищеОбщихНастроек.Загрузить("open-bsl-common-settings-control-1013", "missing");
    М("COMMON_SETTINGS_LOAD.MISSING", ТипЗнч(ОтсутствующаяНастройка));
    ХранилищеОбщихНастроек.Сохранить("open-bsl-common-settings-control-1013", "scalar", 17);
    СкалярНастройки = ХранилищеОбщихНастроек.Загрузить("open-bsl-common-settings-control-1013", "scalar");
    М("COMMON_SETTINGS_LOAD.SCALAR_TYPE", ТипЗнч(СкалярНастройки));
    М("COMMON_SETTINGS_LOAD.SCALAR_EQUAL", СкалярНастройки = 17);
    ДанныеНастройки = Новый Соответствие;
    ДанныеНастройки.Вставить("probe", Истина);
    ХранилищеОбщихНастроек.Сохранить("open-bsl-common-settings-control-1013", "storage", Новый ХранилищеЗначения(ДанныеНастройки));
    ХранилищеНастройки = ХранилищеОбщихНастроек.Загрузить("open-bsl-common-settings-control-1013", "storage");
    М("COMMON_SETTINGS_LOAD.STORAGE_TYPE", ТипЗнч(ХранилищеНастройки));
    ПолученныеДанныеНастройки = ХранилищеНастройки.Получить();
    М("COMMON_SETTINGS_LOAD.CONTENT_TYPE", ТипЗнч(ПолученныеДанныеНастройки));
    М("COMMON_SETTINGS_LOAD.CONTENT_EQUAL", ПолученныеДанныеНастройки.Получить("probe") = Истина);
Исключение
    М("COMMON_SETTINGS_LOAD.ERROR", ОписаниеОшибки());
КонецПопытки;
М("file.end", "common-settings-load-1");
