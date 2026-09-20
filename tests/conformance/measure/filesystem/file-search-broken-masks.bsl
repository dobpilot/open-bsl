// Только чтение подготовленного дерева; все цели ссылок принадлежат этой пробе.
Процедура ПробаПоискаСсылок(ИдСсылок, ПутьСсылок, МаскаСсылок, РекурсияСсылок, КореньСсылок)
	Попытка
		НайденныеСсылки = НайтиФайлы(ПутьСсылок, МаскаСсылок, РекурсияСсылок);
		ТекстСсылок = "ok|count=" + Строка(НайденныеСсылки.Количество());
		Для Каждого ЭлементСсылок Из НайденныеСсылки Цикл
			ТекстСсылок = ТекстСсылок + "|" + СтрЗаменить(ЭлементСсылок.ПолноеИмя, КореньСсылок, "<root>");
		КонецЦикла;
		Сообщить(ИдСсылок + Символ(9) + ТекстСсылок);
	Исключение
		Сообщить(ИдСсылок + Символ(9) + "error|" + ИнформацияОбОшибке().Описание);
	КонецПопытки;
КонецПроцедуры

КореньСсылок = "/tmp/open-bsl-find-broken-masks-Sr1Zha/tree";
Сообщить("file.begin" + Символ(9) + "file-search-broken-masks-1");
ПробаПоискаСсылок("broken.control.star.flat", КореньСсылок + "/control", "*", Ложь, КореньСсылок);
ПробаПоискаСсылок("broken.control.star.recursive", КореньСсылок + "/control", "*", Истина, КореньСсылок);
ПробаПоискаСсылок("broken.control.star_dot.flat", КореньСсылок + "/control", "*.*", Ложь, КореньСсылок);
ПробаПоискаСсылок("broken.control.star_dot.recursive", КореньСсылок + "/control", "*.*", Истина, КореньСсылок);
ПробаПоискаСсылок("broken.control.txt.flat", КореньСсылок + "/control", "*.txt", Ложь, КореньСсылок);
ПробаПоискаСсылок("broken.control.txt.recursive", КореньСсылок + "/control", "*.txt", Истина, КореньСсылок);
ПробаПоискаСсылок("broken.control.broken.flat", КореньСсылок + "/control", "broken.txt", Ложь, КореньСсылок);
ПробаПоискаСсылок("broken.control.broken.recursive", КореньСсылок + "/control", "broken.txt", Истина, КореньСсылок);
ПробаПоискаСсылок("broken.control.plain.flat", КореньСсылок + "/control", "plain.txt", Ложь, КореньСсылок);
ПробаПоискаСсылок("broken.control.plain.recursive", КореньСсылок + "/control", "plain.txt", Истина, КореньСсылок);
ПробаПоискаСсылок("broken.control.branch.flat", КореньСсылок + "/control", "branch.txt", Ложь, КореньСсылок);
ПробаПоискаСсылок("broken.control.branch.recursive", КореньСсылок + "/control", "branch.txt", Истина, КореньСсылок);
ПробаПоискаСсылок("broken.control.bin.flat", КореньСсылок + "/control", "*.bin", Ложь, КореньСсылок);
ПробаПоискаСсылок("broken.control.bin.recursive", КореньСсылок + "/control", "*.bin", Истина, КореньСсылок);
ПробаПоискаСсылок("broken.control.suffix.flat", КореньСсылок + "/control", "*t", Ложь, КореньСсылок);
ПробаПоискаСсылок("broken.control.suffix.recursive", КореньСсылок + "/control", "*t", Истина, КореньСсылок);
ПробаПоискаСсылок("broken.early.star.flat", КореньСсылок + "/early", "*", Ложь, КореньСсылок);
ПробаПоискаСсылок("broken.early.star.recursive", КореньСсылок + "/early", "*", Истина, КореньСсылок);
ПробаПоискаСсылок("broken.early.star_dot.flat", КореньСсылок + "/early", "*.*", Ложь, КореньСсылок);
ПробаПоискаСсылок("broken.early.star_dot.recursive", КореньСсылок + "/early", "*.*", Истина, КореньСсылок);
ПробаПоискаСсылок("broken.early.txt.flat", КореньСсылок + "/early", "*.txt", Ложь, КореньСсылок);
ПробаПоискаСсылок("broken.early.txt.recursive", КореньСсылок + "/early", "*.txt", Истина, КореньСсылок);
ПробаПоискаСсылок("broken.early.broken.flat", КореньСсылок + "/early", "broken.txt", Ложь, КореньСсылок);
ПробаПоискаСсылок("broken.early.broken.recursive", КореньСсылок + "/early", "broken.txt", Истина, КореньСсылок);
ПробаПоискаСсылок("broken.early.plain.flat", КореньСсылок + "/early", "plain.txt", Ложь, КореньСсылок);
ПробаПоискаСсылок("broken.early.plain.recursive", КореньСсылок + "/early", "plain.txt", Истина, КореньСсылок);
ПробаПоискаСсылок("broken.early.branch.flat", КореньСсылок + "/early", "branch.txt", Ложь, КореньСсылок);
ПробаПоискаСсылок("broken.early.branch.recursive", КореньСсылок + "/early", "branch.txt", Истина, КореньСсылок);
ПробаПоискаСсылок("broken.early.bin.flat", КореньСсылок + "/early", "*.bin", Ложь, КореньСсылок);
ПробаПоискаСсылок("broken.early.bin.recursive", КореньСсылок + "/early", "*.bin", Истина, КореньСсылок);
ПробаПоискаСсылок("broken.early.suffix.flat", КореньСсылок + "/early", "*t", Ложь, КореньСсылок);
ПробаПоискаСсылок("broken.early.suffix.recursive", КореньСсылок + "/early", "*t", Истина, КореньСсылок);
ПробаПоискаСсылок("broken.late.star.flat", КореньСсылок + "/late", "*", Ложь, КореньСсылок);
ПробаПоискаСсылок("broken.late.star.recursive", КореньСсылок + "/late", "*", Истина, КореньСсылок);
ПробаПоискаСсылок("broken.late.star_dot.flat", КореньСсылок + "/late", "*.*", Ложь, КореньСсылок);
ПробаПоискаСсылок("broken.late.star_dot.recursive", КореньСсылок + "/late", "*.*", Истина, КореньСсылок);
ПробаПоискаСсылок("broken.late.txt.flat", КореньСсылок + "/late", "*.txt", Ложь, КореньСсылок);
ПробаПоискаСсылок("broken.late.txt.recursive", КореньСсылок + "/late", "*.txt", Истина, КореньСсылок);
ПробаПоискаСсылок("broken.late.broken.flat", КореньСсылок + "/late", "broken.txt", Ложь, КореньСсылок);
ПробаПоискаСсылок("broken.late.broken.recursive", КореньСсылок + "/late", "broken.txt", Истина, КореньСсылок);
ПробаПоискаСсылок("broken.late.plain.flat", КореньСсылок + "/late", "plain.txt", Ложь, КореньСсылок);
ПробаПоискаСсылок("broken.late.plain.recursive", КореньСсылок + "/late", "plain.txt", Истина, КореньСсылок);
ПробаПоискаСсылок("broken.late.branch.flat", КореньСсылок + "/late", "branch.txt", Ложь, КореньСсылок);
ПробаПоискаСсылок("broken.late.branch.recursive", КореньСсылок + "/late", "branch.txt", Истина, КореньСсылок);
ПробаПоискаСсылок("broken.late.bin.flat", КореньСсылок + "/late", "*.bin", Ложь, КореньСсылок);
ПробаПоискаСсылок("broken.late.bin.recursive", КореньСсылок + "/late", "*.bin", Истина, КореньСсылок);
ПробаПоискаСсылок("broken.late.suffix.flat", КореньСсылок + "/late", "*t", Ложь, КореньСсылок);
ПробаПоискаСсылок("broken.late.suffix.recursive", КореньСсылок + "/late", "*t", Истина, КореньСсылок);
Сообщить("file.end" + Символ(9) + "file-search-broken-masks-1");
