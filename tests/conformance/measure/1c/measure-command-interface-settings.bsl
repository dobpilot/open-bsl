// PLATFORM-ONLY: проба фиксирует достигнутый срез настроек командного интерфейса.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "command-interface-settings-1");

Настройки = Новый НастройкиКомандногоИнтерфейса;
М("COMMAND.SETTINGS.TYPE", ТипЗнч(Настройки));
М("COMMAND.SETTINGS.DEFAULT", Настройки.ОтображениеПанелиРазделов);
Настройки.ОтображениеПанелиРазделов = ОтображениеПанелиРазделов.КартинкаИТекст;
М("COMMAND.SETTINGS.MUTATED", Настройки.ОтображениеПанелиРазделов);

М("COMMAND.SECTIONS.PICTURE", ОтображениеПанелиРазделов.Картинка);
М("COMMAND.SECTIONS.PICTURE_TEXT", ОтображениеПанелиРазделов.КартинкаИТекст);
М("COMMAND.SECTIONS.PICTURE_TOP_TEXT", ОтображениеПанелиРазделов.КартинкаСверхуИТекст);
М("COMMAND.SECTIONS.PICTURE_LEFT_TEXT", ОтображениеПанелиРазделов.КартинкаСлеваИТекст);
М("COMMAND.SECTIONS.TEXT", ОтображениеПанелиРазделов.Текст);
М("COMMAND.SECTIONS.TYPE", ТипЗнч(ОтображениеПанелиРазделов.Картинка));

АнглийскиеИмена = "ERROR";
Попытка
	English = New CommandInterfaceSettings;
	English.SectionsPanelRepresentation = SectionsPanelRepresentation.PictureAndText;
	Если English.SectionsPanelRepresentation = SectionsPanelRepresentation.PictureAndText Тогда
		АнглийскиеИмена = "OK";
	КонецЕсли;
Исключение
КонецПопытки;
М("COMMAND.SETTINGS.ENGLISH", АнглийскиеИмена);

НеверныйТип = "ACCEPTED";
Попытка
	Настройки.ОтображениеПанелиРазделов = ВариантИнтерфейсаКлиентскогоПриложения.Такси;
Исключение
	НеверныйТип = "ERROR";
КонецПопытки;
М("COMMAND.SETTINGS.BAD.ENUM", НеверныйТип);

М("file.end", "command-interface-settings-1");
