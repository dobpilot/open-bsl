Попытка
    Сообщить("image.native.function_guard.ascii=" + ЗамерыТранзитивногоРодителя.ЗначениеПробы("abcdef", 3));
    Сообщить("image.native.function_guard.cyrillic=" + ЗамерыТранзитивногоРодителя.ЗначениеПробы("Привет", 2));
    Сообщить("image.native.function_guard.zero=" + ЗамерыТранзитивногоРодителя.ЗначениеПробы("abc", 0));
    Сообщить("image.native.function_guard.long=" + ЗамерыТранзитивногоРодителя.ЗначениеПробы("abc", 20));
Исключение
    Сообщить("image.native.function_guard.error=" + ОписаниеОшибки());
КонецПопытки;
