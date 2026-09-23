// PLATFORM-ONLY: проба фиксирует достигнутые нульаргументные транзакционные процедуры.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "transaction-surface-1");

РусскийОткат = Ложь;
Попытка
	НачатьТранзакцию();
	ОтменитьТранзакцию();
	РусскийОткат = Истина;
Исключение
	РусскийОткат = Ложь;
КонецПопытки;
М("TRANSACTION.RU.ROLLBACK", РусскийОткат);

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
	CommitTransaction();
	EnglishCommit = Истина;
Исключение
	EnglishCommit = Ложь;
КонецПопытки;
М("TRANSACTION.EN.COMMIT", EnglishCommit);
М("file.end", "transaction-surface-1");
