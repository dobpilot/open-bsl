// PLATFORM-ONLY: полное имя текущего пользователя из сеанса 1С.

Процедура М(Ид, Значение)
    Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "user-full-name-1");
М("USER.FULL_NAME.VALUE", ПолноеИмяПользователя());
М("USER.FULL_NAME.TYPE", ТипЗнч(ПолноеИмяПользователя()));
М("USER.FULL_NAME.ALIAS", ПолноеИмяПользователя() = UserFullName());
М("file.end", "user-full-name-1");
