// Серверная характеризация особого sfile-представления имён.
// Для 1С placeholder заменяется корнем дерева, созданного средствами ОС.

Функция ЭкранироватьSFile(ТекстSFile, КореньSFile)
	РезультатSFile = СтрЗаменить(Строка(ТекстSFile), КореньSFile, "<root>");
	РезультатSFile = СтрЗаменить(РезультатSFile, Символ(9), "<TAB>");
	РезультатSFile = СтрЗаменить(РезультатSFile, Символ(10), "<LF>");
	Возврат РезультатSFile;
КонецФункции

Функция СвойствоSFile(ОбъектSFile, ИмяСвойстваSFile, КореньSFile)
	Попытка
		Возврат "ok:" + ЭкранироватьSFile(Вычислить("ОбъектSFile." + ИмяСвойстваSFile), КореньSFile);
	Исключение
		Возврат "error";
	КонецПопытки;
КонецФункции

Функция МетодSFile(ОбъектSFile, ИмяМетодаSFile, КореньSFile)
	Попытка
		Возврат "ok:" + ЭкранироватьSFile(Вычислить("ОбъектSFile." + ИмяМетодаSFile + "()"), КореньSFile);
	Исключение
		Возврат "error";
	КонецПопытки;
КонецФункции

Функция ОписаниеSFile(ОбъектSFile, КореньSFile)
	Возврат "string=" + ЭкранироватьSFile(ОбъектSFile, КореньSFile)
		+ "|full=" + СвойствоSFile(ОбъектSFile, "ПолноеИмя", КореньSFile)
		+ "|path=" + СвойствоSFile(ОбъектSFile, "Путь", КореньSFile)
		+ "|name=" + СвойствоSFile(ОбъектSFile, "Имя", КореньSFile)
		+ "|base=" + СвойствоSFile(ОбъектSFile, "ИмяБезРасширения", КореньSFile)
		+ "|ext=" + СвойствоSFile(ОбъектSFile, "Расширение", КореньSFile)
		+ "|exists=" + МетодSFile(ОбъектSFile, "Существует", КореньSFile)
		+ "|isfile=" + МетодSFile(ОбъектSFile, "ЭтоФайл", КореньSFile)
		+ "|isdir=" + МетодSFile(ОбъектSFile, "ЭтоКаталог", КореньSFile)
		+ "|size=" + МетодSFile(ОбъектSFile, "Размер", КореньSFile);
КонецФункции

Процедура ПробаПоискаSFile(ИдSFile, КореньSFile, ВыражениеSFile)
	Попытка
		МассивSFile = Вычислить(ВыражениеSFile);
		РезультатSFile = "ok:count=" + Строка(МассивSFile.Количество());
		Для ИндексSFile = 0 По МассивSFile.Количество() - 1 Цикл
			РезультатSFile = РезультатSFile + "|item" + Строка(ИндексSFile)
				+ "{" + ОписаниеSFile(МассивSFile[ИндексSFile], КореньSFile) + "}";
		КонецЦикла;
	Исключение
		РезультатSFile = "error";
	КонецПопытки;
	Сообщить(ИдSFile + Символ(9) + РезультатSFile);
КонецПроцедуры

Сообщить("file.begin" + Символ(9) + "search-sfile-2");
КореньSFile = "__OPEN_BSL_SFILE_ROOT__";
СвойКореньSFile = Лев(КореньSFile, 1) = "_";
Попытка
	Если СвойКореньSFile Тогда
		КореньSFile = ПолучитьИмяВременногоФайла(".search-sfile");
		СоздатьКаталог(КореньSFile);
		СоздатьКаталог(КореньSFile + "/star");
		СоздатьКаталог(КореньSFile + "/star-prefix");
		СоздатьКаталог(КореньSFile + "/star-middle");
		СоздатьКаталог(КореньSFile + "/star-double");
		СоздатьКаталог(КореньSFile + "/star-only");
		СоздатьКаталог(КореньSFile + "/star-extension");
		СоздатьКаталог(КореньSFile + "/question");
		СоздатьКаталог(КореньSFile + "/backslash");
		СоздатьКаталог(КореньSFile + "/backslash/a");
		ДанныеSFile = ПолучитьДвоичныеДанныеИзСтроки("ABC", КодировкаТекста.UTF8, Ложь);
		ДанныеSFile.Записать(КореньSFile + "/star/a*");
		ДанныеSFile.Записать(КореньSFile + "/star-prefix/*a");
		ДанныеSFile.Записать(КореньSFile + "/star-middle/a*b");
		ДанныеSFile.Записать(КореньSFile + "/star-double/a**");
		ДанныеSFile.Записать(КореньSFile + "/star-only/*");
		ДанныеSFile.Записать(КореньSFile + "/star-extension/a*.txt");
		ДанныеSFile.Записать(КореньSFile + "/question/a?");
		ДанныеSFile.Записать(КореньSFile + "/backslash/a\b");
	КонецЕсли;

	ПрямойSFile = Новый Файл(КореньSFile + "/star/a*");
	Сообщить("star.direct" + Символ(9) + ОписаниеSFile(ПрямойSFile, КореньSFile));
	ПробаПоискаSFile("star.mask.literal", КореньSFile,
		"НайтиФайлы(КореньSFile + ""/star"", ""a\*"")");
	ПробаПоискаSFile("star.mask.name", КореньSFile,
		"НайтиФайлы(КореньSFile + ""/star"", ""a*"")");
	ПробаПоискаSFile("star.mask.all", КореньSFile,
		"НайтиФайлы(КореньSFile + ""/star"", ""*"")");
	ПробаПоискаSFile("star.mask.two", КореньSFile,
		"НайтиФайлы(КореньSFile + ""/star"", ""??"")");
	ПробаПоискаSFile("star.path", КореньSFile,
		"НайтиФайлы(КореньSFile + ""/star/a*"")");

	ПрямойSFile = Новый Файл(КореньSFile + "/star-prefix/*a");
	Сообщить("star-prefix.direct" + Символ(9) + ОписаниеSFile(ПрямойSFile, КореньSFile));
	ПробаПоискаSFile("star-prefix.mask.literal", КореньSFile,
		"НайтиФайлы(КореньSFile + ""/star-prefix"", ""\*a"")");
	ПробаПоискаSFile("star-prefix.mask.name", КореньSFile,
		"НайтиФайлы(КореньSFile + ""/star-prefix"", ""*a"")");
	ПробаПоискаSFile("star-prefix.mask.all", КореньSFile,
		"НайтиФайлы(КореньSFile + ""/star-prefix"", ""*"")");
	ПробаПоискаSFile("star-prefix.mask.two", КореньSFile,
		"НайтиФайлы(КореньSFile + ""/star-prefix"", ""??"")");

	ПрямойSFile = Новый Файл(КореньSFile + "/star-middle/a*b");
	Сообщить("star-middle.direct" + Символ(9) + ОписаниеSFile(ПрямойSFile, КореньSFile));
	ПробаПоискаSFile("star-middle.mask.literal", КореньSFile,
		"НайтиФайлы(КореньSFile + ""/star-middle"", ""a\*b"")");
	ПробаПоискаSFile("star-middle.mask.name", КореньSFile,
		"НайтиФайлы(КореньSFile + ""/star-middle"", ""a*b"")");
	ПробаПоискаSFile("star-middle.mask.all", КореньSFile,
		"НайтиФайлы(КореньSFile + ""/star-middle"", ""*"")");
	ПробаПоискаSFile("star-middle.mask.three", КореньSFile,
		"НайтиФайлы(КореньSFile + ""/star-middle"", ""???"")");

	ПрямойSFile = Новый Файл(КореньSFile + "/star-double/a**");
	Сообщить("star-double.direct" + Символ(9) + ОписаниеSFile(ПрямойSFile, КореньSFile));
	ПробаПоискаSFile("star-double.mask.literal", КореньSFile,
		"НайтиФайлы(КореньSFile + ""/star-double"", ""a\*\*"")");
	ПробаПоискаSFile("star-double.mask.name", КореньSFile,
		"НайтиФайлы(КореньSFile + ""/star-double"", ""a**"")");
	ПробаПоискаSFile("star-double.mask.all", КореньSFile,
		"НайтиФайлы(КореньSFile + ""/star-double"", ""*"")");
	ПробаПоискаSFile("star-double.mask.three", КореньSFile,
		"НайтиФайлы(КореньSFile + ""/star-double"", ""???"")");

	ПрямойSFile = Новый Файл(КореньSFile + "/star-only/*");
	Сообщить("star-only.direct" + Символ(9) + ОписаниеSFile(ПрямойSFile, КореньSFile));
	ПробаПоискаSFile("star-only.mask.literal", КореньSFile,
		"НайтиФайлы(КореньSFile + ""/star-only"", ""\*"")");
	ПробаПоискаSFile("star-only.mask.all", КореньSFile,
		"НайтиФайлы(КореньSFile + ""/star-only"", ""*"")");
	ПробаПоискаSFile("star-only.mask.one", КореньSFile,
		"НайтиФайлы(КореньSFile + ""/star-only"", ""?"")");

	ПрямойSFile = Новый Файл(КореньSFile + "/star-extension/a*.txt");
	Сообщить("star-extension.direct" + Символ(9) + ОписаниеSFile(ПрямойSFile, КореньSFile));
	ПробаПоискаSFile("star-extension.mask.literal", КореньSFile,
		"НайтиФайлы(КореньSFile + ""/star-extension"", ""a\*.txt"")");
	ПробаПоискаSFile("star-extension.mask.name", КореньSFile,
		"НайтиФайлы(КореньSFile + ""/star-extension"", ""a*.txt"")");
	ПробаПоискаSFile("star-extension.mask.all", КореньSFile,
		"НайтиФайлы(КореньSFile + ""/star-extension"", ""*"")");
	ПробаПоискаSFile("star-extension.mask.six", КореньSFile,
		"НайтиФайлы(КореньSFile + ""/star-extension"", ""??????"")");

	ПрямойSFile = Новый Файл(КореньSFile + "/question/a?");
	Сообщить("question.direct" + Символ(9) + ОписаниеSFile(ПрямойSFile, КореньSFile));
	ПробаПоискаSFile("question.mask.literal", КореньSFile,
		"НайтиФайлы(КореньSFile + ""/question"", ""a\?"")");
	ПробаПоискаSFile("question.mask.name", КореньSFile,
		"НайтиФайлы(КореньSFile + ""/question"", ""a?"")");
	ПробаПоискаSFile("question.mask.all", КореньSFile,
		"НайтиФайлы(КореньSFile + ""/question"", ""*"")");
	ПробаПоискаSFile("question.path", КореньSFile,
		"НайтиФайлы(КореньSFile + ""/question/a?"")");

	ПрямойSFile = Новый Файл(КореньSFile + "/backslash/a\b");
	Сообщить("backslash.direct" + Символ(9) + ОписаниеSFile(ПрямойSFile, КореньSFile));
	ПробаПоискаSFile("backslash.mask.name", КореньSFile,
		"НайтиФайлы(КореньSFile + ""/backslash"", ""a*"")");
	ПробаПоискаSFile("backslash.mask.all", КореньSFile,
		"НайтиФайлы(КореньSFile + ""/backslash"", ""*"")");
	ПробаПоискаSFile("backslash.path", КореньSFile,
		"НайтиФайлы(КореньSFile + ""/backslash/a\b"")");
Исключение
	Сообщить("file.setup_error" + Символ(9) + ИнформацияОбОшибке().Описание);
КонецПопытки;

Если СвойКореньSFile Тогда
	Попытка
		УдалитьФайлы(КореньSFile);
	Исключение
		Сообщить("file.cleanup_error" + Символ(9) + ИнформацияОбОшибке().Описание);
	КонецПопытки;
КонецЕсли;
Сообщить("file.end" + Символ(9) + "search-sfile-2");
