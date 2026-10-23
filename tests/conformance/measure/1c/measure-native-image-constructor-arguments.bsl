// Нативная проба конструктора массива с количеством измерений от нуля до шести.
Попытка
    МассивПробы = ЗамерыТранзитивногоРодителя.Проба0();
    Сообщить("image.native.constructor.arguments.0.count=" + Строка(МассивПробы.Количество()));
    Сообщить("image.native.constructor.arguments.0.type=" + Строка(?(ТипЗнч(МассивПробы) = Тип("Массив"), 1, 0)));
    МассивПробы = ЗамерыТранзитивногоРодителя.Проба1(2);
    Сообщить("image.native.constructor.arguments.1.count=" + Строка(МассивПробы.Количество()));
    Сообщить("image.native.constructor.arguments.1.type=" + Строка(?(ТипЗнч(МассивПробы) = Тип("Массив"), 1, 0)));
    МассивПробы = ЗамерыТранзитивногоРодителя.Проба2(2, 3);
    Сообщить("image.native.constructor.arguments.2.count=" + Строка(МассивПробы.Количество()));
    Сообщить("image.native.constructor.arguments.2.second=" + Строка(МассивПробы[0].Количество()));
    Сообщить("image.native.constructor.arguments.2.type=" + Строка(?(ТипЗнч(МассивПробы) = Тип("Массив"), 1, 0)));
    МассивПробы = ЗамерыТранзитивногоРодителя.Проба3(2, 3, 1);
    Сообщить("image.native.constructor.arguments.3.count=" + Строка(МассивПробы.Количество()));
    Сообщить("image.native.constructor.arguments.3.second=" + Строка(МассивПробы[0].Количество()));
    Сообщить("image.native.constructor.arguments.3.last=" + Строка(МассивПробы[0][0].Количество()));
    Сообщить("image.native.constructor.arguments.3.type=" + Строка(?(ТипЗнч(МассивПробы) = Тип("Массив"), 1, 0)));
    МассивПробы = ЗамерыТранзитивногоРодителя.Проба4(2, 3, 1, 1);
    Сообщить("image.native.constructor.arguments.4.count=" + Строка(МассивПробы.Количество()));
    Сообщить("image.native.constructor.arguments.4.second=" + Строка(МассивПробы[0].Количество()));
    Сообщить("image.native.constructor.arguments.4.last=" + Строка(МассивПробы[0][0][0].Количество()));
    Сообщить("image.native.constructor.arguments.4.type=" + Строка(?(ТипЗнч(МассивПробы) = Тип("Массив"), 1, 0)));
    МассивПробы = ЗамерыТранзитивногоРодителя.Проба5(2, 3, 1, 1, 1);
    Сообщить("image.native.constructor.arguments.5.count=" + Строка(МассивПробы.Количество()));
    Сообщить("image.native.constructor.arguments.5.second=" + Строка(МассивПробы[0].Количество()));
    Сообщить("image.native.constructor.arguments.5.last=" + Строка(МассивПробы[0][0][0][0].Количество()));
    Сообщить("image.native.constructor.arguments.5.type=" + Строка(?(ТипЗнч(МассивПробы) = Тип("Массив"), 1, 0)));
    МассивПробы = ЗамерыТранзитивногоРодителя.Проба6(2, 3, 1, 1, 1, 1);
    Сообщить("image.native.constructor.arguments.6.count=" + Строка(МассивПробы.Количество()));
    Сообщить("image.native.constructor.arguments.6.second=" + Строка(МассивПробы[0].Количество()));
    Сообщить("image.native.constructor.arguments.6.last=" + Строка(МассивПробы[0][0][0][0][0].Количество()));
    Сообщить("image.native.constructor.arguments.6.type=" + Строка(?(ТипЗнч(МассивПробы) = Тип("Массив"), 1, 0)));
    МассивПробы[0][0][0][0][0][0] = 7;
    ДругойМассивПробы = ЗамерыТранзитивногоРодителя.Проба6(2, 3, 1, 1, 1, 1);
    Сообщить("image.native.constructor.arguments.mutation=" + Строка(МассивПробы[0][0][0][0][0][0]));
    Сообщить("image.native.constructor.arguments.independent=" + Строка(?(ДругойМассивПробы[0][0][0][0][0][0] = Неопределено, 1, 0)));
Исключение
    Сообщить("image.native.constructor.arguments.failure=" + ОписаниеОшибки());
КонецПопытки;
