// PLATFORM-ONLY: пустые транзакции в собственной базе, без записи данных.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

Функция ПробаОперации(ТекстОперации)
	Попытка
		Выполнить(ТекстОперации);
		Возврат Истина;
	Исключение
		Возврат Ложь;
	КонецПопытки;
КонецФункции

М("file.begin", "session-transaction-nesting-1");
М("SESSION_TRANSACTION.EMPTY_COMMIT", ПробаОперации("ЗафиксироватьТранзакцию();"));
М("SESSION_TRANSACTION.EMPTY_ROLLBACK", ПробаОперации("ОтменитьТранзакцию();"));
М("SESSION_TRANSACTION.BEGIN", ПробаОперации("НачатьТранзакцию();"));
М("SESSION_TRANSACTION.NESTED_BEGIN", ПробаОперации("НачатьТранзакцию();"));
М("SESSION_TRANSACTION.INNER_COMMIT", ПробаОперации("ЗафиксироватьТранзакцию();"));
М("SESSION_TRANSACTION.ACTIVE_AFTER_INNER_COMMIT", ТранзакцияАктивна());
М("SESSION_TRANSACTION.OUTER_COMMIT", ПробаОперации("ЗафиксироватьТранзакцию();"));
М("SESSION_TRANSACTION.ACTIVE_AFTER_OUTER_COMMIT", ТранзакцияАктивна());
М("SESSION_TRANSACTION.ROLLBACK_BEGIN", ПробаОперации("НачатьТранзакцию();"));
М("SESSION_TRANSACTION.ROLLBACK_NESTED_BEGIN", ПробаОперации("НачатьТранзакцию();"));
М("SESSION_TRANSACTION.INNER_ROLLBACK", ПробаОперации("ОтменитьТранзакцию();"));
М("SESSION_TRANSACTION.ACTIVE_AFTER_INNER_ROLLBACK", ТранзакцияАктивна());
М("SESSION_TRANSACTION.COMMIT_AFTER_INNER_ROLLBACK", ПробаОперации("ЗафиксироватьТранзакцию();"));
М("SESSION_TRANSACTION.ACTIVE_AFTER_ROLLBACK_COMMIT", ТранзакцияАктивна());
М("SESSION_TRANSACTION.OUTER_ROLLBACK", ПробаОперации("ОтменитьТранзакцию();"));
М("SESSION_TRANSACTION.ACTIVE_AFTER_OUTER_ROLLBACK", ТранзакцияАктивна());
М("SESSION_TRANSACTION.CLEANUP", ПробаОперации("ОтменитьТранзакцию();"));
М("file.end", "session-transaction-nesting-1");
