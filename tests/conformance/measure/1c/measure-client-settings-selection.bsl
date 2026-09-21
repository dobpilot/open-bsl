// PLATFORM-ONLY: проба фиксирует позиционирование выборки стандартного
// хранилища настроек. База измерений одноразовая; значение сохраняется под
// отдельным служебным ключом.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "client-settings-selection-1");

ХранилищеСистемныхНастроек.Сохранить(
	"open-bsl.measure", "selection", 42);
Отбор = Новый Структура(
	"КлючОбъекта,КлючНастроек", "open-bsl.measure", "selection");
Выборка = ХранилищеСистемныхНастроек.Выбрать(Отбор);

М("CLIENT_SETTINGS.SELECTION.TYPE", ТипЗнч(Выборка));
Попытка
	ДоПервой = "value:" + Строка(Выборка.КлючОбъекта);
Исключение
	ДоПервой = "error:" + КраткоеПредставлениеОшибки(ИнформацияОбОшибке());
КонецПопытки;
М("CLIENT_SETTINGS.SELECTION.BEFORE_FIRST", ДоПервой);

М("CLIENT_SETTINGS.SELECTION.NEXT_FIRST", Выборка.Следующий());
М("CLIENT_SETTINGS.SELECTION.OBJECT_KEY", Выборка.КлючОбъекта);
М("CLIENT_SETTINGS.SELECTION.SETTINGS_KEY", Выборка.КлючНастроек);
М("CLIENT_SETTINGS.SELECTION.USER_TYPE", ТипЗнч(Выборка.Пользователь));
М("CLIENT_SETTINGS.SELECTION.SETTINGS", Выборка.Настройки);
М("CLIENT_SETTINGS.SELECTION.NEXT_END", Выборка.Следующий());
Попытка
	ПослеКонца = "value:" + Строка(Выборка.КлючОбъекта);
Исключение
	ПослеКонца = "error:" + КраткоеПредставлениеОшибки(ИнформацияОбОшибке());
КонецПопытки;
М("CLIENT_SETTINGS.SELECTION.AFTER_END", ПослеКонца);

М("file.end", "client-settings-selection-1");
