// PLATFORM-ONLY: проба фиксирует достигнутый срез состава клиентского интерфейса.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "client-interface-content-settings-1");

Настройки = Новый НастройкиИнтерфейсаКлиентскогоПриложения;
Состав = Новый НастройкиСоставаИнтерфейсаКлиентскогоПриложения;
ГруппаПанелей = Новый ГруппаНастройкиСоставаИнтерфейсаКлиентскогоПриложения;
ЭлементПанели = Новый ЭлементНастройкиСоставаИнтерфейсаКлиентскогоПриложения("ПанельРазделов");

М("CLIENT.INTERFACE.SETTINGS.TYPE", ТипЗнч(Настройки));
М("CLIENT.INTERFACE.CONTENT.TYPE", ТипЗнч(Состав));
М("CLIENT.INTERFACE.GROUP.TYPE", ТипЗнч(ГруппаПанелей));
М("CLIENT.INTERFACE.ITEM.TYPE", ТипЗнч(ЭлементПанели));
М("CLIENT.INTERFACE.ITEM.NAME", ЭлементПанели.Имя);
М("CLIENT.INTERFACE.CONTENT.TOP.TYPE", ТипЗнч(Состав.Верх));
М("CLIENT.INTERFACE.CONTENT.LEFT.TYPE", ТипЗнч(Состав.Лево));
М("CLIENT.INTERFACE.CONTENT.BOTTOM.TYPE", ТипЗнч(Состав.Низ));
М("CLIENT.INTERFACE.CONTENT.RIGHT.TYPE", ТипЗнч(Состав.Право));
М("CLIENT.INTERFACE.CONTENT.LEFT.DEFAULT_COUNT", Состав.Лево.Количество());

ГруппаПанелей.Добавить(ЭлементПанели);
М("CLIENT.INTERFACE.GROUP.COUNT", ГруппаПанелей.Количество());
М("CLIENT.INTERFACE.GROUP.GET.NAME", ГруппаПанелей.Получить(0).Имя);
Состав.Лево.Добавить(ГруппаПанелей);
М("CLIENT.INTERFACE.CONTENT.LEFT.COUNT", Состав.Лево.Количество());

Установка = "ERROR";
Попытка
	Настройки.УстановитьСостав(Состав);
	ПолученныйСостав = Настройки.ПолучитьСостав();
	Если Строка(ТипЗнч(ПолученныйСостав)) = "Client application interface composition settings" Тогда
		Установка = "OK";
	КонецЕсли;
Исключение
КонецПопытки;
М("CLIENT.INTERFACE.SETTINGS.SET_GET", Установка);

АнглийскиеИмена = "ERROR";
Попытка
	EnglishSettings = New ClientApplicationInterfaceSettings;
	EnglishContent = New ClientApplicationInterfaceContentSettings;
	EnglishGroup = New ClientApplicationInterfaceContentSettingsGroup;
	EnglishItem = New ClientApplicationInterfaceContentSettingsItem("SectionsPanel");
	EnglishGroup.Add(EnglishItem);
	EnglishContent.Left.Add(EnglishGroup);
	Если EnglishItem.Name = "SectionsPanel" И EnglishGroup.Count() = 1 Тогда
		АнглийскиеИмена = "OK";
	КонецЕсли;
Исключение
КонецПопытки;
М("CLIENT.INTERFACE.ENGLISH", АнглийскиеИмена);

М("file.end", "client-interface-content-settings-1");
