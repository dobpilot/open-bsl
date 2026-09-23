// PLATFORM-ONLY: проба фиксирует конструктор и свойства ОписаниеНастроек.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "settings-description-1");
Описание = Новый ОписаниеНастроек;
М("SETTINGS.DESCRIPTION.TYPE", ТипЗнч(Описание));
М("SETTINGS.DESCRIPTION.OBJECT.DEFAULT", Описание.КлючОбъекта = Неопределено);
М("SETTINGS.DESCRIPTION.SETTINGS.DEFAULT", Описание.КлючНастроек = Неопределено);
М("SETTINGS.DESCRIPTION.USER.DEFAULT", Описание.Пользователь);
М("SETTINGS.DESCRIPTION.PRESENTATION.DEFAULT", Описание.Представление);
М("SETTINGS.DESCRIPTION.ADDITIONAL.TYPE", ТипЗнч(Описание.ДополнительныеСвойства));
М("SETTINGS.DESCRIPTION.ADDITIONAL.COUNT", Описание.ДополнительныеСвойства.Количество());

Описание.КлючОбъекта = 1;
Описание.КлючНастроек = "key";
Описание.Пользователь = "user";
Описание.Представление = "presentation";
Описание.ДополнительныеСвойства.Вставить("Поле", 42);
М("SETTINGS.DESCRIPTION.OBJECT.WRITE", Описание.КлючОбъекта);
М("SETTINGS.DESCRIPTION.SETTINGS.WRITE", Описание.КлючНастроек);
М("SETTINGS.DESCRIPTION.USER.WRITE", Описание.Пользователь);
М("SETTINGS.DESCRIPTION.PRESENTATION.WRITE", Описание.Представление);
М("SETTINGS.DESCRIPTION.ADDITIONAL.WRITE", Описание.ДополнительныеСвойства.Поле);

ЗаписьДополнительных = "ERROR";
Попытка
	Описание.ДополнительныеСвойства = Новый Структура;
	ЗаписьДополнительных = "OK";
Исключение
КонецПопытки;
М("SETTINGS.DESCRIPTION.ADDITIONAL.READONLY", ЗаписьДополнительных);

EnglishDescription = New SettingsDescription();
EnglishDescription.ObjectKey = 2;
EnglishDescription.SettingsKey = "english";
EnglishDescription.User = "name";
EnglishDescription.Presentation = "caption";
EnglishDescription.AdditionalProperties.Insert("Field", 7);
М("SETTINGS.DESCRIPTION.ENGLISH", EnglishDescription.ObjectKey = 2
	И EnglishDescription.SettingsKey = "english"
	И EnglishDescription.User = "name"
	И EnglishDescription.Presentation = "caption"
	И EnglishDescription.AdditionalProperties.Field = 7);
М("file.end", "settings-description-1");
