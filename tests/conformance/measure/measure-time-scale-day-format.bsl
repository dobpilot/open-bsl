// Переносимый контроль формата дня шкалы времени.
Сообщить("time_scale_day_format.root" + Символ(9) + Строка(ФорматДняШкалыВремени));
Сообщить("time_scale_day_format.type" + Символ(9) + Строка(ТипЗнч(ФорматДняШкалыВремени.ДеньМесяца)));
Сообщить("time_scale_day_format.MonthDay" + Символ(9) + Строка(ФорматДняШкалыВремени.ДеньМесяца));
Сообщить("time_scale_day_format.WeekDay" + Символ(9) + Строка(ФорматДняШкалыВремени.ДеньНедели));
Сообщить("time_scale_day_format.MonthDayWeekDay" + Символ(9) + Строка(ФорматДняШкалыВремени.ДеньМесяцаДеньНедели));
Сообщить("time_scale_day_format.WeekDayMonthDay" + Символ(9) + Строка(ФорматДняШкалыВремени.ДеньНеделиДеньМесяца));
Если ФорматДняШкалыВремени.ДеньМесяца = TimeScaleDayFormat.MonthDay И ФорматДняШкалыВремени.ДеньНедели = TimeScaleDayFormat.WeekDay И ФорматДняШкалыВремени.ДеньМесяцаДеньНедели = TimeScaleDayFormat.MonthDayWeekDay И ФорматДняШкалыВремени.ДеньНеделиДеньМесяца = TimeScaleDayFormat.WeekDayMonthDay Тогда
 Сообщить("time_scale_day_format.aliases" + Символ(9) + "PASS");
Иначе
 Сообщить("time_scale_day_format.aliases" + Символ(9) + "FAIL");
КонецЕсли;
Если форматдняшкалывремени.деньмесяца = timescaledayformat.monthday Тогда
 Сообщить("time_scale_day_format.folded" + Символ(9) + "PASS");
Иначе
 Сообщить("time_scale_day_format.folded" + Символ(9) + "FAIL");
КонецЕсли;
