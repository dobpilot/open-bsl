// Лексические строки из file-create-layout.platform.txt; без файлового I/O.
Процедура ПробаПробеловПути(ИдПробеловПути, ПутьПробеловПути)
	ФайлПробеловПути = Новый Файл("/tmp/fixture/" + ПутьПробеловПути);
	Сообщить("lexical." + ИдПробеловПути + Символ(9)
		+ СтрЗаменить(ФайлПробеловПути.ПолноеИмя, "/tmp/fixture", "<root>"));
КонецПроцедуры

ПробаПробеловПути("spaces", "spaces/ spaced /leaf ");
ПробаПробеловПути("space-leaf", "space-leaf/leaf ");
ПробаПробеловПути("space-parent", "space-parent/branch /leaf");
ПробаПробеловПути("space-leading", "space-leading/ leaf");
