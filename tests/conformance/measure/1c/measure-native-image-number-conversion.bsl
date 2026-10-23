Попытка
    АргументПробы = 42;
    РезультатПробы = ЗамерыТранзитивногоРодителя.ЗначениеПробы(АргументПробы);
    Сообщить("image.native.number_conversion.number.value=" + Строка(РезультатПробы));
    Сообщить("image.native.number_conversion.number.type=" + Строка(?(ТипЗнч(РезультатПробы) = Тип("Число"), 1, 0)));
Исключение
    Сообщить("image.native.number_conversion.number.error=1");
КонецПопытки;
Попытка
    АргументПробы = 0;
    РезультатПробы = ЗамерыТранзитивногоРодителя.ЗначениеПробы(АргументПробы);
    Сообщить("image.native.number_conversion.zero.value=" + Строка(РезультатПробы));
    Сообщить("image.native.number_conversion.zero.type=" + Строка(?(ТипЗнч(РезультатПробы) = Тип("Число"), 1, 0)));
Исключение
    Сообщить("image.native.number_conversion.zero.error=1");
КонецПопытки;
Попытка
    АргументПробы = -3;
    РезультатПробы = ЗамерыТранзитивногоРодителя.ЗначениеПробы(АргументПробы);
    Сообщить("image.native.number_conversion.negative.value=" + Строка(РезультатПробы));
    Сообщить("image.native.number_conversion.negative.type=" + Строка(?(ТипЗнч(РезультатПробы) = Тип("Число"), 1, 0)));
Исключение
    Сообщить("image.native.number_conversion.negative.error=1");
КонецПопытки;
Попытка
    АргументПробы = "42";
    РезультатПробы = ЗамерыТранзитивногоРодителя.ЗначениеПробы(АргументПробы);
    Сообщить("image.native.number_conversion.text.value=" + Строка(РезультатПробы));
    Сообщить("image.native.number_conversion.text.type=" + Строка(?(ТипЗнч(РезультатПробы) = Тип("Число"), 1, 0)));
Исключение
    Сообщить("image.native.number_conversion.text.error=1");
КонецПопытки;
Попытка
    АргументПробы = "12.5";
    РезультатПробы = ЗамерыТранзитивногоРодителя.ЗначениеПробы(АргументПробы);
    Сообщить("image.native.number_conversion.fraction.scaled=" + Строка(РезультатПробы * 10));
    Сообщить("image.native.number_conversion.fraction.type=" + Строка(?(ТипЗнч(РезультатПробы) = Тип("Число"), 1, 0)));
Исключение
    Сообщить("image.native.number_conversion.fraction.error=1");
КонецПопытки;
Попытка
    АргументПробы = Истина;
    РезультатПробы = ЗамерыТранзитивногоРодителя.ЗначениеПробы(АргументПробы);
    Сообщить("image.native.number_conversion.true.value=" + Строка(РезультатПробы));
    Сообщить("image.native.number_conversion.true.type=" + Строка(?(ТипЗнч(РезультатПробы) = Тип("Число"), 1, 0)));
Исключение
    Сообщить("image.native.number_conversion.true.error=1");
КонецПопытки;
Попытка
    АргументПробы = Ложь;
    РезультатПробы = ЗамерыТранзитивногоРодителя.ЗначениеПробы(АргументПробы);
    Сообщить("image.native.number_conversion.false.value=" + Строка(РезультатПробы));
    Сообщить("image.native.number_conversion.false.type=" + Строка(?(ТипЗнч(РезультатПробы) = Тип("Число"), 1, 0)));
Исключение
    Сообщить("image.native.number_conversion.false.error=1");
КонецПопытки;
Попытка
    АргументПробы = "проба";
    РезультатПробы = ЗамерыТранзитивногоРодителя.ЗначениеПробы(АргументПробы);
    Сообщить("image.native.number_conversion.bad.value=" + Строка(РезультатПробы));
    Сообщить("image.native.number_conversion.bad.type=" + Строка(?(ТипЗнч(РезультатПробы) = Тип("Число"), 1, 0)));
Исключение
    Сообщить("image.native.number_conversion.bad.error=1");
КонецПопытки;
Попытка
    АргументПробы = Неопределено;
    РезультатПробы = ЗамерыТранзитивногоРодителя.ЗначениеПробы(АргументПробы);
    Сообщить("image.native.number_conversion.undefined.value=" + Строка(РезультатПробы));
    Сообщить("image.native.number_conversion.undefined.type=" + Строка(?(ТипЗнч(РезультатПробы) = Тип("Число"), 1, 0)));
Исключение
    Сообщить("image.native.number_conversion.undefined.error=1");
КонецПопытки;
Попытка
    АргументПробы = Null;
    РезультатПробы = ЗамерыТранзитивногоРодителя.ЗначениеПробы(АргументПробы);
    Сообщить("image.native.number_conversion.null.value=" + Строка(РезультатПробы));
    Сообщить("image.native.number_conversion.null.type=" + Строка(?(ТипЗнч(РезультатПробы) = Тип("Число"), 1, 0)));
Исключение
    Сообщить("image.native.number_conversion.null.error=1");
КонецПопытки;
Попытка
    АргументПробы = 99;
    ЗамерыТранзитивногоРодителя.ЗаписьПробы(АргументПробы);
    Сообщить("image.native.number_conversion.reference=" + Строка(АргументПробы));
Исключение
    Сообщить("image.native.number_conversion.reference.error=1");
КонецПопытки;
