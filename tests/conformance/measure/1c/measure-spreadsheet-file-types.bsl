// PLATFORM-ONLY: проба фиксирует достигнутые модулем сеанса члены типа файла.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "spreadsheet-file-types-1");
М("SPREAD.ANSITXT.STRING", ТипФайлаТабличногоДокумента.ANSITXT);
М("SPREAD.ANSITXT.ALIAS", ТипФайлаТабличногоДокумента.ANSITXT = SpreadsheetDocumentFileType.ANSITXT);
М("SPREAD.DOCX.STRING", ТипФайлаТабличногоДокумента.DOCX);
М("SPREAD.DOCX.ALIAS", ТипФайлаТабличногоДокумента.DOCX = SpreadsheetDocumentFileType.DOCX);
М("SPREAD.HTML5.STRING", ТипФайлаТабличногоДокумента.HTML5);
М("SPREAD.HTML5.ALIAS", ТипФайлаТабличногоДокумента.HTML5 = SpreadsheetDocumentFileType.HTML5);
М("SPREAD.ODS.STRING", ТипФайлаТабличногоДокумента.ODS);
М("SPREAD.ODS.ALIAS", ТипФайлаТабличногоДокумента.ODS = SpreadsheetDocumentFileType.ODS);
М("SPREAD.XLS.STRING", ТипФайлаТабличногоДокумента.XLS);
М("SPREAD.XLS.TYPE", ТипЗнч(ТипФайлаТабличногоДокумента.XLS));
М("SPREAD.XLS.ALIAS", ТипФайлаТабличногоДокумента.XLS = SpreadsheetDocumentFileType.XLS);
М("file.end", "spreadsheet-file-types-1");
