Сообщить("native.overlap.entered=1");
Попытка
    Сообщить("native.overlap.shared=" + Строка(ЗамерыТранзитивногоРодителя.SharedProbe()));
    ОбещаниеПерекрытия = ЗамерыТранзитивногоРодителя.AsyncProbe();
    Сообщить("native.overlap.promise=" + Строка(Число(ТипЗнч(ОбещаниеПерекрытия) = Тип("Обещание"))));
Исключение
    Сообщить("native.overlap.failure=ERROR");
КонецПопытки;
