Попытка
    ЗначениеОбработки2ФАПробы = ТипОбработкиНастроекВторогоФактораАутентификации.НеИспользовать;
    Сообщить("twofactor.processing.dont-use.string" + Символ(9) + Строка(ЗначениеОбработки2ФАПробы));
    Сообщить("twofactor.processing.type" + Символ(9) + Строка(ТипЗнч(ЗначениеОбработки2ФАПробы)));
    Сообщить("twofactor.processing.dont-use.alias" + Символ(9) + ?(ЗначениеОбработки2ФАПробы = SecondAuthenticationFactorSettingsProcessingType.DontUse, 1, 0));
    СледующаяОбработка2ФАПробы = ТипОбработкиНастроекВторогоФактораАутентификации.ИспользоватьСледующуюПриОшибке;
    Сообщить("twofactor.processing.use-next.string" + Символ(9) + Строка(СледующаяОбработка2ФАПробы));
    Сообщить("twofactor.processing.use-next.alias" + Символ(9) + ?(СледующаяОбработка2ФАПробы = SecondAuthenticationFactorSettingsProcessingType.UseNextIfFailed, 1, 0));
    Сообщить("twofactor.processing.distinct" + Символ(9) + ?(СледующаяОбработка2ФАПробы <> ЗначениеОбработки2ФАПробы, 1, 0));
Исключение
    Сообщить("twofactor.processing.error" + Символ(9) + ОписаниеОшибки());
КонецПопытки;
