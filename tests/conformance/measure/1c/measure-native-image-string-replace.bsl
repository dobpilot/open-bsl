// Нативная проба инструкции строковой замены и порядка аргументов.
Попытка
    ТекстПробы = "abcabc";
    РезультатПробы = ЗамерыТранзитивногоРодителя.Проба(ТекстПробы, "ab", "X");
    Сообщить("image.native.string_replace.repeat.result=" + РезультатПробы);
    Сообщить("image.native.string_replace.repeat.source=" + ТекстПробы);
Исключение
    Сообщить("image.native.string_replace.repeat.failure=1");
КонецПопытки;
Попытка
    ТекстПробы = "aaa";
    РезультатПробы = ЗамерыТранзитивногоРодителя.Проба(ТекстПробы, "aa", "X");
    Сообщить("image.native.string_replace.overlap.result=" + РезультатПробы);
    Сообщить("image.native.string_replace.overlap.source=" + ТекстПробы);
Исключение
    Сообщить("image.native.string_replace.overlap.failure=1");
КонецПопытки;
Попытка
    ТекстПробы = "абвАбв";
    РезультатПробы = ЗамерыТранзитивногоРодителя.Проба(ТекстПробы, "аб", "я");
    Сообщить("image.native.string_replace.unicode.result=" + РезультатПробы);
    Сообщить("image.native.string_replace.unicode.source=" + ТекстПробы);
Исключение
    Сообщить("image.native.string_replace.unicode.failure=1");
КонецПопытки;
Попытка
    ТекстПробы = "abc";
    РезультатПробы = ЗамерыТранзитивногоРодителя.Проба(ТекстПробы, "z", "X");
    Сообщить("image.native.string_replace.absent.result=" + РезультатПробы);
    Сообщить("image.native.string_replace.absent.source=" + ТекстПробы);
Исключение
    Сообщить("image.native.string_replace.absent.failure=1");
КонецПопытки;
Попытка
    ТекстПробы = "abcabc";
    РезультатПробы = ЗамерыТранзитивногоРодителя.Проба(ТекстПробы, "ab", "");
    Сообщить("image.native.string_replace.delete.result=" + РезультатПробы);
    Сообщить("image.native.string_replace.delete.source=" + ТекстПробы);
Исключение
    Сообщить("image.native.string_replace.delete.failure=1");
КонецПопытки;
Попытка
    ТекстПробы = "abc";
    РезультатПробы = ЗамерыТранзитивногоРодителя.Проба(ТекстПробы, "", "X");
    Сообщить("image.native.string_replace.empty.result=" + РезультатПробы);
    Сообщить("image.native.string_replace.empty.source=" + ТекстПробы);
Исключение
    Сообщить("image.native.string_replace.empty.failure=1");
КонецПопытки;
