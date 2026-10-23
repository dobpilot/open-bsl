// PLATFORM-ONLY: проба фиксирует достигнутый срез настроек клиентского приложения.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "client-settings-1");

Настройки = Новый НастройкиКлиентскогоПриложения;
М("CLIENT.SETTINGS.TYPE", ТипЗнч(Настройки));
М("CLIENT.SETTINGS.DEFAULT.NAVIGATION", Настройки.ОтображатьПанелиНавигацииИДействий);
М("CLIENT.SETTINGS.DEFAULT.SECTIONS", Настройки.ОтображатьПанельРазделов);
М("CLIENT.SETTINGS.DEFAULT.OPENING", Настройки.РежимОткрытияФормПриложения);
М("CLIENT.SETTINGS.DEFAULT.INTERFACE", Настройки.ВариантИнтерфейсаКлиентскогоПриложения);

Настройки.ОтображатьПанелиНавигацииИДействий = Ложь;
Настройки.ОтображатьПанельРазделов = Истина;
Настройки.РежимОткрытияФормПриложения = РежимОткрытияФормПриложения.Закладки;
Настройки.ВариантИнтерфейсаКлиентскогоПриложения = ВариантИнтерфейсаКлиентскогоПриложения.Такси;
М("CLIENT.SETTINGS.MUTATED.NAVIGATION", Настройки.ОтображатьПанелиНавигацииИДействий);
М("CLIENT.SETTINGS.MUTATED.SECTIONS", Настройки.ОтображатьПанельРазделов);
М("CLIENT.SETTINGS.MUTATED.OPENING", Настройки.РежимОткрытияФормПриложения);
М("CLIENT.SETTINGS.MUTATED.INTERFACE", Настройки.ВариантИнтерфейсаКлиентскогоПриложения);

М("CLIENT.SETTINGS.OPENING.TABS", РежимОткрытияФормПриложения.Закладки);
М("CLIENT.SETTINGS.OPENING.WINDOWS", РежимОткрытияФормПриложения.ОтдельныеОкна);
М("CLIENT.SETTINGS.INTERFACE.8_2", ВариантИнтерфейсаКлиентскогоПриложения.Версия8_2);
М("CLIENT.SETTINGS.INTERFACE.TAXI", ВариантИнтерфейсаКлиентскогоПриложения.Такси);
М("CLIENT.SETTINGS.OPENING.TYPE", ТипЗнч(РежимОткрытияФормПриложения.Закладки));
М("CLIENT.SETTINGS.INTERFACE.TYPE", ТипЗнч(ВариантИнтерфейсаКлиентскогоПриложения.Такси));

АнглийскиеИмена = "ERROR";
Попытка
	English = New ClientSettings;
	English.ShowNavigationAndActionsPanels = False;
	English.ShowSectionsPanel = True;
	English.ApplicationFormsOpenningMode = ApplicationFormsOpenningMode.Tabs;
	English.ClientApplicationInterfaceVariant = ClientApplicationInterfaceVariant.Taxi;
	Если Не English.ShowNavigationAndActionsPanels И English.ShowSectionsPanel Тогда
		АнглийскиеИмена = "OK";
	КонецЕсли;
Исключение
КонецПопытки;
М("CLIENT.SETTINGS.ENGLISH", АнглийскиеИмена);

НеверныйТип = "ACCEPTED";
Попытка
	Настройки.РежимОткрытияФормПриложения = ВариантИнтерфейсаКлиентскогоПриложения.Такси;
Исключение
	НеверныйТип = "ERROR";
КонецПопытки;
М("CLIENT.SETTINGS.BAD.ENUM", НеверныйТип);

НеверноеБулево = "ACCEPTED";
Попытка
	Настройки.ОтображатьПанельРазделов = "Да";
Исключение
	НеверноеБулево = "ERROR";
КонецПопытки;
М("CLIENT.SETTINGS.BAD.BOOLEAN", НеверноеБулево);

М("file.end", "client-settings-1");
