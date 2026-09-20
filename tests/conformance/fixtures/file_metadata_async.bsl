// Oracle: filesystem/file-async-read.platform.txt. Локальная дата зависит от зоны host.
Асинх Функция ПробаЧтенияАсинх(ИдЧтения, ПутьЧтения, МетодЧтения, СинхМетодЧтения)
	ФайлЧтенияПробы = Новый Файл(ПутьЧтения);
	СтадияЧтения = "call";
	ТипОбещанияЧтения = "none";
	Попытка
		ОбещаниеЧтения = Вычислить("ФайлЧтенияПробы." + МетодЧтения + "()");
		ТипОбещанияЧтения = Строка(ТипЗнч(ОбещаниеЧтения));
		СтадияЧтения = "await";
		РезультатЧтения = Ждать ОбещаниеЧтения;
		СтадияЧтения = "sync";
		СинхЧтение = Вычислить("ФайлЧтенияПробы." + СинхМетодЧтения + "()");
		ТекстЧтения = Строка(РезультатЧтения);
		Если ТипЗнч(РезультатЧтения) = Тип("Дата") Тогда
			ТекстЧтения = ?(СинхМетодЧтения = "ПолучитьВремяИзменения" Или СинхМетодЧтения = "GetModificationTime", "<local>", Формат(РезультатЧтения, "ДФ=yyyyMMddHHmmss"));
		ИначеЕсли ТипЗнч(РезультатЧтения) = Тип("Булево") Тогда
			ТекстЧтения = ?(РезультатЧтения, "1", "0");
		КонецЕсли;
		ВыводЧтения = "ok|value=" + ТекстЧтения + "|sync_equal=" + ?(РезультатЧтения = СинхЧтение, "1", "0");
	Исключение
		ОшибкаЧтения = ИнформацияОбОшибке();
		ВыводЧтения = "error|stage=" + СтадияЧтения;
	КонецПопытки;
	Если ТипОбещанияЧтения <> "Promise" И ТипОбещанияЧтения <> "Обещание" Тогда ВызватьИсключение "ожидалось обещание"; КонецЕсли;
	Сообщить(ИдЧтения + Символ(9) + ВыводЧтения);
	Возврат Неопределено;
КонецФункции

Асинх Процедура ИзмеритьЧтениеАсинх()
	КореньЧтения = ПолучитьИмяВременногоФайла(".async-read");
	Сообщить("file.begin" + Символ(9) + "file-async-read-1");
	Попытка
		СоздатьКаталог(КореньЧтения + "/directory");
		ДанныеЧтения = ПолучитьДвоичныеДанныеИзСтроки("abc", КодировкаТекста.UTF8, Ложь);
		ДанныеЧтения.Записать(КореньЧтения + "/ordinary");
		ОбычныйФайлЧтения = Новый Файл(КореньЧтения + "/ordinary");
		ОбычныйФайлЧтения.УстановитьУниверсальноеВремяИзменения(Дата(2020, 1, 2, 3, 4, 5));
		КаталогЧтения = Новый Файл(КореньЧтения + "/directory");
		КаталогЧтения.УстановитьУниверсальноеВремяИзменения(Дата(2020, 1, 2, 3, 4, 5));
		Ждать ПробаЧтенияАсинх("read.ordinary.exists.ru", КореньЧтения + "/ordinary", "СуществуетАсинх", "Существует");
		Ждать ПробаЧтенияАсинх("read.ordinary.exists.en", КореньЧтения + "/ordinary", "ExistsAsync", "Exists");
		Ждать ПробаЧтенияАсинх("read.ordinary.isfile.ru", КореньЧтения + "/ordinary", "ЭтоФайлАсинх", "ЭтоФайл");
		Ждать ПробаЧтенияАсинх("read.ordinary.isfile.en", КореньЧтения + "/ordinary", "IsFileAsync", "IsFile");
		Ждать ПробаЧтенияАсинх("read.ordinary.isdir.ru", КореньЧтения + "/ordinary", "ЭтоКаталогАсинх", "ЭтоКаталог");
		Ждать ПробаЧтенияАсинх("read.ordinary.isdir.en", КореньЧтения + "/ordinary", "IsDirectoryAsync", "IsDirectory");
		Ждать ПробаЧтенияАсинх("read.ordinary.size.ru", КореньЧтения + "/ordinary", "РазмерАсинх", "Размер");
		Ждать ПробаЧтенияАсинх("read.ordinary.size.en", КореньЧтения + "/ordinary", "SizeAsync", "Size");
		Ждать ПробаЧтенияАсинх("read.ordinary.readonly.ru", КореньЧтения + "/ordinary", "ПолучитьТолькоЧтениеАсинх", "ПолучитьТолькоЧтение");
		Ждать ПробаЧтенияАсинх("read.ordinary.readonly.en", КореньЧтения + "/ordinary", "GetReadOnlyAsync", "GetReadOnly");
		Ждать ПробаЧтенияАсинх("read.ordinary.hidden.ru", КореньЧтения + "/ordinary", "ПолучитьНевидимостьАсинх", "ПолучитьНевидимость");
		Ждать ПробаЧтенияАсинх("read.ordinary.hidden.en", КореньЧтения + "/ordinary", "GetHiddenAsync", "GetHidden");
		Ждать ПробаЧтенияАсинх("read.ordinary.time.ru", КореньЧтения + "/ordinary", "ПолучитьВремяИзмененияАсинх", "ПолучитьВремяИзменения");
		Ждать ПробаЧтенияАсинх("read.ordinary.time.en", КореньЧтения + "/ordinary", "GetModificationTimeAsync", "GetModificationTime");
		Ждать ПробаЧтенияАсинх("read.ordinary.timeutc.ru", КореньЧтения + "/ordinary", "ПолучитьУниверсальноеВремяИзмененияАсинх", "ПолучитьУниверсальноеВремяИзменения");
		Ждать ПробаЧтенияАсинх("read.ordinary.timeutc.en", КореньЧтения + "/ordinary", "GetModificationUniversalTimeAsync", "GetModificationUniversalTime");
		Ждать ПробаЧтенияАсинх("read.directory.exists.ru", КореньЧтения + "/directory", "СуществуетАсинх", "Существует");
		Ждать ПробаЧтенияАсинх("read.directory.exists.en", КореньЧтения + "/directory", "ExistsAsync", "Exists");
		Ждать ПробаЧтенияАсинх("read.directory.isfile.ru", КореньЧтения + "/directory", "ЭтоФайлАсинх", "ЭтоФайл");
		Ждать ПробаЧтенияАсинх("read.directory.isfile.en", КореньЧтения + "/directory", "IsFileAsync", "IsFile");
		Ждать ПробаЧтенияАсинх("read.directory.isdir.ru", КореньЧтения + "/directory", "ЭтоКаталогАсинх", "ЭтоКаталог");
		Ждать ПробаЧтенияАсинх("read.directory.isdir.en", КореньЧтения + "/directory", "IsDirectoryAsync", "IsDirectory");
		Ждать ПробаЧтенияАсинх("read.directory.size.ru", КореньЧтения + "/directory", "РазмерАсинх", "Размер");
		Ждать ПробаЧтенияАсинх("read.directory.size.en", КореньЧтения + "/directory", "SizeAsync", "Size");
		Ждать ПробаЧтенияАсинх("read.directory.readonly.ru", КореньЧтения + "/directory", "ПолучитьТолькоЧтениеАсинх", "ПолучитьТолькоЧтение");
		Ждать ПробаЧтенияАсинх("read.directory.readonly.en", КореньЧтения + "/directory", "GetReadOnlyAsync", "GetReadOnly");
		Ждать ПробаЧтенияАсинх("read.directory.hidden.ru", КореньЧтения + "/directory", "ПолучитьНевидимостьАсинх", "ПолучитьНевидимость");
		Ждать ПробаЧтенияАсинх("read.directory.hidden.en", КореньЧтения + "/directory", "GetHiddenAsync", "GetHidden");
		Ждать ПробаЧтенияАсинх("read.directory.time.ru", КореньЧтения + "/directory", "ПолучитьВремяИзмененияАсинх", "ПолучитьВремяИзменения");
		Ждать ПробаЧтенияАсинх("read.directory.time.en", КореньЧтения + "/directory", "GetModificationTimeAsync", "GetModificationTime");
		Ждать ПробаЧтенияАсинх("read.directory.timeutc.ru", КореньЧтения + "/directory", "ПолучитьУниверсальноеВремяИзмененияАсинх", "ПолучитьУниверсальноеВремяИзменения");
		Ждать ПробаЧтенияАсинх("read.directory.timeutc.en", КореньЧтения + "/directory", "GetModificationUniversalTimeAsync", "GetModificationUniversalTime");
		Ждать ПробаЧтенияАсинх("read.missing.exists.ru", КореньЧтения + "/missing", "СуществуетАсинх", "Существует");
		Ждать ПробаЧтенияАсинх("read.missing.exists.en", КореньЧтения + "/missing", "ExistsAsync", "Exists");
		Ждать ПробаЧтенияАсинх("read.missing.isfile.ru", КореньЧтения + "/missing", "ЭтоФайлАсинх", "ЭтоФайл");
		Ждать ПробаЧтенияАсинх("read.missing.isfile.en", КореньЧтения + "/missing", "IsFileAsync", "IsFile");
		Ждать ПробаЧтенияАсинх("read.missing.isdir.ru", КореньЧтения + "/missing", "ЭтоКаталогАсинх", "ЭтоКаталог");
		Ждать ПробаЧтенияАсинх("read.missing.isdir.en", КореньЧтения + "/missing", "IsDirectoryAsync", "IsDirectory");
		Ждать ПробаЧтенияАсинх("read.missing.size.ru", КореньЧтения + "/missing", "РазмерАсинх", "Размер");
		Ждать ПробаЧтенияАсинх("read.missing.size.en", КореньЧтения + "/missing", "SizeAsync", "Size");
		Ждать ПробаЧтенияАсинх("read.missing.readonly.ru", КореньЧтения + "/missing", "ПолучитьТолькоЧтениеАсинх", "ПолучитьТолькоЧтение");
		Ждать ПробаЧтенияАсинх("read.missing.readonly.en", КореньЧтения + "/missing", "GetReadOnlyAsync", "GetReadOnly");
		Ждать ПробаЧтенияАсинх("read.missing.hidden.ru", КореньЧтения + "/missing", "ПолучитьНевидимостьАсинх", "ПолучитьНевидимость");
		Ждать ПробаЧтенияАсинх("read.missing.hidden.en", КореньЧтения + "/missing", "GetHiddenAsync", "GetHidden");
		Ждать ПробаЧтенияАсинх("read.missing.time.ru", КореньЧтения + "/missing", "ПолучитьВремяИзмененияАсинх", "ПолучитьВремяИзменения");
		Ждать ПробаЧтенияАсинх("read.missing.time.en", КореньЧтения + "/missing", "GetModificationTimeAsync", "GetModificationTime");
		Ждать ПробаЧтенияАсинх("read.missing.timeutc.ru", КореньЧтения + "/missing", "ПолучитьУниверсальноеВремяИзмененияАсинх", "ПолучитьУниверсальноеВремяИзменения");
		Ждать ПробаЧтенияАсинх("read.missing.timeutc.en", КореньЧтения + "/missing", "GetModificationUniversalTimeAsync", "GetModificationUniversalTime");
	Исключение
		Сообщить("file.setup_error" + Символ(9) + ИнформацияОбОшибке().Описание);
	КонецПопытки;
	Попытка
		УдалитьФайлы(КореньЧтения);
	Исключение
		Сообщить("file.cleanup_error" + Символ(9) + ИнформацияОбОшибке().Описание);
	КонецПопытки;
	Сообщить("file.end" + Символ(9) + "file-async-read-1");
	Попытка
		Выполнить("ЗавершитьРаботуСистемы(Ложь)");
	Исключение
	КонецПопытки;
КонецПроцедуры

ИзмеритьЧтениеАсинх();
