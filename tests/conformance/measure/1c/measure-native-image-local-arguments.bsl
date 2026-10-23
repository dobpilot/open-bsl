// Нативная проба локальных вызовов с несколькими и опущенными аргументами.
Попытка
    Аргумент0 = 1;
    Аргумент1 = 2;
    Аргумент2 = 3;
    Аргумент3 = 4;
    Аргумент4 = 5;
    Аргумент5 = 6;
    РезультатПробы = ЗамерыТранзитивногоРодителя.Проба0(Аргумент0, Аргумент1, Аргумент2, Аргумент3, Аргумент4, Аргумент5);
    Сообщить("image.native.local.arguments.0.result=" + Формат(РезультатПробы, "ЧГ=0"));
    Сообщить("image.native.local.arguments.0.argument0=" + Строка(Аргумент0));
    Сообщить("image.native.local.arguments.0.argument1=" + Строка(Аргумент1));
    Сообщить("image.native.local.arguments.0.argument2=" + Строка(Аргумент2));
    Сообщить("image.native.local.arguments.0.argument3=" + Строка(Аргумент3));
    Сообщить("image.native.local.arguments.0.argument4=" + Строка(Аргумент4));
    Сообщить("image.native.local.arguments.0.argument5=" + Строка(Аргумент5));
    Аргумент0 = 1;
    Аргумент1 = 2;
    Аргумент2 = 3;
    Аргумент3 = 4;
    Аргумент4 = 5;
    Аргумент5 = 6;
    РезультатПробы = ЗамерыТранзитивногоРодителя.Проба2(Аргумент0, Аргумент1, Аргумент2, Аргумент3, Аргумент4, Аргумент5);
    Сообщить("image.native.local.arguments.2.result=" + Формат(РезультатПробы, "ЧГ=0"));
    Сообщить("image.native.local.arguments.2.argument0=" + Строка(Аргумент0));
    Сообщить("image.native.local.arguments.2.argument1=" + Строка(Аргумент1));
    Сообщить("image.native.local.arguments.2.argument2=" + Строка(Аргумент2));
    Сообщить("image.native.local.arguments.2.argument3=" + Строка(Аргумент3));
    Сообщить("image.native.local.arguments.2.argument4=" + Строка(Аргумент4));
    Сообщить("image.native.local.arguments.2.argument5=" + Строка(Аргумент5));
    Аргумент0 = 1;
    Аргумент1 = 2;
    Аргумент2 = 3;
    Аргумент3 = 4;
    Аргумент4 = 5;
    Аргумент5 = 6;
    РезультатПробы = ЗамерыТранзитивногоРодителя.Проба6(Аргумент0, Аргумент1, Аргумент2, Аргумент3, Аргумент4, Аргумент5);
    Сообщить("image.native.local.arguments.6.result=" + Формат(РезультатПробы, "ЧГ=0"));
    Сообщить("image.native.local.arguments.6.argument0=" + Строка(Аргумент0));
    Сообщить("image.native.local.arguments.6.argument1=" + Строка(Аргумент1));
    Сообщить("image.native.local.arguments.6.argument2=" + Строка(Аргумент2));
    Сообщить("image.native.local.arguments.6.argument3=" + Строка(Аргумент3));
    Сообщить("image.native.local.arguments.6.argument4=" + Строка(Аргумент4));
    Сообщить("image.native.local.arguments.6.argument5=" + Строка(Аргумент5));
Исключение
    Сообщить("image.native.local.arguments.failure=" + ОписаниеОшибки());
КонецПопытки;
Попытка
    РезультатПробы = ЗамерыТранзитивногоРодителя.ПробаНоль();
    Сообщить("image.native.local.arguments.zero.failure=0");
Исключение
    Сообщить("image.native.local.arguments.zero.failure=1");
КонецПопытки;
