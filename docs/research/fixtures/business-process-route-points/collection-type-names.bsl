КоллекцияПробы = БизнесПроцессы.ЗамерБизнесПроцесса.ТочкиМаршрута;
Попытка
    Сообщить("route.points.collection.type.russian" + Символ(9) + Строка(ТипЗнч(КоллекцияПробы) = Тип("ТочкиМаршрута")));
Исключение
    Сообщить("route.points.collection.type.russian" + Символ(9) + "error");
КонецПопытки;
Попытка
    Сообщить("route.points.collection.type.long" + Символ(9) + Строка(ТипЗнч(КоллекцияПробы) = Тип("BusinessProcessRoutePoints")));
Исключение
    Сообщить("route.points.collection.type.long" + Символ(9) + "error");
КонецПопытки;
Попытка
    Сообщить("route.points.collection.type.short" + Символ(9) + Строка(ТипЗнч(КоллекцияПробы) = Тип("RoutePoints")));
Исключение
    Сообщить("route.points.collection.type.short" + Символ(9) + "error");
КонецПопытки;
