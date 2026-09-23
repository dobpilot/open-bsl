// PLATFORM-ONLY: проба фиксирует достигнутые нульаргументные транзакционные процедуры.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "transaction-surface-1");
М("TRANSACTION.ACTIVE.INITIAL", ТранзакцияАктивна());

РусскийОткат = Ложь;
Попытка
	НачатьТранзакцию();
	М("TRANSACTION.ACTIVE.INSIDE", ТранзакцияАктивна());
	ОтменитьТранзакцию();
	РусскийОткат = Истина;
Исключение
	РусскийОткат = Ложь;
КонецПопытки;
М("TRANSACTION.RU.ROLLBACK", РусскийОткат);
М("TRANSACTION.ACTIVE.AFTER.ROLLBACK", ТранзакцияАктивна());

РусскаяФиксация = Ложь;
Попытка
	НачатьТранзакцию();
	ЗафиксироватьТранзакцию();
	РусскаяФиксация = Истина;
Исключение
	РусскаяФиксация = Ложь;
КонецПопытки;
М("TRANSACTION.RU.COMMIT", РусскаяФиксация);

EnglishRollback = Ложь;
Попытка
	BeginTransaction();
	RollbackTransaction();
	EnglishRollback = Истина;
Исключение
	EnglishRollback = Ложь;
КонецПопытки;
М("TRANSACTION.EN.ROLLBACK", EnglishRollback);

EnglishCommit = Ложь;
Попытка
	BeginTransaction();
	М("TRANSACTION.ACTIVE.ENGLISH", TransactionActive());
	CommitTransaction();
	EnglishCommit = Истина;
Исключение
	EnglishCommit = Ложь;
КонецПопытки;
М("TRANSACTION.EN.COMMIT", EnglishCommit);
М("TRANSACTION.ACTIVE.AFTER.COMMIT", TransactionActive());
М("file.end", "transaction-surface-1");
