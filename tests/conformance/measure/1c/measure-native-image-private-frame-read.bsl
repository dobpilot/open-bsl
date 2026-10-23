Попытка
    АргументКадра = 5;
    РезультатКадра = ЗамерыТранзитивногоРодителя.Проба0(АргументКадра);
    Сообщить("image.private-frame-read.slot0.type=" + Строка(ТипЗнч(РезультатКадра)));
    Сообщить("image.private-frame-read.slot0.value=" + Строка(РезультатКадра));
    Сообщить("image.private-frame-read.slot0.argument=" + Строка(АргументКадра));
Исключение
    Сообщить("image.private-frame-read.slot0.error=" + ОписаниеОшибки());
КонецПопытки;
Попытка
    АргументКадра = 5;
    РезультатКадра = ЗамерыТранзитивногоРодителя.Проба1(АргументКадра);
    Сообщить("image.private-frame-read.slot1.type=" + Строка(ТипЗнч(РезультатКадра)));
    Сообщить("image.private-frame-read.slot1.value=" + Строка(РезультатКадра));
    Сообщить("image.private-frame-read.slot1.argument=" + Строка(АргументКадра));
Исключение
    Сообщить("image.private-frame-read.slot1.error=" + ОписаниеОшибки());
КонецПопытки;
Попытка
    АргументКадра = 5;
    РезультатКадра = ЗамерыТранзитивногоРодителя.Проба2(АргументКадра);
    Сообщить("image.private-frame-read.slot2.type=" + Строка(ТипЗнч(РезультатКадра)));
    Сообщить("image.private-frame-read.slot2.value=" + Строка(РезультатКадра));
    Сообщить("image.private-frame-read.slot2.argument=" + Строка(АргументКадра));
Исключение
    Сообщить("image.private-frame-read.slot2.error=" + ОписаниеОшибки());
КонецПопытки;
Попытка
    АргументКадра = 5;
    РезультатКадра = ЗамерыТранзитивногоРодителя.Проба3(АргументКадра);
    Сообщить("image.private-frame-read.slot3.type=" + Строка(ТипЗнч(РезультатКадра)));
    Сообщить("image.private-frame-read.slot3.value=" + Строка(РезультатКадра));
    Сообщить("image.private-frame-read.slot3.argument=" + Строка(АргументКадра));
Исключение
    Сообщить("image.private-frame-read.slot3.error=" + ОписаниеОшибки());
КонецПопытки;
Сообщить("image.private-frame-read.done=1");
