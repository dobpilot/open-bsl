Сообщить("native.external.slot.entered=1");
Попытка
    Сообщить("native.external.slot.argument=" + Строка(ЗамерыТранзитивногоРодителя.Probe(42)));
Исключение
    Сообщить("native.external.slot.argument=ERROR");
КонецПопытки;
Попытка
    РезультатБезАргумента = Вычислить("ЗамерыТранзитивногоРодителя.Probe()");
    Сообщить("native.external.slot.omitted.undefined=" + Строка(Число(РезультатБезАргумента = Неопределено)));
Исключение
    Сообщить("native.external.slot.omitted=ERROR");
КонецПопытки;
