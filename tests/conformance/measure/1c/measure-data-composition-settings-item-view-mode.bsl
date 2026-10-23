// PLATFORM-ONLY: проба фиксирует полное перечисление режимов отображения элемента настройки.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "data-composition-settings-item-view-mode-1");
М("DCS.SETTINGS.VIEW.AUTO", РежимОтображенияЭлементаНастройкиКомпоновкиДанных.Авто);
М("DCS.SETTINGS.VIEW.NORMAL", РежимОтображенияЭлементаНастройкиКомпоновкиДанных.Обычный);
М("DCS.SETTINGS.VIEW.QUICK_ACCESS", РежимОтображенияЭлементаНастройкиКомпоновкиДанных.БыстрыйДоступ);
М("DCS.SETTINGS.VIEW.INACCESSIBLE", РежимОтображенияЭлементаНастройкиКомпоновкиДанных.Недоступный);
М("DCS.SETTINGS.VIEW.TYPE", ТипЗнч(РежимОтображенияЭлементаНастройкиКомпоновкиДанных.Авто));

АнглийскиеИмена = "ERROR";
Попытка
	Если DataCompositionSettingsItemViewMode.Auto = РежимОтображенияЭлементаНастройкиКомпоновкиДанных.Авто
		И DataCompositionSettingsItemViewMode.Normal = РежимОтображенияЭлементаНастройкиКомпоновкиДанных.Обычный
		И DataCompositionSettingsItemViewMode.QuickAccess = РежимОтображенияЭлементаНастройкиКомпоновкиДанных.БыстрыйДоступ
		И DataCompositionSettingsItemViewMode.Inaccessible = РежимОтображенияЭлементаНастройкиКомпоновкиДанных.Недоступный Тогда
		АнглийскиеИмена = "OK";
	КонецЕсли;
Исключение
КонецПопытки;
М("DCS.SETTINGS.VIEW.ENGLISH", АнглийскиеИмена);
М("file.end", "data-composition-settings-item-view-mode-1");
