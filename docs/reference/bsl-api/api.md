<!-- Сгенерировано bsl-cli --emit-api-reference. Не редактировать вручную. -->

# BSL API open-bsl

Справочник построен из runtime-дескрипторов стандартной сборки `open-bsl`. Имена регистронезависимы; первым указано каноническое написание. Пометка «объявлено без реализации» означает, что имя компилируется, а вызов поднимает ошибку. Описание — первое предложение doc-комментария реализации; «Реализация» — файл в `crates/`, где она находится.

## Встроенные глобальные функции

| Имя и псевдонимы | Вид | Описание | Реализация |
|---|---|---|---|
| `Sqrt` | встроенная функция | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Pow` | встроенная функция | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Log` | встроенная функция | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Log10` | встроенная функция | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Exp` | встроенная функция | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Sin` | встроенная функция | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Cos` | встроенная функция | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Tan` | встроенная функция | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ASin` | встроенная функция | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ACos` | встроенная функция | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ATan` | встроенная функция | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Мин`, `Min` | встроенная функция | Наименьшее из одного или нескольких сравнимых значений. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Макс`, `Max` | встроенная функция | Наибольшее из одного или нескольких сравнимых значений. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Окр`, `Round` | встроенная функция | `Окр(x, ЧислоРазрядов, Режим)` — арность у самой функции в 1С переменная (второй и третий аргументы необязательны, оба по умолчанию `0`), но здесь всегда 3: `bsl-sema::resolver::resolve_call` подставляет недостающие `0` литералами при резолвинге, а не вводит вариативную арность ради одной функции. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Цел`, `Int` | встроенная функция | `Цел(x)` — усечение к нулю, не округление (см. `BslValue::trunc`). | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Сообщить`, `Message` | процедура | Побочный эффект — печать в stdout. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Строка`, `String` | встроенная функция | `Строка(x)` = `Формат(x, Неопределено)` — форматная строка по умолчанию. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Формат`, `Format` | встроенная функция | `Формат(x, spec)` — с явной форматной строкой. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `СтрокаСЧислом`, `StringWithNumber` — *объявлено без реализации* | функция | Локализованное склонение числа; пока без таблицы категорий языков. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ПолучитьСклоненияСтроки`, `GetStringDeclensions` — *объявлено без реализации* | функция | Склонение строки; пока без морфологического движка. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ПолучитьСклоненияСтрокиПоЧислу`, `GetStringDeclensionsByNumber` — *объявлено без реализации* | функция | Склонение строки по числу; пока без русской морфологии. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ДанныеФормыВЗначение`, `FormDataToValue` — *объявлено без реализации* | функция | Преобразование данных управляемой формы; модель данных формы пока отсутствует. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ЗначениеВДанныеФормы`, `ValueToFormData` — *объявлено без реализации* | процедура | Обратное преобразование в данные формы; модель данных формы пока отсутствует. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `КопироватьДанныеФормы`, `CopyFormData` — *объявлено без реализации* | функция | Копирование данных формы; модель данных формы пока отсутствует. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ПредставлениеСобытияЖурналаРегистрации`, `EventLogEventPresentation` — *объявлено без реализации* | функция | Представление события журнала; пока без локализованного каталога. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ПредставлениеПериода`, `PeriodPresentation` — *объявлено без реализации* | функция | Внешнее представление периода; пока без локализованного форматирования. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ЧислоПрописью`, `NumberInWords` — *объявлено без реализации* | функция | Пропись числа; пока без форм слов для разных локализаций. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Число`, `Number` | встроенная функция | `Число(строка)` — обратный разбор форматированной строки в число. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Булево`, `Boolean` | встроенная функция | `Булево(Значение)` / `Boolean(Value)` — измеренное преобразование числа, булева и строк `Истина`/`True`/`Yes`, `Ложь`/`False`/`No`. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `СтрДлина`, `StrLen` | встроенная функция | `СтрДлина`/`StrLen` — длина в код-юнитах UTF-16, не в символах. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Лев`, `Left` | встроенная функция | `Лев`/`Left(строка, длина)`. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Прав`, `Right` | встроенная функция | `Прав`/`Right(строка, длина)`. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Сред`, `Mid` | встроенная функция | `Сред`/`Mid(строка, начало, длина)` — длину можно опустить, тогда берётся всё до конца строки (см. `arity_range`, который и делает третий аргумент необязательным). | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ВРег`, `Upper` | встроенная функция | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `НРег`, `Lower` | встроенная функция | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ТРег`, `Title` | функция | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `СокрЛП`, `TrimAll` | встроенная функция | `СокрЛП`/`TrimAll` — обрезка пробелов с обеих сторон. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `СокрЛ`, `TrimL` | встроенная функция | `СокрЛ`/`TrimL` — только слева. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `СокрП`, `TrimR` | встроенная функция | `СокрП`/`TrimR` — только справа. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `СтрНайти`, `StrFind` | функция | `СтрНайти`/`StrFind(строка, подстрока)` -> позиция 1-based в код-юнитах UTF-16, `0` если не найдено. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Найти`, `Find` | функция | `Найти`/`Find` — старый двухаргументный поиск с позицией `1` для пустой подстроки. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `СтрСравнить`, `StrCompare` | функция | `СтрСравнить`/`StrCompare(левое, правое)` — регистронезависимое сравнение строковых представлений со знаком `-1`, `0` или `1`. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `СтрЧислоВхождений`, `StrOccurrenceCount` | функция | `СтрЧислоВхождений`/`StrOccurrenceCount(строка, подстрока)` — число неперекрывающихся регистрозависимых вхождений. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `СтрНачинаетсяС`, `StrStartsWith` | функция | Регистрозависимая проверка непустого префикса. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `СтрЗаканчиваетсяНа`, `StrEndsWith` | функция | Регистрозависимая проверка непустого суффикса. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `СтрЗаменить`, `StrReplace` | встроенная функция | `СтрЗаменить`/`StrReplace(строка, что, чем)`. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `СтрРазделить`, `StrSplit` | функция | `СтрРазделить`/`StrSplit(строка, разделитель)` -> `Массив`. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `СтрСоединить`, `StrConcat` | функция | `СтрСоединить`/`StrConcat(массив, разделитель)` -> `Строка`. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `СтрЧислоСтрок`, `StrLineCount` | встроенная функция | `СтрЧислоСтрок`/`StrLineCount`. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `СтрПолучитьСтроку`, `StrGetLine` | встроенная функция | `СтрПолучитьСтроку`/`StrGetLine(строка, номер)`. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `СтрШаблон`, `StrTemplate` | функция | `СтрШаблон`/`StrTemplate(шаблон, З1..З10)` — единственная по-настоящему вариативная встроенная функция (см. `arity_range`). | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `НСтр`, `NStr` | функция | `НСтр`/`NStr(РесурснаяСтрока, КодЯзыка)` выбирает одно значение из записи `ru = '...'; en = '...'`. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ПустаяСтрока`, `IsBlankString` | функция | `ПустаяСтрока`/`IsBlankString(Значение)` проверяет отсутствие непробельных символов; `Неопределено` также считается пустым. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Символ`, `Char` | встроенная функция | `Символ`/`Char(код)`. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `КодСимвола`, `CharCode` | встроенная функция | `КодСимвола`/`CharCode(строка, позиция)`. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ЗначениеЗаполнено`, `ValueIsFilled` | функция | `ЗначениеЗаполнено`/`ValueIsFilled` — см. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ТипЗнч`, `TypeOf` | встроенная функция | `ТипЗнч`/`TypeOf(значение)` -> `Тип`. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Тип`, `Type` | встроенная функция | `Тип`/`Type("ИмяТипа")` -> `Тип`. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ИнформацияОбОшибке`, `ErrorInfo` | функция | Снимок текущего исключения BSL-задачи. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ОписаниеОшибки`, `ErrorDescription` | функция | Безопасное описание ошибки активного обработчика текущего вызова. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `КраткоеПредставлениеОшибки`, `BriefErrorDescription` | функция | Краткое описание из неизменяемого снимка `ИнформацияОбОшибке`. | [bsl-rt/src/error_info.rs](../../../crates/bsl-rt/src/error_info.rs) |
| `ПодробноеПредставлениеОшибки`, `DetailedErrorDescription` | функция | Безопасный текст из неизменяемого снимка `ИнформацияОбОшибке`. | [bsl-rt/src/error_info.rs](../../../crates/bsl-rt/src/error_info.rs) |
| `Дата`, `Date` | встроенная функция | `Дата(Год, Месяц, День, Час, Минута, Секунда)` либо `Дата("ГГГГММДДЧЧММСС")` — одна встроенная функция с перегрузкой по типу первого аргумента, как и в самой 1С (см. `BslValue::make_date`). | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ТекущаяДата`, `CurrentDate` | встроенная функция | `ТекущаяДата`/`CurrentDate` — см. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ТекущаяДатаСеанса`, `CurrentSessionDate` | встроенная функция | `ТекущаяДатаСеанса`/`CurrentSessionDate` — текущий момент в эффективном часовом поясе этого прогона. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `СмещениеСтандартногоВремени`, `StandardTimeOffset` | функция | `СмещениеСтандартногоВремени`/`StandardTimeOffset` — стандартное смещение текущей или именованной зоны в секундах. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `СмещениеЛетнегоВремени`, `DaylightTimeOffset` | функция | `СмещениеЛетнегоВремени`/`DaylightTimeOffset` — летнее смещение в секундах. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `УниверсальноеВремя`, `ToUniversalTime` | функция | `УниверсальноеВремя`/`ToUniversalTime` — перевод местной даты текущей или явно заданной зоны в UTC. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `МестноеВремя`, `ToLocalTime` | функция | `МестноеВремя`/`ToLocalTime` — перевод UTC в текущую или явно заданную зону. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ЧасовойПоясСеанса`, `SessionTimeZone` | функция | `ЧасовойПоясСеанса`/`SessionTimeZone` — идентификатор зоны текущего прогона. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ЧасовойПояс`, `TimeZone` | функция | `ЧасовойПояс`/`TimeZone` — идентификатор зоны компьютера. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ПолучитьДопустимыеЧасовыеПояса`, `GetAvailableTimeZones` — *объявлено без реализации* | функция | `ПолучитьДопустимыеЧасовыеПояса` — список зон платформы текущего хоста. | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `ТекущаяУниверсальнаяДата`, `CurrentUniversalDate` | функция | `ТекущаяУниверсальнаяДата`/`CurrentUniversalDate` — текущий момент UTC как значение `Дата`. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ТекущаяУниверсальнаяДатаВМиллисекундах`, `CurrentUniversalDateInMilliseconds` | функция | Число миллисекунд с начала Unix-эпохи в UTC. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Год`, `Year` | встроенная функция | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Месяц`, `Month` | встроенная функция | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `День`, `Day` | встроенная функция | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Час`, `Hour` | встроенная функция | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Минута`, `Minute` | встроенная функция | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Секунда`, `Second` | встроенная функция | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ДеньНедели`, `WeekDay` | встроенная функция | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ДеньГода`, `DayOfYear` | встроенная функция | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `НеделяГода`, `WeekOfYear` | встроенная функция | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `НачалоМинуты`, `BegOfMinute` | встроенная функция | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `КонецМинуты`, `EndOfMinute` | встроенная функция | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `НачалоЧаса`, `BegOfHour` | встроенная функция | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `КонецЧаса`, `EndOfHour` | встроенная функция | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `НачалоДня`, `BegOfDay` | встроенная функция | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `КонецДня`, `EndOfDay` | встроенная функция | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `НачалоМесяца`, `BegOfMonth` | встроенная функция | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `КонецМесяца`, `EndOfMonth` | встроенная функция | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `НачалоКвартала`, `BegOfQuarter` | встроенная функция | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `КонецКвартала`, `EndOfQuarter` | встроенная функция | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `НачалоГода`, `BegOfYear` | встроенная функция | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `КонецГода`, `EndOfYear` | встроенная функция | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `НачалоНедели`, `BegOfWeek` | встроенная функция | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `КонецНедели`, `EndOfWeek` | встроенная функция | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ДобавитьМесяц`, `AddMonth` | встроенная функция | `ДобавитьМесяц(Дата, Количество)`. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ЗаполнитьЗначенияСвойств`, `FillPropertyValues` | процедура | `ЗаполнитьЗначенияСвойств(Приемник, Источник, СписокСвойств[, ИсключаяСвойства])` — единственная встроенная ФУНКЦИЯ, которой нужен рантайм-контекст имён (набор полей приёмника резолвится по `NameId`), поэтому её тело живёт в модуле `fill` и вызывается через `call_builtin_fn_ctx`, а не через `call_builtin_fn`. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ЗначениеВСтрокуВнутр`, `ValueToStringInternal` | функция | `ЗначениеВСтрокуВнутр(Значение)` — внутренний строковый формат платформы (см. модуль `vstr`). | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ЗначениеИзСтрокиВнутр`, `ValueFromStringInternal` | функция | `ЗначениеИзСтрокиВнутр(Строка)` — обратный разбор; контекст нужен на запись: поля структур интернируются, формы растут. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ЗначениеВФайл`, `ValueToFile` | функция | `ЗначениеВФайл(ИмяФайла, Значение)` — строка `ЗначениеВСтрокуВнутр` в UTF-8 С BOM и переводами строк CRLF (измерено побайтовым сравнением с файлами платформы, см. `vstr::value_to_file`). | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ЗначениеИзФайла`, `ValueFromFile` | функция | `ЗначениеИзФайла(ИмяФайла)` — обратное чтение того же файла. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `АргументыКоманднойСтроки`, `CommandLineArguments` | функция | `АргументыКоманднойСтроки` — массив строк, переданных скрипту после его имени в командной строке. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Метаданные`, `Metadata` | функция | Глобальное свойство конфигурации. | [bsl-rt/src/metadata.rs](../../../crates/bsl-rt/src/metadata.rs) |
| `ЭтотОбъект`, `ThisObject` | функция | Ссылка на текущий экземпляр BSL-модуля open-bsl, предоставляемая VM. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ПолучитьИмяВременногоФайла`, `GetTempFileName` | функция | `ПолучитьИмяВременногоФайла(Расширение)` — отсутствующий путь из пространства host-файловой системы. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `КаталогВременныхФайлов`, `TempFilesDir` | функция | Временный каталог host с завершающим разделителем. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `КаталогПрограммы`, `BinDir` — *объявлено без реализации* | функция | Каталог исполняемых файлов платформы 1С; host пока не задаёт его. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ПолучитьРазделительПути`, `GetPathSeparator` | функция | Разделитель пути host-файловой системы. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ПолучитьМаскуВсеФайлы`, `GetAllFilesMask` | функция | Маска выбора всех файлов в host-файловой системе. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `УдалитьФайлы`, `DeleteFiles` | процедура | `УдалитьФайлы(Путь)` — файл либо дерево; отсутствующий путь не ошибка. | [bsl-rt/src/file_search.rs](../../../crates/bsl-rt/src/file_search.rs) |
| `СоздатьКаталог`, `CreateDirectory` | процедура | `СоздатьКаталог(Путь)` — создание каталога с родителями через host. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `НайтиФайлы`, `FindFiles` | функция | `НайтиФайлы(Путь, Маска, Рекурсивно)` — массив объектов Файл. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `НайтиФайлыАсинх`, `FindFilesAsync` | функция | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ЗапуститьПриложение`, `RunApp` | процедура | Процедурный запуск; внутренний результат используется для выходного аргумента. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ЗапуститьПриложениеАсинх`, `RunAppAsync` | функция | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `СоздатьКаталогАсинх`, `CreateDirectoryAsync` | функция | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `УдалитьФайлыАсинх`, `DeleteFilesAsync` | функция | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `КаталогВременныхФайловАсинх`, `TempFilesDirAsync` | функция | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `РазделитьДвоичныеДанные`, `SplitBinaryData` | функция | `РазделитьДвоичныеДанные(Данные, РазмерЧасти)` -> `Массив` частей (см. `BslValue::binary_data_split`). | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `СоединитьДвоичныеДанные`, `ConcatBinaryData` | функция | `СоединитьДвоичныеДанные(Массив)` -> склеенные данные (см. `BslValue::binary_data_combine`). | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `СоединитьБуферыДвоичныхДанных`, `ConcatBinaryDataBuffers` | функция | Соединяет массив буферов с необязательными разделителем и порядком байтов. | [bsl-rt/src/bindata.rs](../../../crates/bsl-rt/src/bindata.rs) |
| `Base64Значение`, `Base64Value` | функция | `Base64Значение(Строка)` — двоичные данные из Base64. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Base64Строка`, `Base64String` | функция | `Base64Строка(ДвоичныеДанные)` — исходная функция платформы. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ПолучитьBase64СтрокуИзДвоичныхДанных`, `GetBase64StringFromBinaryData` | функция | Стандартный Base64 без переносов строк. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ПолучитьBase64СтрокуИзБуфераДвоичныхДанных`, `GetBase64StringFromBinaryDataBuffer` | функция | Кодирует байты буфера в Base64 с переносами по 64 символа. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ПолучитьДвоичныеДанныеИзBase64Строки`, `GetBinaryDataFromBase64String` | функция | Обратное преобразование Base64 в `ДвоичныеДанные`. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ПолучитьДвоичныеДанныеИзBase64ДвоичныхДанных`, `GetBinaryDataFromBase64BinaryData` | функция | Декодирует Base64 из байтов `ДвоичныеДанные`. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ПолучитьHexСтрокуИзДвоичныхДанных`, `GetHexStringFromBinaryData` | функция | Заглавное шестнадцатеричное представление байтов. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ПолучитьДвоичныеДанныеИзHexСтроки`, `GetBinaryDataFromHexString` | функция | Двоичные данные из Hex-цифр строки с пропуском остальных символов. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ПолучитьБуферДвоичныхДанныхИзHexСтроки`, `GetBinaryDataBufferFromHexString` | функция | Из тех же Hex-цифр создаёт изменяемый буфер. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ПолучитьHexСтрокуИзБуфераДвоичныхДанных`, `GetHexStringFromBinaryDataBuffer` | функция | Заглавное шестнадцатеричное представление байтов буфера. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `КодироватьСтроку`, `EncodeString` | функция | Percent-кодирование UTF-8 строки в одном из двух режимов платформы. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `РаскодироватьСтроку`, `DecodeString` | функция | Обратное percent-декодирование; `+` не считается пробелом. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ПолучитьДвоичныеДанныеИзСтроки`, `GetBinaryDataFromString` | функция | `ПолучитьДвоичныеДанныеИзСтроки(Строка, Кодировка, ДобавлятьBOM)` (см. `bindata::binary_data_from_string`). | [bsl-rt/src/bindata.rs](../../../crates/bsl-rt/src/bindata.rs) |
| `ПолучитьБуферДвоичныхДанныхИзСтроки`, `GetBinaryDataBufferFromString` | функция | `ПолучитьБуферДвоичныхДанныхИзСтроки` — то же самое, но в буфер. | [bsl-rt/src/bindata.rs](../../../crates/bsl-rt/src/bindata.rs) |
| `ПолучитьСтрокуИзДвоичныхДанных`, `GetStringFromBinaryData` | функция | `ПолучитьСтрокуИзДвоичныхДанных(Данные, Кодировка)`; ведущая сигнатура снимается (измерено). | [bsl-rt/src/bindata.rs](../../../crates/bsl-rt/src/bindata.rs) |
| `ПолучитьСтрокуИзБуфераДвоичныхДанных`, `GetStringFromBinaryDataBuffer` | функция | `ПолучитьСтрокуИзБуфераДвоичныхДанных(Буфер, Кодировка)`. | [bsl-rt/src/bindata.rs](../../../crates/bsl-rt/src/bindata.rs) |
| `ПолучитьДвоичныеДанныеИзБуфераДвоичныхДанных`, `GetBinaryDataFromBinaryDataBuffer` | функция | Снимок байтов буфера как неизменяемые `ДвоичныеДанные`. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ПолучитьБуферДвоичныхДанныхИзДвоичныхДанных`, `GetBinaryDataBufferFromBinaryData` | функция | Новый изменяемый буфер из байтов `ДвоичныеДанные`. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |

## Встроенные методы

Эти методы обслуживает базовый runtime. Применимость зависит от типа получателя, поэтому ссылка ведёт на общую таблицу методов.

| Имя и псевдонимы | Описание | Реализация |
|---|---|---|
| `Количество`, `Count` | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ВГраница`, `UBound` | `Массив.ВГраница()` — последний индекс или `-1` для пустого массива. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Добавить`, `Add` | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Удалить`, `Delete` | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Очистить`, `Clear` | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Вставить`, `Insert` | `Массив.Вставить(Индекс, Значение)` и вставка пары в структуру/соответствие. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Получить`, `Get` | `Соответствие.Получить(Ключ)` и `БуферДвоичныхДанных.Получить(Позиция)`. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Свойство`, `Property` | `Структура.Свойство(Ключ, ЗначениеПоУмолчанию)`. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Найти`, `Find` | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `НайтиСтроки`, `FindRows` | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Сортировать`, `Sort` | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ЗаполнитьЗначения`, `FillValues` | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Итог`, `Total` | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Скопировать`, `Copy` | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `СкопироватьКолонки`, `CopyColumns` | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ВыгрузитьКолонку`, `UnloadColumn` | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ЗагрузитьКолонку`, `LoadColumn` | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Сдвинуть`, `Move` | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Индекс`, `IndexOf` | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Свернуть`, `GroupBy` | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Записать`, `Write` | `ЗаписьТекста.Записать(Текст)` и блочная запись в буфер. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Закрыть`, `Close` | `ЗаписьТекста.Закрыть()`. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Размер`, `Size` | `ДвоичныеДанные.Размер()`. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ОткрытьПотокДляЧтения`, `OpenStreamForRead` | `ДвоичныеДанные.ОткрытьПотокДляЧтения()` — объект строит зарегистрированный компонент потоков. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ПривестиЗначение` | `ОписаниеТипов.ПривестиЗначение(Значение)`. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Типы`, `Types` | `ОписаниеТипов.Типы()` возвращает независимый массив типов. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `СодержитТип`, `ContainsType` | `ОписаниеТипов.СодержитТип(Тип)` проверяет состав описания. | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Установить`, `Set` | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ПрочитатьЦелое16`, `ReadInt16` | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ПрочитатьЦелое32`, `ReadInt32` | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ПрочитатьЦелое64`, `ReadInt64` | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ЗаписатьЦелое16`, `WriteInt16` | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ЗаписатьЦелое32`, `WriteInt32` | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ЗаписатьЦелое64`, `WriteInt64` | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Разделить`, `Split` | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Соединить`, `Concat` | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ПолучитьСрез`, `GetSlice` | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ЗаписатьПобитовоеИ`, `WriteBitwiseAnd` | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ЗаписатьПобитовоеИли`, `WriteBitwiseOr` | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ЗаписатьПобитовоеИсключительноеИли`, `WriteBitwiseXor` | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `ЗаписатьПобитовоеИНе`, `WriteBitwiseAndNot` | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |
| `Инвертировать`, `Invert` | — | [bsl-rt/src/builtin.rs](../../../crates/bsl-rt/src/builtin.rs) |

Свойства нативного `ОписаниеТипов` описаны в [справочнике квалификаторов](type-description-qualifiers.md).

## Компоненты

### `bsl-rt` 0.5.0

#### Глобальные свойства

| Имя и псевдонимы | Функция получения |
|---|---|
| `ЦветаСтиля`, `StyleColors` | `ЦветаСтиля`, `StyleColors` |
| `ШрифтыСтиля`, `StyleFonts` | `ШрифтыСтиля`, `StyleFonts` |
| `WebЦвета`, `WebColors` | `WebЦвета`, `WebColors` |

#### Глобальные функции

| Имя и псевдонимы | Вид | Описание | Реализация |
|---|---|---|---|
| `РазделитьФайл`, `SplitFile` — *объявлено без реализации* | функция | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `СтрНайтиИВыделитьОформлением`, `StrFindAndHighlightByAppearance` — *объявлено без реализации* | функция | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `КопироватьФайл`, `CopyFile` — *объявлено без реализации* | процедура | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `ПереместитьФайл`, `MoveFile` — *объявлено без реализации* | процедура | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `ОбъединитьФайлы`, `MergeFiles` — *объявлено без реализации* | процедура | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `WebЦвета`, `WebColors` — *объявлено без реализации* | функция | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `ШрифтыСтиля`, `StyleFonts` — *объявлено без реализации* | функция | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `ЦветаСтиля`, `StyleColors` — *объявлено без реализации* | функция | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `ПоместитьВоВременноеХранилище`, `PutToTempStorage` | функция | `ПоместитьВоВременноеХранилище(Значение, Адрес)`. | [bsl-rt/src/temp_storage.rs](../../../crates/bsl-rt/src/temp_storage.rs) |
| `ПолучитьИзВременногоХранилища`, `GetFromTempStorage` | функция | `ПолучитьИзВременногоХранилища(Адрес)`. | [bsl-rt/src/temp_storage.rs](../../../crates/bsl-rt/src/temp_storage.rs) |
| `УдалитьИзВременногоХранилища`, `DeleteFromTempStorage` | процедура | `УдалитьИзВременногоХранилища(Адрес)`. | [bsl-rt/src/temp_storage.rs](../../../crates/bsl-rt/src/temp_storage.rs) |
| `ПолучитьСообщенияПользователю`, `GetUserMessages` | функция | Возвращает ожидающие сообщения внедрённого хоста. | [bsl-rt/src/user_message.rs](../../../crates/bsl-rt/src/user_message.rs) |
| `ЭтоАдресВременногоХранилища`, `IsTempStorageURL` | функция | `ЭтоАдресВременногоХранилища(Адрес)` проверяет форму адреса, а не его наличие. | [bsl-rt/src/temp_storage.rs](../../../crates/bsl-rt/src/temp_storage.rs) |

#### Конструкторы

| Имя и псевдонимы | Описание | Реализация |
|---|---|---|
| `МакетКомпоновкиДанных`, `DataCompositionTemplate` — *объявлено без реализации* | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `МакетЗаголовкаКоллекцииЗначенийОбластиКомпоновкиДанных`, `DataCompositionAreaTemplateValueCollectionHeader` — *объявлено без реализации* | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `МакетКоллекцииЗначенийОбластиКомпоновкиДанных`, `DataCompositionAreaTemplateValueCollection` — *объявлено без реализации* | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `Диапазон`, `Range` — *объявлено без реализации* | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `ТекстыСообщенияОбОшибке`, `ErrorMessageTexts` — *объявлено без реализации* | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `БлокировкаСеансов`, `SessionsLock` — *объявлено без реализации* | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `ЭлементИзбранногоРаботыПользователя`, `UserWorkFavoritesItem` — *объявлено без реализации* | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `ИзбранноеРаботыПользователя`, `UserWorkFavorites` — *объявлено без реализации* | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `ОтборКомпоновкиДанных`, `DataCompositionFilter` — *объявлено без реализации* | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `ИзвлечениеТекста`, `TextExtraction` — *объявлено без реализации* | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `ФлагиИнтернетПочтовогоСообщения`, `InternetMailMessageFlags` | Создаёт набор, в котором ни один флаг ещё не установлен. | [bsl-rt/src/internet_mail_message_flags.rs](../../../crates/bsl-rt/src/internet_mail_message_flags.rs) |
| `МоментВремени`, `PointInTime` | Создаёт набор, в котором ни один флаг ещё не установлен. | [bsl-rt/src/internet_mail_message_flags.rs](../../../crates/bsl-rt/src/internet_mail_message_flags.rs) |
| `ОписаниеПередаваемогоФайла`, `TransferableFileDescription` | Создаёт набор, в котором ни один флаг ещё не установлен. | [bsl-rt/src/internet_mail_message_flags.rs](../../../crates/bsl-rt/src/internet_mail_message_flags.rs) |
| `COMSafeArray` — *объявлено без реализации* | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `СвязьПараметраВыбора`, `ChoiceParameterLink` — *объявлено без реализации* | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `FTPСоединение`, `FTPConnection` — *объявлено без реализации* | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `СвязьПоТипу`, `TypeLink` — *объявлено без реализации* | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `ДанныеРасшифровкиКомпоновкиДанных`, `DataCompositionDetailsData` — *объявлено без реализации* | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `ПакетОтображаемыхДокументов`, `RepresentableDocumentBatch` — *объявлено без реализации* | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `ЧтениеHTML`, `HTMLReader` — *объявлено без реализации* | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `МенеджерКриптографии`, `CryptoManager` — *объявлено без реализации* | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `Рамка`, `Border` — *объявлено без реализации* | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `СочетаниеКлавиш`, `Shortcut` — *объявлено без реализации* | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `СертификатКриптографии`, `CryptoCertificate` — *объявлено без реализации* | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `WSПрокси`, `WSProxy` — *объявлено без реализации* | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `WSОпределения`, `WSDefinitions` — *объявлено без реализации* | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `ДоставляемоеУведомление`, `DeliverableNotification` — *объявлено без реализации* | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `ФорматированныйДокумент`, `FormattedDocument` — *объявлено без реализации* | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `ГенераторСлучайныхЧисел`, `RandomNumberGenerator` — *объявлено без реализации* | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `ОбрабатываемаяКартинка`, `ProcessingPicture` — *объявлено без реализации* | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `Линия`, `Line` — *объявлено без реализации* | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `ПараметрВыбора`, `ChoiceParameter` — *объявлено без реализации* | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `ЗаписьТекста`, `TextWriter` | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `Цвет`, `Color` — *объявлено без реализации* | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `РеквизитФормы`, `FormAttribute` — *объявлено без реализации* | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `ИнтернетПочтовыйПрофиль`, `InternetMailProfile` — *объявлено без реализации* | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `ИнтернетПочта`, `InternetMail` — *объявлено без реализации* | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `ДеревоЗначений`, `ValueTree` — *объявлено без реализации* | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `Шрифт`, `Font` — *объявлено без реализации* | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `COMОбъект`, `COMObject` — *объявлено без реализации* | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `ОписаниеОповещения`, `NotifyDescription` | — | [bsl-rt/src/notification.rs](../../../crates/bsl-rt/src/notification.rs) |
| `Файл`, `File` | — | [bsl-rt/src/file.rs](../../../crates/bsl-rt/src/file.rs) |
| `ДвоичныеДанные`, `BinaryData` | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `УникальныйИдентификатор`, `UUID` | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `СписокЗначений`, `ValueList` | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `КвалификаторыДаты` | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `ОписаниеТипов`, `TypeDescription` | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `ФоновыеЗадания`, `BackgroundJobs` | Конструктор менеджера для голого имени `ФоновыеЗадания`. | [bsl-rt/src/background_jobs.rs](../../../crates/bsl-rt/src/background_jobs.rs) |
| `ФиксированныйМассив`, `FixedArray` | `Новый ФиксированныйМассив(<Массив>)` — конструктор ядра. | [bsl-rt/src/fixed_array.rs](../../../crates/bsl-rt/src/fixed_array.rs) |
| `СообщениеПользователю`, `UserMessage` | `Новый СообщениеПользователю` — конструктор по умолчанию. | [bsl-rt/src/user_message.rs](../../../crates/bsl-rt/src/user_message.rs) |
| `ФиксированнаяСтруктура`, `FixedStructure` | — | [bsl-rt/src/fixed_collections.rs](../../../crates/bsl-rt/src/fixed_collections.rs) |
| `ФиксированноеСоответствие`, `FixedMap` | — | [bsl-rt/src/fixed_collections.rs](../../../crates/bsl-rt/src/fixed_collections.rs) |
| `ПолеКомпоновкиДанных`, `DataCompositionField` | — | [bsl-rt/src/data_composition.rs](../../../crates/bsl-rt/src/data_composition.rs) |
| `СтандартнаяДатаНачала`, `StandardBeginningDate` | — | [bsl-rt/src/data_composition.rs](../../../crates/bsl-rt/src/data_composition.rs) |
| `КвалификаторыСтроки`, `StringQualifiers` | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `КвалификаторыЧисла`, `NumberQualifiers` | — | [bsl-rt/src/component.rs](../../../crates/bsl-rt/src/component.rs) |
| `ФорматированнаяСтрока`, `FormattedString` | — | [bsl-rt/src/formatted_string.rs](../../../crates/bsl-rt/src/formatted_string.rs) |
| `Картинка`, `Picture` | — | [bsl-rt/src/picture.rs](../../../crates/bsl-rt/src/picture.rs) |
| `ПараметрКомпоновкиДанных`, `DataCompositionParameter` | — | [bsl-rt/src/data_composition.rs](../../../crates/bsl-rt/src/data_composition.rs) |

#### Типы объектов

| Имя | Представление типа | Дополнительные имена | Описание | Реализация |
|---|---|---|---|---|
| `МоментВремени` | `Момент времени` | `PointInTime` | Описание общего типа момента времени. | [bsl-rt/src/point_in_time.rs](../../../crates/bsl-rt/src/point_in_time.rs) |
| `ФлагиИнтернетПочтовогоСообщения` | `ФлагиИнтернетПочтовогоСообщения` | `InternetMailMessageFlags` | — | [bsl-rt/src/internet_mail_message_flags.rs](../../../crates/bsl-rt/src/internet_mail_message_flags.rs) |
| `ОписаниеПередаваемогоФайла` | `Описание передаваемого файла` | `TransferableFileDescription` | — | [bsl-rt/src/transferable_file_description.rs](../../../crates/bsl-rt/src/transferable_file_description.rs) |
| `МодульBSL` | `Модуль BSL` | `BSLModule` | Тип значения ЭтотОбъект. | [bsl-rt/src/module_reference.rs](../../../crates/bsl-rt/src/module_reference.rs) |
| `ОбщийМодуль` | `Общий Модуль` | `CommonModule` | Тип ссылки на общий модуль конфигурации текущего сеанса. | [bsl-rt/src/module_reference.rs](../../../crates/bsl-rt/src/module_reference.rs) |
| `ОписаниеОповещения` | `Описание оповещения` | `NotifyDescription` | — | [bsl-rt/src/notification.rs](../../../crates/bsl-rt/src/notification.rs) |
| `Обещание` | `Обещание` | — | Тип намеренно не имеет конструктора, методов и свойств BSL. | [bsl-rt/src/promise.rs](../../../crates/bsl-rt/src/promise.rs) |
| `Файл` | `Файл` | `File` | — | [bsl-rt/src/file.rs](../../../crates/bsl-rt/src/file.rs) |
| `ФиксированныйМассив` | `Фиксированный массив` | `FixedArray` | — | [bsl-rt/src/fixed_array.rs](../../../crates/bsl-rt/src/fixed_array.rs) |
| `ФиксированнаяСтруктура` | `Фиксированная структура` | `FixedStructure` | — | [bsl-rt/src/fixed_collections.rs](../../../crates/bsl-rt/src/fixed_collections.rs) |
| `ФиксированноеСоответствие` | `Фиксированное соответствие` | `FixedMap` | — | [bsl-rt/src/fixed_collections.rs](../../../crates/bsl-rt/src/fixed_collections.rs) |
| `МенеджерФоновыхЗаданий` | `Менеджер фоновых заданий` | `BackgroundJobsManager` | — | [bsl-rt/src/background_jobs.rs](../../../crates/bsl-rt/src/background_jobs.rs) |
| `ФоновоеЗадание` | `Фоновое задание` | `BackgroundJob` | — | [bsl-rt/src/background_jobs.rs](../../../crates/bsl-rt/src/background_jobs.rs) |
| `СообщениеПользователю` | `Сообщение пользователю` | `UserMessage` | — | [bsl-rt/src/user_message.rs](../../../crates/bsl-rt/src/user_message.rs) |
| `ИнформацияОбОшибке` | `Информация об ошибке` | `ErrorInfo` | — | [bsl-rt/src/error_info.rs](../../../crates/bsl-rt/src/error_info.rs) |
| `СписокЗначений` | `Список значений` | `ValueList` | — | [bsl-rt/src/value_list.rs](../../../crates/bsl-rt/src/value_list.rs) |
| `ЭлементСпискаЗначений` | `Элемент списка значений` | `ValueListItem` | — | [bsl-rt/src/value_list.rs](../../../crates/bsl-rt/src/value_list.rs) |
| `КоллекцияИндексовТаблицыЗначений` | `Value table index collection` | `ValueTableIndexCollection` | — | [bsl-rt/src/value_table_indexes.rs](../../../crates/bsl-rt/src/value_table_indexes.rs) |
| `ИндексТаблицыЗначений` | `Value table index` | `ValueTableIndex` | — | [bsl-rt/src/value_table_indexes.rs](../../../crates/bsl-rt/src/value_table_indexes.rs) |
| `КвалификаторыДаты` | `Квалификаторы даты` | `DateQualifiers` | — | [bsl-rt/src/type_description.rs](../../../crates/bsl-rt/src/type_description.rs) |
| `КвалификаторыСтроки` | `Квалификаторы строки` | `StringQualifiers` | — | [bsl-rt/src/type_description.rs](../../../crates/bsl-rt/src/type_description.rs) |
| `КвалификаторыЧисла` | `Квалификаторы числа` | `NumberQualifiers` | — | [bsl-rt/src/type_description.rs](../../../crates/bsl-rt/src/type_description.rs) |
| `ПолеКомпоновкиДанных` | `Поле компоновки данных` | `DataCompositionField` | — | [bsl-rt/src/data_composition.rs](../../../crates/bsl-rt/src/data_composition.rs) |
| `ПараметрКомпоновкиДанных` | `Параметр компоновки данных` | `DataCompositionParameter` | — | [bsl-rt/src/data_composition.rs](../../../crates/bsl-rt/src/data_composition.rs) |
| `СтандартнаяДатаНачала` | `Стандартная дата начала` | `StandardBeginningDate` | — | [bsl-rt/src/data_composition.rs](../../../crates/bsl-rt/src/data_composition.rs) |
| `ФорматированнаяСтрока` | `Форматированная строка` | `FormattedString` | — | [bsl-rt/src/formatted_string.rs](../../../crates/bsl-rt/src/formatted_string.rs) |
| `Картинка` | `Картинка` | `Picture` | Дескриптор единого платформенного типа `Картинка`. | [bsl-rt/src/picture.rs](../../../crates/bsl-rt/src/picture.rs) |

#### Члены объектов

##### `МоментВремени`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `Дата`, `Date` | только чтение | Возвращает дату снимка без округления. | [bsl-rt/src/point_in_time.rs](../../../crates/bsl-rt/src/point_in_time.rs) |
| `Ссылка`, `Ref` | только чтение | Возвращает ссылку либо `Неопределено`, если ссылка не задана. | [bsl-rt/src/point_in_time.rs](../../../crates/bsl-rt/src/point_in_time.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `Сравнить`, `Compare` | — | [bsl-rt/src/point_in_time.rs](../../../crates/bsl-rt/src/point_in_time.rs) |

##### `ФлагиИнтернетПочтовогоСообщения`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `Новое`, `New` | только чтение | — | [bsl-rt/src/internet_mail_message_flags.rs](../../../crates/bsl-rt/src/internet_mail_message_flags.rs) |
| `Отвеченное`, `Answered` | чтение и запись | — | [bsl-rt/src/internet_mail_message_flags.rs](../../../crates/bsl-rt/src/internet_mail_message_flags.rs) |
| `Отмененное`, `Canceled` | чтение и запись | — | [bsl-rt/src/internet_mail_message_flags.rs](../../../crates/bsl-rt/src/internet_mail_message_flags.rs) |
| `Пересланное`, `Forwarded` | чтение и запись | — | [bsl-rt/src/internet_mail_message_flags.rs](../../../crates/bsl-rt/src/internet_mail_message_flags.rs) |
| `Помеченное`, `Flagged` | чтение и запись | — | [bsl-rt/src/internet_mail_message_flags.rs](../../../crates/bsl-rt/src/internet_mail_message_flags.rs) |
| `Прочитанное`, `Seen` | чтение и запись | — | [bsl-rt/src/internet_mail_message_flags.rs](../../../crates/bsl-rt/src/internet_mail_message_flags.rs) |
| `Удаленное`, `Deleted` | чтение и запись | — | [bsl-rt/src/internet_mail_message_flags.rs](../../../crates/bsl-rt/src/internet_mail_message_flags.rs) |

##### `ОписаниеПередаваемогоФайла`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `Имя`, `Name` | чтение и запись | — | [bsl-rt/src/transferable_file_description.rs](../../../crates/bsl-rt/src/transferable_file_description.rs) |
| `Хранение`, `Location` | чтение и запись | — | [bsl-rt/src/transferable_file_description.rs](../../../crates/bsl-rt/src/transferable_file_description.rs) |

##### `ОписаниеОповещения`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `ИмяПроцедуры`, `ProcedureName` | только чтение | — | [bsl-rt/src/notification.rs](../../../crates/bsl-rt/src/notification.rs) |
| `Модуль`, `Module` | только чтение | — | [bsl-rt/src/notification.rs](../../../crates/bsl-rt/src/notification.rs) |
| `ДополнительныеПараметры`, `AdditionalParameters` | только чтение | — | [bsl-rt/src/notification.rs](../../../crates/bsl-rt/src/notification.rs) |
| `ИмяПроцедурыОбработкиОшибки`, `ErrorHandlerProcedureName` | только чтение | — | [bsl-rt/src/notification.rs](../../../crates/bsl-rt/src/notification.rs) |
| `МодульОбработкиОшибки`, `ErrorHandlerModule` | только чтение | — | [bsl-rt/src/notification.rs](../../../crates/bsl-rt/src/notification.rs) |

##### `Файл`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `ПолноеИмя`, `FullName` | только чтение | — | [bsl-rt/src/file.rs](../../../crates/bsl-rt/src/file.rs) |
| `Имя`, `Name` | только чтение | — | [bsl-rt/src/transferable_file_description.rs](../../../crates/bsl-rt/src/transferable_file_description.rs) |
| `Путь`, `Path` | только чтение | — | [bsl-rt/src/file.rs](../../../crates/bsl-rt/src/file.rs) |
| `ИмяБезРасширения`, `BaseName` | только чтение | — | [bsl-rt/src/file.rs](../../../crates/bsl-rt/src/file.rs) |
| `Расширение`, `Extension` | только чтение | — | [bsl-rt/src/file.rs](../../../crates/bsl-rt/src/file.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `ПолучитьПредставлениеФайлаБиблиотекиМобильногоУстройства`, `GetMobileDeviceLibraryFilePresentation` | — | [bsl-rt/src/file.rs](../../../crates/bsl-rt/src/file.rs) |
| `ПолучитьКартинкуПредставленияФайлаБиблиотекиМобильногоУстройства`, `GetMobileDeviceLibraryFileThumbnail` | — | [bsl-rt/src/file.rs](../../../crates/bsl-rt/src/file.rs) |
| `ПолучитьКартинкуПредставленияФайлаБиблиотекиМобильногоУстройстваАсинх`, `GetMobileDeviceLibraryFileThumbnailAsync` | — | [bsl-rt/src/file.rs](../../../crates/bsl-rt/src/file.rs) |
| `НачатьПолучениеКартинкиПредставленияФайлаБиблиотекиМобильногоУстройства`, `BeginGetMobileDeviceLibraryFileThumbnail` | — | [bsl-rt/src/file.rs](../../../crates/bsl-rt/src/file.rs) |
| `НачатьИнициализацию`, `BeginInitialization` | — | [bsl-rt/src/file.rs](../../../crates/bsl-rt/src/file.rs) |
| `НачатьПроверкуСуществования`, `BeginCheckingExistence` | — | [bsl-rt/src/file.rs](../../../crates/bsl-rt/src/file.rs) |
| `НачатьПроверкуЭтоФайл`, `BeginCheckingIsFile` | — | [bsl-rt/src/file.rs](../../../crates/bsl-rt/src/file.rs) |
| `НачатьПроверкуЭтоКаталог`, `BeginCheckingIsDirectory` | — | [bsl-rt/src/file.rs](../../../crates/bsl-rt/src/file.rs) |
| `НачатьПолучениеРазмера`, `BeginGettingSize` | — | [bsl-rt/src/file.rs](../../../crates/bsl-rt/src/file.rs) |
| `НачатьПолучениеТолькоЧтения`, `BeginGettingReadOnly` | — | [bsl-rt/src/file.rs](../../../crates/bsl-rt/src/file.rs) |
| `НачатьПолучениеНевидимости`, `BeginGettingHidden` | — | [bsl-rt/src/file.rs](../../../crates/bsl-rt/src/file.rs) |
| `НачатьПолучениеВремениИзменения`, `BeginGettingModificationTime` | — | [bsl-rt/src/file.rs](../../../crates/bsl-rt/src/file.rs) |
| `НачатьПолучениеУниверсальногоВремениИзменения`, `BeginGettingModificationUniversalTime` | — | [bsl-rt/src/file.rs](../../../crates/bsl-rt/src/file.rs) |
| `НачатьУстановкуТолькоЧтения`, `BeginSettingReadOnly` | — | [bsl-rt/src/file.rs](../../../crates/bsl-rt/src/file.rs) |
| `НачатьУстановкуНевидимости`, `BeginSettingHidden` | — | [bsl-rt/src/file.rs](../../../crates/bsl-rt/src/file.rs) |
| `НачатьУстановкуВремениИзменения`, `BeginSettingModificationTime` | — | [bsl-rt/src/file.rs](../../../crates/bsl-rt/src/file.rs) |
| `НачатьУстановкуУниверсальногоВремениИзменения`, `BeginSettingModificationUniversalTime` | — | [bsl-rt/src/file.rs](../../../crates/bsl-rt/src/file.rs) |
| `ИнициализироватьАсинх`, `InitializeAsync` | — | [bsl-rt/src/file.rs](../../../crates/bsl-rt/src/file.rs) |
| `УстановитьТолькоЧтениеАсинх`, `SetReadOnlyAsync` | — | [bsl-rt/src/file.rs](../../../crates/bsl-rt/src/file.rs) |
| `УстановитьНевидимостьАсинх`, `SetHiddenAsync` | — | [bsl-rt/src/file.rs](../../../crates/bsl-rt/src/file.rs) |
| `УстановитьВремяИзмененияАсинх`, `SetModificationTimeAsync` | — | [bsl-rt/src/file.rs](../../../crates/bsl-rt/src/file.rs) |
| `УстановитьУниверсальноеВремяИзмененияАсинх`, `SetModificationUniversalTimeAsync` | — | [bsl-rt/src/file.rs](../../../crates/bsl-rt/src/file.rs) |
| `СуществуетАсинх`, `ExistsAsync` | — | [bsl-rt/src/file.rs](../../../crates/bsl-rt/src/file.rs) |
| `ЭтоФайлАсинх`, `IsFileAsync` | — | [bsl-rt/src/file.rs](../../../crates/bsl-rt/src/file.rs) |
| `ЭтоКаталогАсинх`, `IsDirectoryAsync` | — | [bsl-rt/src/file.rs](../../../crates/bsl-rt/src/file.rs) |
| `РазмерАсинх`, `SizeAsync` | — | [bsl-rt/src/file.rs](../../../crates/bsl-rt/src/file.rs) |
| `ПолучитьТолькоЧтениеАсинх`, `GetReadOnlyAsync` | — | [bsl-rt/src/file.rs](../../../crates/bsl-rt/src/file.rs) |
| `ПолучитьНевидимостьАсинх`, `GetHiddenAsync` | — | [bsl-rt/src/file.rs](../../../crates/bsl-rt/src/file.rs) |
| `ПолучитьВремяИзмененияАсинх`, `GetModificationTimeAsync` | — | [bsl-rt/src/file.rs](../../../crates/bsl-rt/src/file.rs) |
| `ПолучитьУниверсальноеВремяИзмененияАсинх`, `GetModificationUniversalTimeAsync` | — | [bsl-rt/src/file.rs](../../../crates/bsl-rt/src/file.rs) |
| `ПолучитьНевидимость`, `GetHidden` | — | [bsl-rt/src/file.rs](../../../crates/bsl-rt/src/file.rs) |
| `УстановитьНевидимость`, `SetHidden` | — | [bsl-rt/src/file.rs](../../../crates/bsl-rt/src/file.rs) |
| `ПолучитьТолькоЧтение`, `GetReadOnly` | — | [bsl-rt/src/file.rs](../../../crates/bsl-rt/src/file.rs) |
| `УстановитьТолькоЧтение`, `SetReadOnly` | — | [bsl-rt/src/file.rs](../../../crates/bsl-rt/src/file.rs) |
| `ПолучитьВремяИзменения`, `GetModificationTime` | — | [bsl-rt/src/file.rs](../../../crates/bsl-rt/src/file.rs) |
| `ПолучитьУниверсальноеВремяИзменения`, `GetModificationUniversalTime` | — | [bsl-rt/src/file.rs](../../../crates/bsl-rt/src/file.rs) |
| `УстановитьВремяИзменения`, `SetModificationTime` | — | [bsl-rt/src/file.rs](../../../crates/bsl-rt/src/file.rs) |
| `УстановитьУниверсальноеВремяИзменения`, `SetModificationUniversalTime` | — | [bsl-rt/src/file.rs](../../../crates/bsl-rt/src/file.rs) |
| `Существует`, `Exists` | — | [bsl-rt/src/file.rs](../../../crates/bsl-rt/src/file.rs) |
| `ЭтоФайл`, `IsFile` | — | [bsl-rt/src/file.rs](../../../crates/bsl-rt/src/file.rs) |
| `ЭтоКаталог`, `IsDirectory` | — | [bsl-rt/src/file.rs](../../../crates/bsl-rt/src/file.rs) |
| `Размер`, `Size` | — | [bsl-rt/src/file.rs](../../../crates/bsl-rt/src/file.rs) |

##### `ФиксированныйМассив`

| Метод | Описание | Реализация |
|---|---|---|
| `ВГраница`, `UBound` | — | [bsl-rt/src/fixed_array.rs](../../../crates/bsl-rt/src/fixed_array.rs) |
| `Количество`, `Count` | — | [bsl-rt/src/fixed_array.rs](../../../crates/bsl-rt/src/fixed_array.rs) |
| `Найти`, `Find` | — | [bsl-rt/src/fixed_array.rs](../../../crates/bsl-rt/src/fixed_array.rs) |
| `Получить`, `Get` | — | [bsl-rt/src/fixed_array.rs](../../../crates/bsl-rt/src/fixed_array.rs) |

##### `ФиксированнаяСтруктура`

| Метод | Описание | Реализация |
|---|---|---|
| `Количество`, `Count` | — | [bsl-rt/src/fixed_collections.rs](../../../crates/bsl-rt/src/fixed_collections.rs) |
| `Свойство`, `Property` | — | [bsl-rt/src/fixed_collections.rs](../../../crates/bsl-rt/src/fixed_collections.rs) |

##### `ФиксированноеСоответствие`

| Метод | Описание | Реализация |
|---|---|---|
| `Количество`, `Count` | — | [bsl-rt/src/fixed_collections.rs](../../../crates/bsl-rt/src/fixed_collections.rs) |
| `Получить`, `Get` | — | [bsl-rt/src/fixed_collections.rs](../../../crates/bsl-rt/src/fixed_collections.rs) |

##### `МенеджерФоновыхЗаданий`

| Метод | Описание | Реализация |
|---|---|---|
| `Выполнить`, `Execute` | — | [bsl-rt/src/background_jobs.rs](../../../crates/bsl-rt/src/background_jobs.rs) |
| `ПолучитьФоновыеЗадания`, `GetBackgroundJobs` | — | [bsl-rt/src/background_jobs.rs](../../../crates/bsl-rt/src/background_jobs.rs) |
| `НайтиПоУникальномуИдентификатору`, `FindByUUID` | — | [bsl-rt/src/background_jobs.rs](../../../crates/bsl-rt/src/background_jobs.rs) |
| `ОжидатьЗавершенияВыполнения`, `WaitForExecutionCompletion` | — | [bsl-rt/src/background_jobs.rs](../../../crates/bsl-rt/src/background_jobs.rs) |
| `ОжидатьЗавершения`, `WaitForCompletion` | — | [bsl-rt/src/background_jobs.rs](../../../crates/bsl-rt/src/background_jobs.rs) |

##### `ФоновоеЗадание`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `УникальныйИдентификатор`, `UUID` | только чтение | — | [bsl-rt/src/background_jobs.rs](../../../crates/bsl-rt/src/background_jobs.rs) |
| `ИмяМетода`, `MethodName` | только чтение | — | [bsl-rt/src/background_jobs.rs](../../../crates/bsl-rt/src/background_jobs.rs) |
| `Ключ`, `Key` | только чтение | — | [bsl-rt/src/background_jobs.rs](../../../crates/bsl-rt/src/background_jobs.rs) |
| `Наименование`, `Description` | только чтение | — | [bsl-rt/src/background_jobs.rs](../../../crates/bsl-rt/src/background_jobs.rs) |
| `Состояние`, `State` | только чтение | — | [bsl-rt/src/background_jobs.rs](../../../crates/bsl-rt/src/background_jobs.rs) |
| `Начало`, `Begin` | только чтение | — | [bsl-rt/src/background_jobs.rs](../../../crates/bsl-rt/src/background_jobs.rs) |
| `Конец`, `End` | только чтение | — | [bsl-rt/src/background_jobs.rs](../../../crates/bsl-rt/src/background_jobs.rs) |
| `ИнформацияОбОшибке`, `ErrorInfo` | только чтение | — | [bsl-rt/src/background_jobs.rs](../../../crates/bsl-rt/src/background_jobs.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `Отменить`, `Cancel` | — | [bsl-rt/src/background_jobs.rs](../../../crates/bsl-rt/src/background_jobs.rs) |
| `ОжидатьЗавершенияВыполнения`, `WaitForExecutionCompletion` | — | [bsl-rt/src/background_jobs.rs](../../../crates/bsl-rt/src/background_jobs.rs) |
| `ОжидатьЗавершения`, `WaitForCompletion` | — | [bsl-rt/src/background_jobs.rs](../../../crates/bsl-rt/src/background_jobs.rs) |
| `ПолучитьСообщенияПользователю`, `GetUserMessages` | — | [bsl-rt/src/background_jobs.rs](../../../crates/bsl-rt/src/background_jobs.rs) |

##### `СообщениеПользователю`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `Текст`, `Text` | чтение и запись | — | [bsl-rt/src/user_message.rs](../../../crates/bsl-rt/src/user_message.rs) |
| `Поле`, `Field` | чтение и запись | — | [bsl-rt/src/value/collections.rs](../../../crates/bsl-rt/src/value/collections.rs) |
| `КлючДанных`, `DataKey` | чтение и запись | — | [bsl-rt/src/user_message.rs](../../../crates/bsl-rt/src/user_message.rs) |
| `ПутьКДанным`, `DataPath` | чтение и запись | — | [bsl-rt/src/user_message.rs](../../../crates/bsl-rt/src/user_message.rs) |
| `ИдентификаторНазначения`, `TargetID` | чтение и запись | — | [bsl-rt/src/user_message.rs](../../../crates/bsl-rt/src/user_message.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `Сообщить`, `Message` | — | [bsl-rt/src/user_message.rs](../../../crates/bsl-rt/src/user_message.rs) |
| `УстановитьДанные`, `SetData` | — | [bsl-rt/src/user_message.rs](../../../crates/bsl-rt/src/user_message.rs) |

##### `ИнформацияОбОшибке`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `Описание`, `Description` | только чтение | — | [bsl-rt/src/error_info.rs](../../../crates/bsl-rt/src/error_info.rs) |
| `ИмяМодуля`, `ModuleName` | только чтение | — | [bsl-rt/src/error_info.rs](../../../crates/bsl-rt/src/error_info.rs) |
| `НомерСтроки`, `LineNumber` | только чтение | — | [bsl-rt/src/error_info.rs](../../../crates/bsl-rt/src/error_info.rs) |
| `ИсходнаяСтрока`, `SourceLine` | только чтение | — | [bsl-rt/src/error_info.rs](../../../crates/bsl-rt/src/error_info.rs) |
| `Код`, `Code` | только чтение | — | [bsl-rt/src/error_info.rs](../../../crates/bsl-rt/src/error_info.rs) |
| `Причина`, `Cause` | только чтение | — | [bsl-rt/src/error_info.rs](../../../crates/bsl-rt/src/error_info.rs) |
| `ДополнительнаяИнформация`, `AdditionalInformation` | только чтение | — | [bsl-rt/src/error_info.rs](../../../crates/bsl-rt/src/error_info.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `ЯвляетсяОшибкойКатегории`, `IsErrorOfCategory` | — | [bsl-rt/src/error_info.rs](../../../crates/bsl-rt/src/error_info.rs) |

##### `СписокЗначений`

| Метод | Описание | Реализация |
|---|---|---|
| `ВыгрузитьЗначения`, `UnloadValues` | — | [bsl-rt/src/value_list.rs](../../../crates/bsl-rt/src/value_list.rs) |
| `СортироватьПоПредставлению`, `SortByPresentation` | — | [bsl-rt/src/value_list.rs](../../../crates/bsl-rt/src/value_list.rs) |
| `Количество`, `Count` | — | [bsl-rt/src/value_list.rs](../../../crates/bsl-rt/src/value_list.rs) |
| `Добавить`, `Add` | — | [bsl-rt/src/value_list.rs](../../../crates/bsl-rt/src/value_list.rs) |
| `Удалить`, `Delete` | — | [bsl-rt/src/value_list.rs](../../../crates/bsl-rt/src/value_list.rs) |
| `Очистить`, `Clear` | — | [bsl-rt/src/value_list.rs](../../../crates/bsl-rt/src/value_list.rs) |
| `Вставить`, `Insert` | — | [bsl-rt/src/value_list.rs](../../../crates/bsl-rt/src/value_list.rs) |
| `Получить`, `Get` | — | [bsl-rt/src/value_list.rs](../../../crates/bsl-rt/src/value_list.rs) |
| `Скопировать`, `Copy` | — | [bsl-rt/src/value_list.rs](../../../crates/bsl-rt/src/value_list.rs) |
| `Сдвинуть`, `Move` | — | [bsl-rt/src/value_list.rs](../../../crates/bsl-rt/src/value_list.rs) |
| `Индекс`, `IndexOf` | — | [bsl-rt/src/value_list.rs](../../../crates/bsl-rt/src/value_list.rs) |
| `НайтиПоЗначению`, `FindByValue` | — | [bsl-rt/src/value_list.rs](../../../crates/bsl-rt/src/value_list.rs) |
| `СортироватьПоЗначению`, `SortByValue` | — | [bsl-rt/src/value_list.rs](../../../crates/bsl-rt/src/value_list.rs) |

##### `ЭлементСпискаЗначений`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `Значение`, `Value` | только чтение | — | [bsl-rt/src/value_list.rs](../../../crates/bsl-rt/src/value_list.rs) |
| `Представление`, `Presentation` | только чтение | — | [bsl-rt/src/value_list.rs](../../../crates/bsl-rt/src/value_list.rs) |
| `Пометка`, `Check` | только чтение | — | [bsl-rt/src/value_list.rs](../../../crates/bsl-rt/src/value_list.rs) |
| `Картинка`, `Picture` | только чтение | — | [bsl-rt/src/value_list.rs](../../../crates/bsl-rt/src/value_list.rs) |

##### `КоллекцияИндексовТаблицыЗначений`

| Метод | Описание | Реализация |
|---|---|---|
| `Количество`, `Count` | — | [bsl-rt/src/value_table_indexes.rs](../../../crates/bsl-rt/src/value_table_indexes.rs) |
| `Добавить`, `Add` | — | [bsl-rt/src/value_table_indexes.rs](../../../crates/bsl-rt/src/value_table_indexes.rs) |
| `Удалить`, `Delete` | — | [bsl-rt/src/value_table_indexes.rs](../../../crates/bsl-rt/src/value_table_indexes.rs) |
| `Очистить`, `Clear` | — | [bsl-rt/src/value_table_indexes.rs](../../../crates/bsl-rt/src/value_table_indexes.rs) |

##### `КвалификаторыСтроки`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `Длина`, `Length` | только чтение | — | [bsl-rt/src/type_description.rs](../../../crates/bsl-rt/src/type_description.rs) |
| `ДопустимаяДлина`, `AllowedLength` | только чтение | — | [bsl-rt/src/type_description.rs](../../../crates/bsl-rt/src/type_description.rs) |

##### `КвалификаторыЧисла`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `Разрядность`, `Digits` | только чтение | — | [bsl-rt/src/type_description.rs](../../../crates/bsl-rt/src/type_description.rs) |
| `РазрядностьДробнойЧасти`, `FractionDigits` | только чтение | — | [bsl-rt/src/type_description.rs](../../../crates/bsl-rt/src/type_description.rs) |
| `ДопустимыйЗнак`, `AllowedSign` | только чтение | — | [bsl-rt/src/type_description.rs](../../../crates/bsl-rt/src/type_description.rs) |

### `bsl-binbuf` 0.3.0

#### Глобальные функции

| Имя и псевдонимы | Вид | Описание | Реализация |
|---|---|---|---|
| `ПобитовоеИ`, `BitwiseAnd` | функция | — | [bsl-binbuf/src/lib.rs](../../../crates/bsl-binbuf/src/lib.rs) |
| `ПобитовоеИли`, `BitwiseOr` | функция | — | [bsl-binbuf/src/lib.rs](../../../crates/bsl-binbuf/src/lib.rs) |
| `ПобитовоеНе`, `BitwiseNot` | функция | — | [bsl-binbuf/src/lib.rs](../../../crates/bsl-binbuf/src/lib.rs) |
| `ПобитовоеИНе`, `BitwiseAndNot` | функция | — | [bsl-binbuf/src/lib.rs](../../../crates/bsl-binbuf/src/lib.rs) |
| `ПобитовоеИсключительноеИли`, `BitwiseXor` | функция | — | [bsl-binbuf/src/lib.rs](../../../crates/bsl-binbuf/src/lib.rs) |
| `ПобитовыйСдвигВлево`, `BitwiseShiftLeft` | функция | — | [bsl-binbuf/src/lib.rs](../../../crates/bsl-binbuf/src/lib.rs) |
| `ПобитовыйСдвигВправо`, `BitwiseShiftRight` | функция | — | [bsl-binbuf/src/lib.rs](../../../crates/bsl-binbuf/src/lib.rs) |
| `ПроверитьБит`, `CheckBit` | функция | — | [bsl-binbuf/src/lib.rs](../../../crates/bsl-binbuf/src/lib.rs) |
| `ПроверитьПоБитовойМаске`, `CheckByBitMask` | функция | — | [bsl-binbuf/src/lib.rs](../../../crates/bsl-binbuf/src/lib.rs) |
| `УстановитьБит`, `SetBit` | функция | — | [bsl-binbuf/src/lib.rs](../../../crates/bsl-binbuf/src/lib.rs) |
| `ЧислоИзШестнадцатеричнойСтроки`, `NumberFromHexString` | функция | — | [bsl-binbuf/src/lib.rs](../../../crates/bsl-binbuf/src/lib.rs) |
| `ЧислоИзДвоичнойСтроки`, `NumberFromBinaryString` | функция | — | [bsl-binbuf/src/lib.rs](../../../crates/bsl-binbuf/src/lib.rs) |

#### Конструкторы

| Имя и псевдонимы | Описание | Реализация |
|---|---|---|
| `БуферДвоичныхДанных`, `BinaryDataBuffer` | Представление буфера временно живёт в `bsl-rt` (см. док рефакторинга: `ДвоичныеДанные` ссылается на буферы, а зависимость `bsl-rt -> bsl-binbuf` запрещена), но ТИП принадлежит этому компоненту — конструктор объявлен здесь и лишь делегирует представлению. | [bsl-binbuf/src/lib.rs](../../../crates/bsl-binbuf/src/lib.rs) |

#### Типы объектов

Нет.

### `bsl-crypto` 0.3.0

#### Глобальные функции

Нет.

#### Конструкторы

| Имя и псевдонимы | Описание | Реализация |
|---|---|---|
| `ХешированиеДанных`, `DataHashing` | — | [bsl-crypto/src/lib.rs](../../../crates/bsl-crypto/src/lib.rs) |
| `ТокенДоступа`, `AccessToken` — *объявлено без реализации* | — | [bsl-crypto/src/lib.rs](../../../crates/bsl-crypto/src/lib.rs) |

#### Типы объектов

| Имя | Представление типа | Дополнительные имена | Описание | Реализация |
|---|---|---|---|---|
| `ХешированиеДанных` | `ХешированиеДанных` | `DataHashing` | Версия компонента в заголовке байт-кода. | [bsl-crypto/src/lib.rs](../../../crates/bsl-crypto/src/lib.rs) |
| `ТокенДоступа` | `ТокенДоступа` | `AccessToken` | Тип токена доступа, объявленный для компиляции исходного кода конфигурации. | [bsl-crypto/src/lib.rs](../../../crates/bsl-crypto/src/lib.rs) |

#### Члены объектов

##### `ХешированиеДанных`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `ХешСумма`, `HashSum` | только чтение | — | [bsl-crypto/src/lib.rs](../../../crates/bsl-crypto/src/lib.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `Добавить`, `Add` | — | [bsl-crypto/src/lib.rs](../../../crates/bsl-crypto/src/lib.rs) |

### `bsl-http` 0.4.0

Зависимости: `bsl-stream=0.3.0`.

#### Глобальные функции

Нет.

#### Конструкторы

| Имя и псевдонимы | Описание | Реализация |
|---|---|---|
| `СертификатыУдостоверяющихЦентровLinux`, `LinuxCertificationAuthorityCertificates` | — | [bsl-http/src/lib.rs](../../../crates/bsl-http/src/lib.rs) |
| `HTTPЗапрос`, `HTTPRequest` | — | [bsl-http/src/lib.rs](../../../crates/bsl-http/src/lib.rs) |
| `HTTPСоединение`, `HTTPConnection` | — | [bsl-http/src/lib.rs](../../../crates/bsl-http/src/lib.rs) |
| `ИнтернетПрокси`, `InternetProxy` | — | [bsl-http/src/lib.rs](../../../crates/bsl-http/src/lib.rs) |
| `СертификатыУдостоверяющихЦентровОС`, `CertificationAuthorityCertificatesOS` | — | [bsl-http/src/lib.rs](../../../crates/bsl-http/src/lib.rs) |
| `ЗащищенноеСоединениеOpenSSL`, `OpenSSLSecureConnection` | — | [bsl-http/src/lib.rs](../../../crates/bsl-http/src/lib.rs) |
| `СертификатыУдостоверяющихЦентровФайл`, `FileCertificationAuthorityCertificates` | — | [bsl-http/src/lib.rs](../../../crates/bsl-http/src/lib.rs) |
| `СертификатКлиентаФайл`, `FileClientCertificate` | — | [bsl-http/src/lib.rs](../../../crates/bsl-http/src/lib.rs) |
| `HTTPСервисОтвет`, `HTTPServiceResponse` — *объявлено без реализации* | — | [bsl-http/src/lib.rs](../../../crates/bsl-http/src/lib.rs) |
| `СертификатКлиентаWindows`, `WindowsClientCertificate` — *объявлено без реализации* | — | [bsl-http/src/lib.rs](../../../crates/bsl-http/src/lib.rs) |
| `СертификатыУдостоверяющихЦентровWindows`, `WindowsCertificationAuthorityCertificates` — *объявлено без реализации* | — | [bsl-http/src/lib.rs](../../../crates/bsl-http/src/lib.rs) |

#### Типы объектов

| Имя | Представление типа | Дополнительные имена | Описание | Реализация |
|---|---|---|---|---|
| `СертификатыУдостоверяющихЦентровLinux` | `СертификатыУдостоверяющихЦентровLinux` | `LinuxCertificationAuthorityCertificates` | — | [bsl-http/src/objects.rs](../../../crates/bsl-http/src/objects.rs) |
| `HTTPЗапрос` | `HTTPЗапрос` | `HTTPRequest` | — | [bsl-http/src/objects.rs](../../../crates/bsl-http/src/objects.rs) |
| `HTTPОтвет` | `HTTPОтвет` | `HTTPResponse` | — | [bsl-http/src/objects.rs](../../../crates/bsl-http/src/objects.rs) |
| `HTTPСоединение` | `HTTPСоединение` | `HTTPConnection` | — | [bsl-http/src/objects.rs](../../../crates/bsl-http/src/objects.rs) |
| `ИнтернетПрокси` | `ИнтернетПрокси` | `InternetProxy` | — | [bsl-http/src/objects.rs](../../../crates/bsl-http/src/objects.rs) |
| `СертификатыУдостоверяющихЦентровОС` | `СертификатыУдостоверяющихЦентровОС` | `CertificationAuthorityCertificatesOS` | — | [bsl-http/src/objects.rs](../../../crates/bsl-http/src/objects.rs) |
| `ЗащищенноеСоединениеOpenSSL` | `ЗащищенноеСоединениеOpenSSL` | `OpenSSLSecureConnection` | — | [bsl-http/src/objects.rs](../../../crates/bsl-http/src/objects.rs) |
| `СертификатыУдостоверяющихЦентровФайл` | `СертификатыУдостоверяющихЦентровФайл` | `FileCertificationAuthorityCertificates` | — | [bsl-http/src/objects.rs](../../../crates/bsl-http/src/objects.rs) |
| `СертификатКлиентаФайл` | `СертификатКлиентаФайл` | `FileClientCertificate` | — | [bsl-http/src/objects.rs](../../../crates/bsl-http/src/objects.rs) |
| `СертификатКлиентаWindows` | `СертификатКлиентаWindows` | `WindowsClientCertificate` | Дескриптор платформенного Windows-сертификата. | [bsl-http/src/objects.rs](../../../crates/bsl-http/src/objects.rs) |
| `СертификатыУдостоверяющихЦентровWindows` | `СертификатыУдостоверяющихЦентровWindows` | `WindowsCertificationAuthorityCertificates` | Дескриптор сертификатов удостоверяющих центров из хранилища Windows. | [bsl-http/src/objects.rs](../../../crates/bsl-http/src/objects.rs) |

#### Члены объектов

##### `HTTPЗапрос`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `АдресРесурса`, `ResourceAddress` | чтение и запись | — | [bsl-http/src/objects.rs](../../../crates/bsl-http/src/objects.rs) |
| `Заголовки`, `Headers` | чтение и запись | — | [bsl-http/src/objects.rs](../../../crates/bsl-http/src/objects.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `УстановитьТелоИзСтроки`, `SetBodyFromString` | — | [bsl-http/src/objects.rs](../../../crates/bsl-http/src/objects.rs) |
| `УстановитьТелоИзДвоичныхДанных`, `SetBodyFromBinaryData` | — | [bsl-http/src/objects.rs](../../../crates/bsl-http/src/objects.rs) |
| `ПолучитьТелоКакДвоичныеДанные`, `GetBodyAsBinaryData` | — | [bsl-http/src/objects.rs](../../../crates/bsl-http/src/objects.rs) |
| `ПолучитьТелоКакСтроку`, `GetBodyAsString` | — | [bsl-http/src/objects.rs](../../../crates/bsl-http/src/objects.rs) |
| `ПолучитьТелоКакПоток`, `GetBodyAsStream` | — | [bsl-http/src/objects.rs](../../../crates/bsl-http/src/objects.rs) |

##### `HTTPОтвет`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `КодСостояния`, `StatusCode` | только чтение | — | [bsl-http/src/objects.rs](../../../crates/bsl-http/src/objects.rs) |
| `Заголовки`, `Headers` | только чтение | — | [bsl-http/src/objects.rs](../../../crates/bsl-http/src/objects.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `ПолучитьТелоКакДвоичныеДанные`, `GetBodyAsBinaryData` | — | [bsl-http/src/objects.rs](../../../crates/bsl-http/src/objects.rs) |
| `ПолучитьТелоКакСтроку`, `GetBodyAsString` | — | [bsl-http/src/objects.rs](../../../crates/bsl-http/src/objects.rs) |
| `ПолучитьТелоКакПоток`, `GetBodyAsStream` | — | [bsl-http/src/objects.rs](../../../crates/bsl-http/src/objects.rs) |

##### `HTTPСоединение`

| Метод | Описание | Реализация |
|---|---|---|
| `ВызватьHTTPМетод`, `CallHTTPMethod` | — | [bsl-http/src/objects.rs](../../../crates/bsl-http/src/objects.rs) |
| `Удалить`, `Delete` | — | [bsl-http/src/objects.rs](../../../crates/bsl-http/src/objects.rs) |
| `Получить`, `Get` | — | [bsl-http/src/objects.rs](../../../crates/bsl-http/src/objects.rs) |
| `ПолучитьЗаголовки`, `Head` | — | [bsl-http/src/objects.rs](../../../crates/bsl-http/src/objects.rs) |
| `Изменить`, `Patch` | — | [bsl-http/src/objects.rs](../../../crates/bsl-http/src/objects.rs) |
| `ОтправитьДляОбработки`, `Post` | — | [bsl-http/src/objects.rs](../../../crates/bsl-http/src/objects.rs) |
| `Записать`, `Put` | `ПоместитьВоВременноеХранилище`: запись по своему адресу — в локальную карту; по адресу вызывателя (в сеансе задания) — в staging; чужой адрес — ловимая ошибка, значения владельца целы. | [bsl-rt/src/temp_storage.rs](../../../crates/bsl-rt/src/temp_storage.rs) |
| `ВызватьHTTPМетодАсинх`, `CallHTTPMethodAsync` | — | [bsl-http/src/objects.rs](../../../crates/bsl-http/src/objects.rs) |
| `УдалитьАсинх`, `DeleteAsync` | — | [bsl-http/src/objects.rs](../../../crates/bsl-http/src/objects.rs) |
| `ПолучитьАсинх`, `GetAsync` | — | [bsl-http/src/objects.rs](../../../crates/bsl-http/src/objects.rs) |
| `ПолучитьЗаголовкиАсинх`, `HeadAsync` | — | [bsl-http/src/objects.rs](../../../crates/bsl-http/src/objects.rs) |
| `ИзменитьАсинх`, `PatchAsync` | — | [bsl-http/src/objects.rs](../../../crates/bsl-http/src/objects.rs) |
| `ОтправитьДляОбработкиАсинх`, `PostAsync` | — | [bsl-http/src/objects.rs](../../../crates/bsl-http/src/objects.rs) |
| `ЗаписатьАсинх`, `PutAsync` | — | [bsl-http/src/objects.rs](../../../crates/bsl-http/src/objects.rs) |

##### `ИнтернетПрокси`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `НеИспользоватьПроксиДляАдресов`, `BypassProxyForAddresses` | только чтение | — | [bsl-http/src/objects.rs](../../../crates/bsl-http/src/objects.rs) |
| `НеИспользоватьПроксиДляЛокальныхАдресов`, `BypassProxyForLocalAddresses` | чтение и запись | — | [bsl-http/src/objects.rs](../../../crates/bsl-http/src/objects.rs) |
| `Пользователь`, `User` | чтение и запись | — | [bsl-http/src/objects.rs](../../../crates/bsl-http/src/objects.rs) |
| `Пароль`, `Password` | чтение и запись | — | [bsl-http/src/objects.rs](../../../crates/bsl-http/src/objects.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `Установить`, `Set` | — | [bsl-http/src/objects.rs](../../../crates/bsl-http/src/objects.rs) |

##### `ЗащищенноеСоединениеOpenSSL`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `СертификатКлиента`, `ClientCertificate` | только чтение | — | [bsl-http/src/objects.rs](../../../crates/bsl-http/src/objects.rs) |
| `РежимПроверкиОтзываTLSСертификатаСервера`, `ServerTLSCertificateRevocationCheckMode` | только чтение | — | [bsl-http/src/objects.rs](../../../crates/bsl-http/src/objects.rs) |
| `СертификатыУдостоверяющихЦентров`, `CertificationAuthorityCertificates` | только чтение | — | [bsl-http/src/objects.rs](../../../crates/bsl-http/src/objects.rs) |

### `bsl-json` 0.3.0

#### Глобальные функции

| Имя и псевдонимы | Вид | Описание | Реализация |
|---|---|---|---|
| `ПрочитатьJSON`, `ReadJSON` | функция | — | [bsl-json/src/bridge.rs](../../../crates/bsl-json/src/bridge.rs) |
| `ЗаписатьJSON`, `WriteJSON` | процедура | — | [bsl-json/src/bridge.rs](../../../crates/bsl-json/src/bridge.rs) |
| `ЗаписатьДатуJSON`, `WriteJSONDate` | функция | — | [bsl-json/src/bridge.rs](../../../crates/bsl-json/src/bridge.rs) |
| `ПрочитатьДатуJSON`, `ReadJSONDate` | функция | — | [bsl-json/src/bridge.rs](../../../crates/bsl-json/src/bridge.rs) |
| `ЗаписатьЗначениеJSON`, `WriteJSONValue` | функция | — | [bsl-json/src/bridge.rs](../../../crates/bsl-json/src/bridge.rs) |
| `ПрочитатьЗначениеJSON`, `ReadJSONValue` | функция | — | [bsl-json/src/bridge.rs](../../../crates/bsl-json/src/bridge.rs) |

#### Конструкторы

| Имя и псевдонимы | Описание | Реализация |
|---|---|---|
| `ЧтениеJSON`, `JSONReader` | — | [bsl-json/src/bridge.rs](../../../crates/bsl-json/src/bridge.rs) |
| `ЗаписьJSON`, `JSONWriter` | — | [bsl-json/src/bridge.rs](../../../crates/bsl-json/src/bridge.rs) |
| `ПараметрыЗаписиJSON`, `JSONWriterSettings` | — | [bsl-json/src/bridge.rs](../../../crates/bsl-json/src/bridge.rs) |
| `НастройкиСериализацииJSON`, `JSONSerializerSettings` | — | [bsl-json/src/bridge.rs](../../../crates/bsl-json/src/bridge.rs) |

#### Типы объектов

| Имя | Представление типа | Дополнительные имена | Описание | Реализация |
|---|---|---|---|---|
| `ЧтениеJSON` | `Чтение JSON` | `JSONReader` | — | [bsl-json/src/objects.rs](../../../crates/bsl-json/src/objects.rs) |
| `НастройкиСериализацииJSON` | `Настройки сериализации JSON` | `JSONSerializerSettings` | — | [bsl-json/src/objects.rs](../../../crates/bsl-json/src/objects.rs) |
| `ПараметрыЗаписиJSON` | `Параметры записи JSON` | `JSONWriterSettings` | — | [bsl-json/src/objects.rs](../../../crates/bsl-json/src/objects.rs) |
| `ЗаписьJSON` | `Запись JSON` | `JSONWriter` | — | [bsl-json/src/objects.rs](../../../crates/bsl-json/src/objects.rs) |

#### Члены объектов

##### `ЧтениеJSON`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `ТипТекущегоЗначения`, `CurrentValueType` | только чтение | — | [bsl-json/src/objects.rs](../../../crates/bsl-json/src/objects.rs) |
| `ТекущееЗначение`, `CurrentValue` | только чтение | — | [bsl-json/src/objects.rs](../../../crates/bsl-json/src/objects.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `УстановитьСтроку`, `SetString` | — | [bsl-json/src/objects.rs](../../../crates/bsl-json/src/objects.rs) |
| `ОткрытьФайл`, `OpenFile` | — | [bsl-json/src/objects.rs](../../../crates/bsl-json/src/objects.rs) |
| `ОткрытьПоток`, `OpenStream` | — | [bsl-json/src/objects.rs](../../../crates/bsl-json/src/objects.rs) |
| `Закрыть`, `Close` | — | [bsl-json/src/objects.rs](../../../crates/bsl-json/src/objects.rs) |
| `Прочитать`, `Read` | — | [bsl-json/src/objects.rs](../../../crates/bsl-json/src/objects.rs) |
| `Пропустить`, `Skip` | — | [bsl-json/src/objects.rs](../../../crates/bsl-json/src/objects.rs) |

##### `НастройкиСериализацииJSON`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `ФорматСериализацииДаты`, `DateSerializationFormat` | чтение и запись | — | [bsl-json/src/objects.rs](../../../crates/bsl-json/src/objects.rs) |
| `ВариантЗаписиДаты`, `DateWritingVariant` | чтение и запись | — | [bsl-json/src/objects.rs](../../../crates/bsl-json/src/objects.rs) |
| `СериализовыватьМассивыКакОбъекты`, `SerializeArraysAsObjects` | чтение и запись | — | [bsl-json/src/objects.rs](../../../crates/bsl-json/src/objects.rs) |

##### `ПараметрыЗаписиJSON`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `ПереносСтрок`, `LineBreak` | только чтение | — | [bsl-json/src/objects.rs](../../../crates/bsl-json/src/objects.rs) |
| `СимволыОтступа`, `IndentChars` | только чтение | — | [bsl-json/src/objects.rs](../../../crates/bsl-json/src/objects.rs) |
| `ИспользоватьДвойныеКавычки`, `UseDoubleQuotes` | только чтение | — | [bsl-json/src/objects.rs](../../../crates/bsl-json/src/objects.rs) |
| `ЭкранированиеСимволов`, `EscapeCharacters` | только чтение | — | [bsl-json/src/objects.rs](../../../crates/bsl-json/src/objects.rs) |
| `ЭкранироватьУгловыеСкобки`, `EscapeAngleBrackets` | только чтение | — | [bsl-json/src/objects.rs](../../../crates/bsl-json/src/objects.rs) |
| `ЭкранироватьРазделителиСтрок`, `EscapeLineSeparators` | только чтение | — | [bsl-json/src/objects.rs](../../../crates/bsl-json/src/objects.rs) |
| `ЭкранироватьАмперсанд`, `EscapeAmpersand` | только чтение | — | [bsl-json/src/objects.rs](../../../crates/bsl-json/src/objects.rs) |
| `ЭкранироватьОдинарныеКавычки`, `EscapeSingleQuotes` | только чтение | — | [bsl-json/src/objects.rs](../../../crates/bsl-json/src/objects.rs) |
| `ЭкранироватьСлеш`, `EscapeSlash` | только чтение | — | [bsl-json/src/objects.rs](../../../crates/bsl-json/src/objects.rs) |

##### `ЗаписьJSON`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `ПроверятьСтруктуру`, `CheckStructure` | чтение и запись | — | [bsl-json/src/objects.rs](../../../crates/bsl-json/src/objects.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `УстановитьСтроку`, `SetString` | — | [bsl-json/src/objects.rs](../../../crates/bsl-json/src/objects.rs) |
| `ОткрытьФайл`, `OpenFile` | — | [bsl-json/src/objects.rs](../../../crates/bsl-json/src/objects.rs) |
| `Закрыть`, `Close` | — | [bsl-json/src/objects.rs](../../../crates/bsl-json/src/objects.rs) |
| `ЗаписатьНачалоОбъекта`, `WriteStartObject` | — | [bsl-json/src/objects.rs](../../../crates/bsl-json/src/objects.rs) |
| `ЗаписатьКонецОбъекта`, `WriteEndObject` | — | [bsl-json/src/objects.rs](../../../crates/bsl-json/src/objects.rs) |
| `ЗаписатьНачалоМассива`, `WriteStartArray` | — | [bsl-json/src/objects.rs](../../../crates/bsl-json/src/objects.rs) |
| `ЗаписатьКонецМассива`, `WriteEndArray` | — | [bsl-json/src/objects.rs](../../../crates/bsl-json/src/objects.rs) |
| `ЗаписатьИмяСвойства`, `WritePropertyName` | — | [bsl-json/src/objects.rs](../../../crates/bsl-json/src/objects.rs) |
| `ЗаписатьЗначение`, `WriteValue` | — | [bsl-json/src/objects.rs](../../../crates/bsl-json/src/objects.rs) |

### `bsl-orm` 0.2.0

#### Глобальные свойства

| Имя и псевдонимы | Функция получения |
|---|---|
| `КритерииОтбора`, `FilterCriteria` | `ПолучитьКритерииОтбораORM` |
| `WSСсылки`, `WSReferences` | `WSСсылки`, `WSReferences` |
| `ШаблоныНастроекВторогоФактораАутентификации`, `SecondAuthenticationFactorSettingsTemplates` | `ПолучитьШаблоныНастроекВторогоФактораАутентификацииORM` |
| `ВнешниеОбработки`, `ExternalDataProcessors` | `ПолучитьВнешниеОбработкиORM` |
| `ВнешниеОтчеты`, `ExternalReports` | `ПолучитьВнешниеОтчетыORM` |
| `ИсторияДанных`, `DataHistory` | `ПолучитьИсториюДанныхORM` |
| `Метаданные`, `Metadata` | `ПолучитьМетаданныеORM` |
| `Справочники`, `Catalogs` | `ПолучитьСправочникиORM` |
| `Документы`, `Documents` | `ПолучитьДокументыORM` |
| `ЖурналыДокументов`, `DocumentJournals` | `ПолучитьЖурналыДокументовORM` |
| `Отчеты`, `Reports` | `ПолучитьОтчетыORM` |
| `Обработки`, `DataProcessors` | `ПолучитьОбработкиORM` |
| `РегистрыНакопления`, `AccumulationRegisters` | `ПолучитьРегистрыНакопленияORM` |
| `РегистрыБухгалтерии`, `AccountingRegisters` | `ПолучитьРегистрыБухгалтерииORM` |
| `РегистрыРасчета`, `CalculationRegisters` | `ПолучитьРегистрыРасчетаORM` |
| `ПланыСчетов`, `ChartsOfAccounts` | `ПолучитьПланыСчетовORM` |
| `ПланыВидовХарактеристик`, `ChartsOfCharacteristicTypes` | `ПолучитьПланыВидовХарактеристикORM` |
| `ПланыВидовРасчета`, `ChartsOfCalculationTypes` | `ПолучитьПланыВидовРасчетаORM` |
| `БизнесПроцессы`, `BusinessProcesses` | `ПолучитьБизнесПроцессыORM` |
| `Задачи`, `Tasks` | `ПолучитьЗадачиORM` |
| `ХранилищаНастроек`, `SettingsStorages` | `ПолучитьХранилищаНастроекORM` |
| `Последовательности`, `Sequences` | `ПолучитьПоследовательностиORM` |
| `РегистрыСведений`, `InformationRegisters` | `ПолучитьРегистрыСведенийORM` |
| `Константы`, `Constants` | `ПолучитьКонстантыORM` |
| `РасширенияКонфигурации`, `ConfigurationExtensions` | `ПолучитьРасширенияКонфигурацииORM` |
| `ПользователиИнформационнойБазы`, `InfoBaseUsers` | `ПолучитьПользователейИнформационнойБазыORM` |
| `ПолитикиПаролейПользователей`, `UserPasswordPolicies` | `ПолучитьПолитикиПаролейПользователейORM` |
| `БлокировкаАутентификации`, `AuthenticationLock` | `ПолучитьБлокировкуАутентификацииORM` |
| `Перечисления`, `Enums` | `ПолучитьПеречисленияORM` |
| `ПланыОбмена`, `ExchangePlans` | `ПолучитьПланыОбменаORM` |
| `БиблиотекаКартинок`, `PictureLib` | `ПолучитьБиблиотекуКартинокORM` |
| `СервисыИнтеграции`, `IntegrationServices` | `ПолучитьСервисыИнтеграцииORM` |

#### Глобальные функции

| Имя и псевдонимы | Вид | Описание | Реализация |
|---|---|---|---|
| `ПолучитьКритерииОтбораORM` | функция | — | [bsl-orm/src/infobase.rs](../../../crates/bsl-orm/src/infobase.rs) |
| `WSСсылки`, `WSReferences` | функция | — | [bsl-orm/src/infobase.rs](../../../crates/bsl-orm/src/infobase.rs) |
| `ПолучитьРазмерДанныхБазыДанных`, `GetDatabaseDataSize` | функция | — | [bsl-orm/src/infobase.rs](../../../crates/bsl-orm/src/infobase.rs) |
| `УдалитьОбъекты`, `DeleteObjects` | процедура | — | [bsl-orm/src/infobase.rs](../../../crates/bsl-orm/src/infobase.rs) |
| `ПолучитьДанныеВыбора`, `GetChoiceData` | функция | — | [bsl-orm/src/globals.rs](../../../crates/bsl-orm/src/globals.rs) |
| `УстановитьСоставСтандартногоИнтерфейсаOData`, `SetStandardODataInterfaceContent` | процедура | — | [bsl-orm/src/globals.rs](../../../crates/bsl-orm/src/globals.rs) |
| `ПолучитьСоставСтандартногоИнтерфейсаOData`, `GetStandardODataInterfaceContent` | функция | — | [bsl-orm/src/globals.rs](../../../crates/bsl-orm/src/globals.rs) |
| `ПолноеИмяПользователя`, `UserFullName` | функция | — | [bsl-orm/src/infobase_users.rs](../../../crates/bsl-orm/src/infobase_users.rs) |
| `УдалитьДанныеИнформационнойБазы`, `EraseInfoBaseData` | процедура | — | [bsl-orm/src/infobase.rs](../../../crates/bsl-orm/src/infobase.rs) |
| `РазблокироватьДанныеДляРедактирования`, `UnlockDataForEdit` | процедура | — | [bsl-orm/src/infobase.rs](../../../crates/bsl-orm/src/infobase.rs) |
| `ЗаблокироватьДанныеДляРедактирования`, `LockDataForEdit` | процедура | — | [bsl-orm/src/infobase.rs](../../../crates/bsl-orm/src/infobase.rs) |
| `ПолучитьМетаданныеORM` | функция | — | [bsl-orm/src/catalog_object.rs](../../../crates/bsl-orm/src/catalog_object.rs) |
| `ПолучитьСправочникиORM` | функция | — | [bsl-orm/src/managers.rs](../../../crates/bsl-orm/src/managers.rs) |
| `ПолучитьРегистрыСведенийORM` | функция | — | [bsl-orm/src/managers.rs](../../../crates/bsl-orm/src/managers.rs) |
| `ПолучитьКонстантыORM` | функция | — | [bsl-orm/src/managers.rs](../../../crates/bsl-orm/src/managers.rs) |
| `РольДоступна`, `IsInRole` | функция | — | [bsl-orm/src/access_rights.rs](../../../crates/bsl-orm/src/access_rights.rs) |
| `ПолучитьРасширенияКонфигурацииORM` | функция | — | [bsl-orm/src/configuration_extension.rs](../../../crates/bsl-orm/src/configuration_extension.rs) |
| `ПолучитьПользователейИнформационнойБазыORM` | функция | — | [bsl-orm/src/infobase_users.rs](../../../crates/bsl-orm/src/infobase_users.rs) |
| `ПолучитьПеречисленияORM` | функция | — | [bsl-orm/src/managers.rs](../../../crates/bsl-orm/src/managers.rs) |
| `ПолучитьБиблиотекуКартинокORM` | функция | — | [bsl-orm/src/picture_library.rs](../../../crates/bsl-orm/src/picture_library.rs) |
| `КонфигурацияИзменена`, `ConfigurationChanged` | функция | — | [bsl-orm/src/globals.rs](../../../crates/bsl-orm/src/globals.rs) |
| `ПолучитьПланыОбменаORM` | функция | — | [bsl-orm/src/managers.rs](../../../crates/bsl-orm/src/managers.rs) |
| `НачатьТранзакцию`, `BeginTransaction` | процедура | Начинает транзакцию на соединении текущего сеанса. | [bsl-orm/src/data_access.rs](../../../crates/bsl-orm/src/data_access.rs) |
| `ЗафиксироватьТранзакцию`, `CommitTransaction` | процедура | Фиксирует текущую транзакцию соединения сеанса. | [bsl-orm/src/data_access.rs](../../../crates/bsl-orm/src/data_access.rs) |
| `ОтменитьТранзакцию`, `RollbackTransaction` | процедура | Отменяет текущую транзакцию соединения сеанса. | [bsl-orm/src/data_access.rs](../../../crates/bsl-orm/src/data_access.rs) |
| `ПравоДоступа`, `AccessRight` | функция | — | [bsl-orm/src/access_rights.rs](../../../crates/bsl-orm/src/access_rights.rs) |
| `ИмяПользователя`, `UserName` | функция | — | [bsl-orm/src/infobase_users.rs](../../../crates/bsl-orm/src/infobase_users.rs) |
| `ПредопределенноеЗначение`, `PredefinedValue` | функция | — | [bsl-orm/src/globals.rs](../../../crates/bsl-orm/src/globals.rs) |
| `МонопольныйРежим`, `ExclusiveMode` | функция | Возвращает состояние монопольного режима текущего соединения. | [bsl-orm/src/data_access.rs](../../../crates/bsl-orm/src/data_access.rs) |
| `УстановитьМонопольныйРежим`, `SetExclusiveMode` | процедура | Изменяет монопольный режим текущего соединения. | [bsl-orm/src/data_access.rs](../../../crates/bsl-orm/src/data_access.rs) |
| `ПолучитьСервисыИнтеграцииORM` | функция | — | [bsl-orm/src/infobase.rs](../../../crates/bsl-orm/src/infobase.rs) |
| `ПолучитьОбновлениеПредопределенныхДанныхИнформационнойБазы`, `GetInfoBasePredefinedData` | функция | Возвращает режим обновления предопределённых данных информационной базы. | [bsl-orm/src/data_access.rs](../../../crates/bsl-orm/src/data_access.rs) |
| `УстановитьОбновлениеПредопределенныхДанныхИнформационнойБазы`, `SetInfoBasePredefinedDataUpdate` | процедура | Изменяет режим обновления предопределённых данных информационной базы. | [bsl-orm/src/data_access.rs](../../../crates/bsl-orm/src/data_access.rs) |
| `ИнициализироватьПредопределенныеДанные`, `InitializePredefinedData` | процедура | Инициализирует предопределённые данные текущей области базы. | [bsl-orm/src/data_access.rs](../../../crates/bsl-orm/src/data_access.rs) |
| `ПолучитьФункциональнуюОпцию`, `GetFunctionalOption` | функция | Возвращает значение функциональной опции для текущего сеанса. | [bsl-orm/src/data_access.rs](../../../crates/bsl-orm/src/data_access.rs) |
| `ТранзакцияАктивна`, `TransactionActive` | функция | Возвращает состояние транзакции на соединении текущего сеанса. | [bsl-orm/src/data_access.rs](../../../crates/bsl-orm/src/data_access.rs) |
| `ТекущийЯзык`, `CurrentLanguage` | функция | — | [bsl-orm/src/globals.rs](../../../crates/bsl-orm/src/globals.rs) |
| `НайтиПоСсылкам`, `FindByRef` | функция | Ищет ссылки в доступных текущему пользователю данных. | [bsl-orm/src/data_access.rs](../../../crates/bsl-orm/src/data_access.rs) |
| `ПолучитьДокументыORM` | функция | — | [bsl-orm/src/managers.rs](../../../crates/bsl-orm/src/managers.rs) |
| `ПолучитьЖурналыДокументовORM` | функция | — | [bsl-orm/src/managers.rs](../../../crates/bsl-orm/src/managers.rs) |
| `ПолучитьОтчетыORM` | функция | — | [bsl-orm/src/managers.rs](../../../crates/bsl-orm/src/managers.rs) |
| `ПолучитьОбработкиORM` | функция | — | [bsl-orm/src/managers.rs](../../../crates/bsl-orm/src/managers.rs) |
| `ПолучитьРегистрыНакопленияORM` | функция | — | [bsl-orm/src/managers.rs](../../../crates/bsl-orm/src/managers.rs) |
| `ПолучитьРегистрыБухгалтерииORM` | функция | — | [bsl-orm/src/managers.rs](../../../crates/bsl-orm/src/managers.rs) |
| `ПолучитьРегистрыРасчетаORM` | функция | — | [bsl-orm/src/managers.rs](../../../crates/bsl-orm/src/managers.rs) |
| `ПолучитьПланыСчетовORM` | функция | — | [bsl-orm/src/managers.rs](../../../crates/bsl-orm/src/managers.rs) |
| `ПолучитьПланыВидовХарактеристикORM` | функция | — | [bsl-orm/src/managers.rs](../../../crates/bsl-orm/src/managers.rs) |
| `ПолучитьПланыВидовРасчетаORM` | функция | — | [bsl-orm/src/managers.rs](../../../crates/bsl-orm/src/managers.rs) |
| `ПолучитьБизнесПроцессыORM` | функция | — | [bsl-orm/src/managers.rs](../../../crates/bsl-orm/src/managers.rs) |
| `ПолучитьЗадачиORM` | функция | — | [bsl-orm/src/managers.rs](../../../crates/bsl-orm/src/managers.rs) |
| `ПолучитьХранилищаНастроекORM` | функция | — | [bsl-orm/src/managers.rs](../../../crates/bsl-orm/src/managers.rs) |
| `ПолучитьПоследовательностиORM` | функция | — | [bsl-orm/src/managers.rs](../../../crates/bsl-orm/src/managers.rs) |
| `ПолучитьПолноеИмяПредопределенногоЗначения`, `GetPredefinedValueFullName` | функция | — | [bsl-orm/src/globals.rs](../../../crates/bsl-orm/src/globals.rs) |
| `ПолучитьНавигационнуюСсылку`, `GetURL` | функция | Строит навигационную ссылку для прикладного значения текущего сеанса. | [bsl-orm/src/data_access.rs](../../../crates/bsl-orm/src/data_access.rs) |
| `ПолучитьПолитикиПаролейПользователейORM` | функция | — | [bsl-orm/src/infobase_users.rs](../../../crates/bsl-orm/src/infobase_users.rs) |
| `ПолучитьМинимальнуюДлинуПаролейПользователей`, `GetUserPasswordMinLength` | функция | — | [bsl-orm/src/infobase_users.rs](../../../crates/bsl-orm/src/infobase_users.rs) |
| `ПолучитьБлокировкуАутентификацииORM` | функция | — | [bsl-orm/src/infobase_users.rs](../../../crates/bsl-orm/src/infobase_users.rs) |
| `ПолучитьПроверкуСложностиПаролейПользователей`, `GetUserPasswordStrengthCheck` | функция | — | [bsl-orm/src/infobase_users.rs](../../../crates/bsl-orm/src/infobase_users.rs) |
| `ПолучитьМаксимальныйСрокДействияПаролейПользователей`, `GetUserPasswordMaxEffectivePeriod` | функция | — | [bsl-orm/src/infobase_users.rs](../../../crates/bsl-orm/src/infobase_users.rs) |
| `ПолучитьМинимальныйСрокДействияПаролейПользователей`, `GetUserPasswordMinEffectivePeriod` | функция | — | [bsl-orm/src/infobase_users.rs](../../../crates/bsl-orm/src/infobase_users.rs) |
| `ПолучитьОграничениеПовторенияПаролейПользователейСредиПоследних`, `GetUserPasswordReuseLimit` | функция | — | [bsl-orm/src/infobase_users.rs](../../../crates/bsl-orm/src/infobase_users.rs) |
| `ПолучитьСрокПредупрежденияОбИстеченииСрокаДействияПаролейПользователей`, `GetUserPasswordExpirationNotificationPeriod` | функция | — | [bsl-orm/src/infobase_users.rs](../../../crates/bsl-orm/src/infobase_users.rs) |
| `УстановитьПроверкуСложностиПаролейПользователей`, `SetUserPasswordStrengthCheck` | процедура | — | [bsl-orm/src/infobase_users.rs](../../../crates/bsl-orm/src/infobase_users.rs) |
| `УстановитьМинимальнуюДлинуПаролейПользователей`, `SetUserPasswordMinLength` | процедура | — | [bsl-orm/src/infobase_users.rs](../../../crates/bsl-orm/src/infobase_users.rs) |
| `УстановитьМаксимальныйСрокДействияПаролейПользователей`, `SetUserPasswordMaxEffectivePeriod` | процедура | — | [bsl-orm/src/infobase_users.rs](../../../crates/bsl-orm/src/infobase_users.rs) |
| `УстановитьМинимальныйСрокДействияПаролейПользователей`, `SetUserPasswordMinEffectivePeriod` | процедура | — | [bsl-orm/src/infobase_users.rs](../../../crates/bsl-orm/src/infobase_users.rs) |
| `УстановитьОграничениеПовторенияПаролейПользователейСредиПоследних`, `SetUserPasswordReuseLimit` | процедура | — | [bsl-orm/src/infobase_users.rs](../../../crates/bsl-orm/src/infobase_users.rs) |
| `УстановитьСрокПредупрежденияОбИстеченииСрокаДействияПаролейПользователей`, `SetUserPasswordExpirationNotificationPeriod` | процедура | — | [bsl-orm/src/infobase_users.rs](../../../crates/bsl-orm/src/infobase_users.rs) |
| `ПолучитьОбщийМакет`, `GetCommonTemplate` | функция | — | [bsl-orm/src/globals.rs](../../../crates/bsl-orm/src/globals.rs) |
| `ВыполнитьПроверкуПравДоступа`, `VerifyAccessRights` | процедура | — | [bsl-orm/src/access_rights.rs](../../../crates/bsl-orm/src/access_rights.rs) |
| `ПолучитьВнешниеОбработкиORM` | функция | — | [bsl-orm/src/globals.rs](../../../crates/bsl-orm/src/globals.rs) |
| `ПолучитьВнешниеОтчетыORM` | функция | — | [bsl-orm/src/globals.rs](../../../crates/bsl-orm/src/globals.rs) |
| `ПолучитьИсториюДанныхORM` | функция | — | [bsl-orm/src/globals.rs](../../../crates/bsl-orm/src/globals.rs) |
| `ПолучитьШаблоныНастроекВторогоФактораАутентификацииORM` | функция | — | [bsl-orm/src/globals.rs](../../../crates/bsl-orm/src/globals.rs) |

#### Конструкторы

| Имя и псевдонимы | Описание | Реализация |
|---|---|---|
| `МоментВремениСУточнениемПериода`, `PointInTimeWithPeriodAdjustment` | — | [bsl-orm/src/unavailable_constructors.rs](../../../crates/bsl-orm/src/unavailable_constructors.rs) |
| `НастройкаВторогоФактораАутентификации`, `SecondAuthenticationFactorSetting` | — | [bsl-orm/src/query_boundary.rs](../../../crates/bsl-orm/src/query_boundary.rs) |
| `ОписаниеОбновленияКонфигурации`, `ConfigurationUpdateDescription` | — | [bsl-orm/src/unavailable_constructors.rs](../../../crates/bsl-orm/src/unavailable_constructors.rs) |
| `ОбработкаРасшифровкиКомпоновкиДанных`, `DataCompositionDetailsProcess` | — | [bsl-orm/src/data_composition.rs](../../../crates/bsl-orm/src/data_composition.rs) |
| `ПостроительЗапроса`, `QueryBuilder` | — | [bsl-orm/src/unavailable_constructors.rs](../../../crates/bsl-orm/src/unavailable_constructors.rs) |
| `ОбъектМетаданныхКонфигурация`, `ConfigurationMetadataObject` | — | [bsl-orm/src/unavailable_constructors.rs](../../../crates/bsl-orm/src/unavailable_constructors.rs) |
| `УдалениеОбъекта`, `ObjectDeletion` | — | [bsl-orm/src/unavailable_constructors.rs](../../../crates/bsl-orm/src/unavailable_constructors.rs) |
| `ВыражениеСхемыЗапроса`, `QuerySchemaExpression` | — | [bsl-orm/src/unavailable_constructors.rs](../../../crates/bsl-orm/src/unavailable_constructors.rs) |
| `ПользовательскиеНастройкиКомпоновкиДанных`, `DataCompositionUserSettings` | — | [bsl-orm/src/data_composition.rs](../../../crates/bsl-orm/src/data_composition.rs) |
| `Граница`, `Boundary` | — | [bsl-orm/src/query_boundary.rs](../../../crates/bsl-orm/src/query_boundary.rs) |
| `ПроцессорВыводаРезультатаКомпоновкиДанныхВТабличныйДокумент`, `DataCompositionResultSpreadsheetDocumentOutputProcessor` | — | [bsl-orm/src/data_composition.rs](../../../crates/bsl-orm/src/data_composition.rs) |
| `СхемаКомпоновкиДанных`, `DataCompositionSchema` | — | [bsl-orm/src/data_composition.rs](../../../crates/bsl-orm/src/data_composition.rs) |
| `СтандартныйПериод`, `StandardPeriod` | — | [bsl-orm/src/unavailable_constructors.rs](../../../crates/bsl-orm/src/unavailable_constructors.rs) |
| `ОписаниеКонфигурации`, `ConfigurationDescription` | — | [bsl-orm/src/unavailable_constructors.rs](../../../crates/bsl-orm/src/unavailable_constructors.rs) |
| `ОписаниеИсточникаДанных`, `DataSourceDescription` | — | [bsl-orm/src/unavailable_constructors.rs](../../../crates/bsl-orm/src/unavailable_constructors.rs) |
| `ПостроительОтчета`, `ReportBuilder` | — | [bsl-orm/src/unavailable_constructors.rs](../../../crates/bsl-orm/src/unavailable_constructors.rs) |
| `Запрос`, `Query` | — | [bsl-orm/src/query.rs](../../../crates/bsl-orm/src/query.rs) |
| `СжатиеДанных`, `Deflation` | — | [bsl-orm/src/value_storage.rs](../../../crates/bsl-orm/src/value_storage.rs) |
| `ХранилищеЗначения`, `ValueStorage` | — | [bsl-orm/src/value_storage.rs](../../../crates/bsl-orm/src/value_storage.rs) |
| `БлокировкаДанных`, `DataLock` | — | [bsl-orm/src/infobase.rs](../../../crates/bsl-orm/src/infobase.rs) |
| `МенеджерВременныхТаблиц`, `TempTablesManager` | — | [bsl-orm/src/query.rs](../../../crates/bsl-orm/src/query.rs) |
| `СхемаЗапроса`, `QuerySchema` | — | [bsl-orm/src/unavailable_constructors.rs](../../../crates/bsl-orm/src/unavailable_constructors.rs) |
| `ГенераторСлучайныхПаролей`, `RandomPasswordGenerator` | — | [bsl-orm/src/infobase_users.rs](../../../crates/bsl-orm/src/infobase_users.rs) |
| `КомпоновщикМакетаКомпоновкиДанных`, `DataCompositionTemplateComposer` | — | [bsl-orm/src/data_composition.rs](../../../crates/bsl-orm/src/data_composition.rs) |
| `ПроцессорКомпоновкиДанных`, `DataCompositionProcessor` | — | [bsl-orm/src/data_composition.rs](../../../crates/bsl-orm/src/data_composition.rs) |
| `ПроцессорВыводаРезультатаКомпоновкиДанныхВКоллекциюЗначений`, `DataCompositionResultValueCollectionOutputProcessor` | — | [bsl-orm/src/data_composition.rs](../../../crates/bsl-orm/src/data_composition.rs) |
| `КомпоновщикНастроекКомпоновкиДанных`, `DataCompositionSettingsComposer` | — | [bsl-orm/src/data_composition.rs](../../../crates/bsl-orm/src/data_composition.rs) |
| `ИсточникДоступныхНастроекКомпоновкиДанных`, `DataCompositionAvailableSettingsSource` | — | [bsl-orm/src/data_composition.rs](../../../crates/bsl-orm/src/data_composition.rs) |

#### Типы объектов

| Имя | Представление типа | Дополнительные имена | Описание | Реализация |
|---|---|---|---|---|
| `Граница` | `Граница` | `Boundary` | — | [bsl-orm/src/query_boundary.rs](../../../crates/bsl-orm/src/query_boundary.rs) |
| `НастройкаВторогоФактораАутентификации` | `НастройкаВторогоФактораАутентификации` | `SecondAuthenticationFactorSetting` | — | [bsl-orm/src/second_factor_setting.rs](../../../crates/bsl-orm/src/second_factor_setting.rs) |
| `ОбъектМетаданныхКонфигурация` | `ОбъектМетаданныхКонфигурация` | — | — | [bsl-orm/src/metadata_values.rs](../../../crates/bsl-orm/src/metadata_values.rs) |
| `КоллекцияОбъектовМетаданных` | `КоллекцияОбъектовМетаданных` | — | — | [bsl-orm/src/metadata_values.rs](../../../crates/bsl-orm/src/metadata_values.rs) |
| `ОбъектМетаданныхКонфигурации` | `Объект метаданных` | `ОбъектМетаданных`, `MetadataObject` | — | [bsl-orm/src/metadata_values.rs](../../../crates/bsl-orm/src/metadata_values.rs) |
| `КоллекцияЗначенийСвойстваОбъектаМетаданных` | `КоллекцияЗначенийСвойстваОбъектаМетаданных` | — | — | [bsl-orm/src/metadata_values.rs](../../../crates/bsl-orm/src/metadata_values.rs) |
| `ПеречислимыеСвойстваОбъектовМетаданных` | `ПеречислимыеСвойстваОбъектовМетаданных` | — | — | [bsl-orm/src/metadata_values.rs](../../../crates/bsl-orm/src/metadata_values.rs) |
| `Metadata object enumerated property` | `Перечислимое свойство объектов метаданных` | — | — | [bsl-orm/src/metadata_values.rs](../../../crates/bsl-orm/src/metadata_values.rs) |
| `МенеджерыОбъектов` | `МенеджерыОбъектов` | — | — | [bsl-orm/src/managers.rs](../../../crates/bsl-orm/src/managers.rs) |
| `МенеджерОбъекта` | `МенеджерОбъекта` | — | — | [bsl-orm/src/managers.rs](../../../crates/bsl-orm/src/managers.rs) |
| `ПеречислениеСсылка` | `Enumeration reference` | `EnumerationRef` | — | [bsl-orm/src/references.rs](../../../crates/bsl-orm/src/references.rs) |
| `ТочкиМаршрутаБизнесПроцесса` | `Точки маршрута бизнес-процесса` | `BusinessProcessRoutePoints` | — | [bsl-orm/src/route_points.rs](../../../crates/bsl-orm/src/route_points.rs) |
| `ТочкаМаршрутаБизнесПроцессаСсылка` | `Business process route point reference` | `BusinessProcessRoutePointRef` | — | [bsl-orm/src/route_points.rs](../../../crates/bsl-orm/src/route_points.rs) |
| `СсылкаКонфигурации` | `СсылкаКонфигурации` | — | — | [bsl-orm/src/references.rs](../../../crates/bsl-orm/src/references.rs) |
| `ОбъектСправочникаКонфигурации` | `ОбъектСправочникаКонфигурации` | — | — | [bsl-orm/src/catalog_object.rs](../../../crates/bsl-orm/src/catalog_object.rs) |
| `ОбъектПланаВидовХарактеристикКонфигурации` | `ОбъектПланаВидовХарактеристикКонфигурации` | — | — | [bsl-orm/src/characteristic_object.rs](../../../crates/bsl-orm/src/characteristic_object.rs) |
| `ДокументОбъект` | `Документ объект` | `DocumentObject` | — | [bsl-orm/src/document_object.rs](../../../crates/bsl-orm/src/document_object.rs) |
| `БизнесПроцессОбъект` | `БизнесПроцессОбъект` | `BusinessProcessObject` | — | [bsl-orm/src/business_process_object.rs](../../../crates/bsl-orm/src/business_process_object.rs) |
| `ОбработкаОбъект` | `Обработка объект` | `DataProcessorObject` | — | [bsl-orm/src/data_processor_object.rs](../../../crates/bsl-orm/src/data_processor_object.rs) |
| `ОтчетОбъект` | `Отчет объект` | `ReportObject` | — | [bsl-orm/src/data_processor_object.rs](../../../crates/bsl-orm/src/data_processor_object.rs) |
| `КоллекцияДвижений` | `Коллекция движений` | `RegisterRecordsCollection` | — | [bsl-orm/src/document_movements.rs](../../../crates/bsl-orm/src/document_movements.rs) |
| `РегистрСведенийНаборЗаписей` | `Регистр сведений набор записей` | `InformationRegisterRecordSet` | — | [bsl-orm/src/document_movements.rs](../../../crates/bsl-orm/src/document_movements.rs) |
| `ДокументТабличнаяЧасть` | `Документ табличная часть` | `DocumentTabularSection` | — | [bsl-orm/src/document_tabular_section.rs](../../../crates/bsl-orm/src/document_tabular_section.rs) |
| `ДокументТабличнаяЧастьСтрока` | `Документ табличная часть строка` | `DocumentTabularSectionRow` | — | [bsl-orm/src/document_tabular_section.rs](../../../crates/bsl-orm/src/document_tabular_section.rs) |
| `ПараметрыОбменаДанными` | `Параметры обмена данными` | `DataExchangeParameters` | — | [bsl-orm/src/catalog_object.rs](../../../crates/bsl-orm/src/catalog_object.rs) |
| `НаборУзлов` | `Коллекция узлов` | `NodeSet` | — | [bsl-orm/src/catalog_object.rs](../../../crates/bsl-orm/src/catalog_object.rs) |
| `БиблиотекаКартинок` | `БиблиотекаКартинок` | `PictureLib` | — | [bsl-orm/src/picture_library.rs](../../../crates/bsl-orm/src/picture_library.rs) |
| `МенеджерРасширенийКонфигурации` | `МенеджерРасширенийКонфигурации` | — | — | [bsl-orm/src/configuration_extension.rs](../../../crates/bsl-orm/src/configuration_extension.rs) |
| `МенеджерПользователейИнформационнойБазы` | `Менеджер пользователей` | `InfoBaseUsersManager` | — | [bsl-orm/src/infobase_users.rs](../../../crates/bsl-orm/src/infobase_users.rs) |
| `ПользовательИнформационнойБазы` | `Пользователь информационной базы` | `InfoBaseUser` | — | [bsl-orm/src/infobase_users.rs](../../../crates/bsl-orm/src/infobase_users.rs) |
| `МенеджерПолитикПаролейПользователей` | `Менеджер политик паролей пользователей` | `UserPasswordPolicyManager` | — | [bsl-orm/src/infobase_users.rs](../../../crates/bsl-orm/src/infobase_users.rs) |
| `ПолитикаПаролейПользователей` | `Политика паролей пользователей` | `UserPasswordPolicy` | — | [bsl-orm/src/infobase_users.rs](../../../crates/bsl-orm/src/infobase_users.rs) |
| `МенеджерБлокировкиАутентификацииПользователейИнформационнойБазы` | `Менеджер блокировки аутентификации пользователей информационной базы` | `InfoBaseUserAuthenticationLockManager` | — | [bsl-orm/src/infobase_users.rs](../../../crates/bsl-orm/src/infobase_users.rs) |
| `НастройкиБлокировкиАутентификацииПользователейИнформационнойБазы` | `Настройки блокировки аутентификации пользователей информационной базы` | `InfoBaseUserAuthenticationLockSettings` | — | [bsl-orm/src/infobase_users.rs](../../../crates/bsl-orm/src/infobase_users.rs) |
| `ГенераторСлучайныхПаролей` | `ГенераторСлучайныхПаролей` | `RandomPasswordGenerator` | — | [bsl-orm/src/infobase_users.rs](../../../crates/bsl-orm/src/infobase_users.rs) |
| `МенеджерСервисовИнтеграции` | `Integration services manager` | `IntegrationServicesManager` | — | [bsl-orm/src/infobase.rs](../../../crates/bsl-orm/src/infobase.rs) |
| `РасширениеКонфигурации` | `РасширениеКонфигурации` | — | — | [bsl-orm/src/configuration_extension.rs](../../../crates/bsl-orm/src/configuration_extension.rs) |
| `ИнформацияОПроблемеПримененияРасширенияКонфигурации` | `ИнформацияОПроблемеПримененияРасширенияКонфигурации` | — | — | [bsl-orm/src/configuration_extension.rs](../../../crates/bsl-orm/src/configuration_extension.rs) |
| `Запрос` | `Запрос` | `Query` | — | [bsl-orm/src/query.rs](../../../crates/bsl-orm/src/query.rs) |
| `СхемаЗапроса` | `Схема запроса` | `QuerySchema` | — | [bsl-orm/src/unavailable_constructors.rs](../../../crates/bsl-orm/src/unavailable_constructors.rs) |
| `КомпоновщикНастроекКомпоновкиДанных` | `Компоновщик настроек компоновки данных` | `DataCompositionSettingsComposer` | — | [bsl-orm/src/data_composition.rs](../../../crates/bsl-orm/src/data_composition.rs) |
| `ИсточникДоступныхНастроекКомпоновкиДанных` | `Источник доступных настроек компоновки данных` | `DataCompositionAvailableSettingsSource` | — | [bsl-orm/src/data_composition.rs](../../../crates/bsl-orm/src/data_composition.rs) |
| `ОбработкаРасшифровкиКомпоновкиДанных` | `Обработка расшифровки компноновки данных` | `DataCompositionDetailsProcess` | — | [bsl-orm/src/data_composition.rs](../../../crates/bsl-orm/src/data_composition.rs) |
| `ОписаниеОбновленияКонфигурации` | `Описание обновления конфигурации` | `ConfigurationUpdateDescription` | — | [bsl-orm/src/unavailable_constructors.rs](../../../crates/bsl-orm/src/unavailable_constructors.rs) |
| `КомпоновщикМакетаКомпоновкиДанных` | `Компоновщик макета компоновки данных` | `DataCompositionTemplateComposer` | — | [bsl-orm/src/data_composition.rs](../../../crates/bsl-orm/src/data_composition.rs) |
| `ПроцессорКомпоновкиДанных` | `Процессор компоновки данных` | `DataCompositionProcessor` | — | [bsl-orm/src/data_composition.rs](../../../crates/bsl-orm/src/data_composition.rs) |
| `ПроцессорВыводаРезультатаКомпоновкиДанныхВКоллекциюЗначений` | `Процессор вывода результата компоновки данных в коллекцию значений` | `DataCompositionResultValueCollectionOutputProcessor` | — | [bsl-orm/src/data_composition.rs](../../../crates/bsl-orm/src/data_composition.rs) |
| `МенеджерВременныхТаблиц` | `Менеджер временных таблиц` | `TempTablesManager` | — | [bsl-orm/src/query.rs](../../../crates/bsl-orm/src/query.rs) |
| `РезультатЗапроса` | `Результат запроса` | `QueryResult` | — | [bsl-orm/src/query.rs](../../../crates/bsl-orm/src/query.rs) |
| `ВыборкаИзРезультатаЗапроса` | `Выборка из результата запроса` | `QueryResultSelection` | — | [bsl-orm/src/query.rs](../../../crates/bsl-orm/src/query.rs) |
| `СправочникВыборка` | `Catalog selection` | `CatalogSelection` | — | [bsl-orm/src/catalog_selection.rs](../../../crates/bsl-orm/src/catalog_selection.rs) |
| `СжатиеДанных` | `Сжатие данных` | `Deflation` | — | [bsl-orm/src/value_storage.rs](../../../crates/bsl-orm/src/value_storage.rs) |
| `ХранилищеЗначения` | `Хранилище значения` | `ValueStorage` | — | [bsl-orm/src/value_storage.rs](../../../crates/bsl-orm/src/value_storage.rs) |
| `БлокировкаДанных` | `Блокировка данных` | `DataLock` | — | [bsl-orm/src/infobase.rs](../../../crates/bsl-orm/src/infobase.rs) |
| `ЭлементБлокировкиДанных` | `Элемент блокировки данных` | `DataLockItem` | — | [bsl-orm/src/infobase.rs](../../../crates/bsl-orm/src/infobase.rs) |
| `СоставОбщегоРеквизита` | `Состав общего реквизита` | `CommonAttributeContent` | — | [bsl-orm/src/common_attribute_content.rs](../../../crates/bsl-orm/src/common_attribute_content.rs) |
| `ЭлементСоставаОбщегоРеквизита` | `Элемент состава общего реквизита` | `CommonAttributeContentItem` | — | [bsl-orm/src/common_attribute_content.rs](../../../crates/bsl-orm/src/common_attribute_content.rs) |
| `МоментВремениСУточнениемПериода` | `Момент времени с уточнением периода` | `PointInTimeWithPeriodAdjustment` | — | [bsl-orm/src/unavailable_constructors.rs](../../../crates/bsl-orm/src/unavailable_constructors.rs) |

#### Члены объектов

##### `Граница`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `Значение`, `Value` | только чтение | — | [bsl-orm/src/query_boundary.rs](../../../crates/bsl-orm/src/query_boundary.rs) |
| `ВидГраницы`, `BoundaryType` | только чтение | — | [bsl-orm/src/query_boundary.rs](../../../crates/bsl-orm/src/query_boundary.rs) |

##### `НастройкаВторогоФактораАутентификации`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `ИмяШаблонаНастройки`, `SettingTemplateName` | чтение и запись | — | [bsl-orm/src/second_factor_setting.rs](../../../crates/bsl-orm/src/second_factor_setting.rs) |
| `Параметры`, `Parameters` | чтение и запись | — | [bsl-orm/src/second_factor_setting.rs](../../../crates/bsl-orm/src/second_factor_setting.rs) |

##### `ОбъектМетаданныхКонфигурация`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `ОбщиеФормы`, `CommonForms` | только чтение | — | [bsl-orm/src/file_source.rs](../../../crates/bsl-orm/src/file_source.rs) |
| `ОбщиеМакеты`, `CommonTemplates` | только чтение | — | [bsl-orm/src/file_source.rs](../../../crates/bsl-orm/src/file_source.rs) |
| `ОсновнойИнтерфейс`, `DefaultInterface` | только чтение | — | [bsl-orm/src/metadata_values.rs](../../../crates/bsl-orm/src/metadata_values.rs) |
| `ОбщиеРеквизиты`, `CommonAttributes` | только чтение | — | [bsl-orm/src/metadata_values.rs](../../../crates/bsl-orm/src/metadata_values.rs) |
| `ОсновнойЯзык`, `DefaultLanguage` | только чтение | — | [bsl-orm/src/metadata_values.rs](../../../crates/bsl-orm/src/metadata_values.rs) |
| `Подсистемы`, `Subsystems` | только чтение | — | [bsl-orm/src/metadata_values.rs](../../../crates/bsl-orm/src/metadata_values.rs) |
| `Имя`, `Name` | только чтение | — | [bsl-orm/src/metadata_values.rs](../../../crates/bsl-orm/src/metadata_values.rs) |
| `Версия`, `Version` | только чтение | — | [bsl-orm/src/metadata_values.rs](../../../crates/bsl-orm/src/metadata_values.rs) |
| `ВариантВстроенногоЯзыка`, `ScriptVariant` | только чтение | — | [bsl-orm/src/metadata_values.rs](../../../crates/bsl-orm/src/metadata_values.rs) |
| `РежимСовместимости`, `CompatibilityMode` | только чтение | — | [bsl-orm/src/metadata_values.rs](../../../crates/bsl-orm/src/metadata_values.rs) |
| `ОсновныеРоли`, `DefaultRoles` | только чтение | — | [bsl-orm/src/metadata_values.rs](../../../crates/bsl-orm/src/metadata_values.rs) |
| `СвойстваОбъектов`, `ObjectProperties` | только чтение | — | [bsl-orm/src/metadata_values.rs](../../../crates/bsl-orm/src/metadata_values.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `НайтиПоТипу`, `FindByType` | — | [bsl-orm/src/metadata_find_by_type.rs](../../../crates/bsl-orm/src/metadata_find_by_type.rs) |

##### `КоллекцияОбъектовМетаданных`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

| Метод | Описание | Реализация |
|---|---|---|
| `Получить`, `Get` | — | [bsl-orm/src/metadata_values.rs](../../../crates/bsl-orm/src/metadata_values.rs) |
| `Найти`, `Find` | — | [bsl-orm/src/metadata_values.rs](../../../crates/bsl-orm/src/metadata_values.rs) |
| `Количество`, `Count` | — | [bsl-orm/src/metadata_values.rs](../../../crates/bsl-orm/src/metadata_values.rs) |

##### `ОбъектМетаданныхКонфигурации`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `РазделениеДанных`, `DataSeparation` | только чтение | — | [bsl-orm/src/metadata_values.rs](../../../crates/bsl-orm/src/metadata_values.rs) |
| `АвтоИспользование`, `AutoUse` | только чтение | — | [bsl-orm/src/metadata_values.rs](../../../crates/bsl-orm/src/metadata_values.rs) |
| `УсловноеРазделение`, `ConditionalSeparation` | только чтение | — | [bsl-orm/src/common_attribute_content.rs](../../../crates/bsl-orm/src/common_attribute_content.rs) |
| `Состав`, `Content` | только чтение | — | [bsl-orm/src/common_attribute_content.rs](../../../crates/bsl-orm/src/common_attribute_content.rs) |
| `Реквизиты`, `Attributes` | только чтение | — | [bsl-orm/src/metadata_values.rs](../../../crates/bsl-orm/src/metadata_values.rs) |
| `ЗаполнятьИзДанныхЗаполнения`, `FillFromFillingValue` | только чтение | — | [bsl-orm/src/metadata_attributes.rs](../../../crates/bsl-orm/src/metadata_attributes.rs) |
| `ЗначениеЗаполнения`, `FillValue` | только чтение | — | [bsl-orm/src/metadata_attributes.rs](../../../crates/bsl-orm/src/metadata_attributes.rs) |
| `КодЯзыка`, `LanguageCode` | только чтение | — | [bsl-orm/src/metadata_values.rs](../../../crates/bsl-orm/src/metadata_values.rs) |
| `Имя`, `Name` | только чтение | — | [bsl-orm/src/metadata_values.rs](../../../crates/bsl-orm/src/metadata_values.rs) |
| `Подсистемы`, `Subsystems` | только чтение | — | [bsl-orm/src/metadata_values.rs](../../../crates/bsl-orm/src/metadata_values.rs) |
| `ВключатьВКомандныйИнтерфейс`, `IncludeInCommandInterface` | только чтение | — | [bsl-orm/src/metadata_values.rs](../../../crates/bsl-orm/src/metadata_values.rs) |
| `Формы`, `Forms` | только чтение | — | [bsl-orm/src/file_source.rs](../../../crates/bsl-orm/src/file_source.rs) |
| `Макеты`, `Templates` | только чтение | — | [bsl-orm/src/file_source.rs](../../../crates/bsl-orm/src/file_source.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `ПолноеИмя`, `FullName` | — | [bsl-orm/src/metadata_values.rs](../../../crates/bsl-orm/src/metadata_values.rs) |

##### `КоллекцияЗначенийСвойстваОбъектаМетаданных`

| Метод | Описание | Реализация |
|---|---|---|
| `Содержит`, `Contains` | — | [bsl-orm/src/metadata_values.rs](../../../crates/bsl-orm/src/metadata_values.rs) |

##### `ПеречислимыеСвойстваОбъектовМетаданных`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `НазначениеРасширенияКонфигурации`, `ConfigurationExtensionPurpose` | только чтение | — | [bsl-orm/src/metadata_values.rs](../../../crates/bsl-orm/src/metadata_values.rs) |
| `РазделениеДанныхОбщегоРеквизита`, `CommonAttributeDataSeparation` | только чтение | — | [bsl-orm/src/metadata_values.rs](../../../crates/bsl-orm/src/metadata_values.rs) |
| `АвтоИспользованиеОбщегоРеквизита`, `CommonAttributeAutoUse` | только чтение | — | [bsl-orm/src/metadata_values.rs](../../../crates/bsl-orm/src/metadata_values.rs) |
| `ИспользованиеОбщегоРеквизита`, `CommonAttributeUse` | только чтение | — | [bsl-orm/src/metadata_values.rs](../../../crates/bsl-orm/src/metadata_values.rs) |
| `ВариантВстроенногоЯзыка`, `ScriptVariant` | только чтение | — | [bsl-orm/src/metadata_values.rs](../../../crates/bsl-orm/src/metadata_values.rs) |
| `РежимСовместимости`, `CompatibilityMode` | только чтение | — | [bsl-orm/src/metadata_values.rs](../../../crates/bsl-orm/src/metadata_values.rs) |

##### `МенеджерыОбъектов`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

##### `МенеджерОбъекта`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `Имя`, `Name` | только чтение | — | [bsl-orm/src/managers.rs](../../../crates/bsl-orm/src/managers.rs) |
| `ТочкиМаршрута`, `RoutePoints` | только чтение | — | [bsl-orm/src/route_points.rs](../../../crates/bsl-orm/src/route_points.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `СоздатьУзел`, `CreateNode` | — | [bsl-orm/src/exchange_plan.rs](../../../crates/bsl-orm/src/exchange_plan.rs) |
| `ЭтотУзел`, `ThisNode` | — | [bsl-orm/src/exchange_plan.rs](../../../crates/bsl-orm/src/exchange_plan.rs) |
| `ПолучитьМакет`, `GetTemplate` | — | [bsl-orm/src/managers.rs](../../../crates/bsl-orm/src/managers.rs) |
| `Загрузить`, `Load` | — | [bsl-session/src/client_settings.rs](../../../crates/bsl-session/src/client_settings.rs) |
| `СоздатьБизнесПроцесс`, `CreateBusinessProcess` | — | [bsl-orm/src/business_process_object.rs](../../../crates/bsl-orm/src/business_process_object.rs) |
| `Остатки`, `Balance` | — | [bsl-orm/src/accumulation_balance.rs](../../../crates/bsl-orm/src/accumulation_balance.rs) |
| `Получить`, `Get` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Установить`, `Set` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `СоздатьМенеджерЗначения`, `CreateValueManager` | — | [bsl-orm/src/managers.rs](../../../crates/bsl-orm/src/managers.rs) |
| `Количество`, `Count` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Получить`, `Get` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Индекс`, `IndexOf` | — | [bsl-orm/src/managers.rs](../../../crates/bsl-orm/src/managers.rs) |
| `ПустаяСсылка`, `EmptyRef` | — | [bsl-orm/src/managers.rs](../../../crates/bsl-orm/src/managers.rs) |
| `ПолучитьСсылку`, `GetRef` | — | [bsl-orm/src/managers.rs](../../../crates/bsl-orm/src/managers.rs) |
| `СоздатьЭлемент`, `CreateItem` | — | [bsl-orm/src/managers.rs](../../../crates/bsl-orm/src/managers.rs) |
| `СоздатьГруппу`, `CreateFolder` | — | [bsl-orm/src/managers.rs](../../../crates/bsl-orm/src/managers.rs) |
| `Выбрать`, `Select` | — | [bsl-orm/src/managers.rs](../../../crates/bsl-orm/src/managers.rs) |
| `НайтиПоКоду`, `FindByCode` | — | [bsl-orm/src/managers.rs](../../../crates/bsl-orm/src/managers.rs) |
| `НайтиПоНаименованию`, `FindByDescription` | — | [bsl-orm/src/managers.rs](../../../crates/bsl-orm/src/managers.rs) |
| `СоздатьМенеджерЗаписи`, `CreateRecordManager` | — | [bsl-orm/src/managers.rs](../../../crates/bsl-orm/src/managers.rs) |
| `СоздатьНаборЗаписей`, `CreateRecordSet` | — | [bsl-orm/src/managers.rs](../../../crates/bsl-orm/src/managers.rs) |
| `ПолучитьМакет`, `GetTemplate` | — | [bsl-orm/src/managers.rs](../../../crates/bsl-orm/src/managers.rs) |
| `СоздатьКлючЗаписи`, `CreateRecordKey` | — | [bsl-orm/src/managers.rs](../../../crates/bsl-orm/src/managers.rs) |
| `Получить`, `Get` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `СрезПервых`, `SliceFirst` | — | [bsl-orm/src/managers.rs](../../../crates/bsl-orm/src/managers.rs) |
| `СрезПоследних`, `SliceLast` | — | [bsl-orm/src/managers.rs](../../../crates/bsl-orm/src/managers.rs) |
| `Выбрать`, `Select` | — | [bsl-orm/src/managers.rs](../../../crates/bsl-orm/src/managers.rs) |
| `ПолучитьПоследнее`, `GetLast` | — | [bsl-orm/src/managers.rs](../../../crates/bsl-orm/src/managers.rs) |
| `ПолучитьМакет`, `GetTemplate` | — | [bsl-orm/src/managers.rs](../../../crates/bsl-orm/src/managers.rs) |
| `Создать`, `Create` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `НайтиПоРеквизиту`, `FindByAttribute` | — | [bsl-orm/src/managers.rs](../../../crates/bsl-orm/src/managers.rs) |
| `СоздатьДокумент`, `CreateDocument` | — | [bsl-orm/src/managers.rs](../../../crates/bsl-orm/src/managers.rs) |
| `ПолучитьМакет`, `GetTemplate` | — | [bsl-orm/src/managers.rs](../../../crates/bsl-orm/src/managers.rs) |
| `Создать`, `Create` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |

##### `ПеречислениеСсылка`

| Метод | Описание | Реализация |
|---|---|---|
| `Пустая`, `IsEmpty` | — | [bsl-orm/src/references.rs](../../../crates/bsl-orm/src/references.rs) |
| `Метаданные`, `Metadata` | — | [bsl-orm/src/references.rs](../../../crates/bsl-orm/src/references.rs) |

##### `ТочкиМаршрутаБизнесПроцесса`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

| Метод | Описание | Реализация |
|---|---|---|
| `Количество`, `Count` | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |

##### `СсылкаКонфигурации`

| Метод | Описание | Реализация |
|---|---|---|
| `Пустая`, `IsEmpty` | — | [bsl-orm/src/references.rs](../../../crates/bsl-orm/src/references.rs) |
| `УникальныйИдентификатор`, `UUID` | — | [bsl-orm/src/references.rs](../../../crates/bsl-orm/src/references.rs) |
| `Метаданные`, `Metadata` | — | [bsl-orm/src/references.rs](../../../crates/bsl-orm/src/references.rs) |

##### `ОбъектСправочникаКонфигурации`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `Предопределенный`, `Predefined` | только чтение | — | [bsl-orm/src/catalog_object.rs](../../../crates/bsl-orm/src/catalog_object.rs) |
| `ИмяПредопределенныхДанных`, `PredefinedDataName` | чтение и запись | — | [bsl-orm/src/catalog_object.rs](../../../crates/bsl-orm/src/catalog_object.rs) |
| `ПометкаУдаления`, `DeletionMark` | чтение и запись | — | [bsl-orm/src/catalog_object.rs](../../../crates/bsl-orm/src/catalog_object.rs) |
| `ЭтоГруппа`, `IsFolder` | только чтение | — | [bsl-orm/src/catalog_object.rs](../../../crates/bsl-orm/src/catalog_object.rs) |
| `Родитель`, `Parent` | чтение и запись | — | [bsl-orm/src/catalog_object.rs](../../../crates/bsl-orm/src/catalog_object.rs) |
| `Владелец`, `Owner` | чтение и запись | — | [bsl-orm/src/catalog_object.rs](../../../crates/bsl-orm/src/catalog_object.rs) |
| `Ссылка`, `Ref` | только чтение | — | [bsl-orm/src/catalog_object.rs](../../../crates/bsl-orm/src/catalog_object.rs) |
| `Код`, `Code` | чтение и запись | — | [bsl-orm/src/catalog_object.rs](../../../crates/bsl-orm/src/catalog_object.rs) |
| `Наименование`, `Description` | чтение и запись | — | [bsl-orm/src/catalog_object.rs](../../../crates/bsl-orm/src/catalog_object.rs) |
| `ДополнительныеСвойства`, `AdditionalProperties` | только чтение | — | [bsl-orm/src/catalog_object.rs](../../../crates/bsl-orm/src/catalog_object.rs) |
| `ОбменДанными`, `DataExchange` | только чтение | — | [bsl-orm/src/catalog_object.rs](../../../crates/bsl-orm/src/catalog_object.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `УстановитьНовыйКод`, `SetNewCode` | — | [bsl-orm/src/catalog_object.rs](../../../crates/bsl-orm/src/catalog_object.rs) |
| `Уровень`, `Level` | — | [bsl-orm/src/catalog_object.rs](../../../crates/bsl-orm/src/catalog_object.rs) |
| `ПроверитьЗаполнение`, `CheckFilling` | — | [bsl-orm/src/catalog_object.rs](../../../crates/bsl-orm/src/catalog_object.rs) |
| `Записать`, `Write` | — | [bsl-orm/src/catalog_object.rs](../../../crates/bsl-orm/src/catalog_object.rs) |
| `Заблокировать`, `Lock` | — | [bsl-orm/src/catalog_object.rs](../../../crates/bsl-orm/src/catalog_object.rs) |
| `ЭтоНовый`, `IsNew` | — | [bsl-orm/src/catalog_object.rs](../../../crates/bsl-orm/src/catalog_object.rs) |
| `Метаданные`, `Metadata` | — | [bsl-orm/src/catalog_object.rs](../../../crates/bsl-orm/src/catalog_object.rs) |
| `ПолучитьСсылкуНового`, `GetNewObjectRef` | — | [bsl-orm/src/catalog_object.rs](../../../crates/bsl-orm/src/catalog_object.rs) |
| `УстановитьСсылкуНового`, `SetNewObjectRef` | — | [bsl-orm/src/catalog_object.rs](../../../crates/bsl-orm/src/catalog_object.rs) |

##### `ОбъектПланаВидовХарактеристикКонфигурации`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `ПометкаУдаления`, `DeletionMark` | чтение и запись | — | [bsl-orm/src/characteristic_object.rs](../../../crates/bsl-orm/src/characteristic_object.rs) |
| `Предопределенный`, `Predefined` | только чтение | — | [bsl-orm/src/characteristic_object.rs](../../../crates/bsl-orm/src/characteristic_object.rs) |
| `ДополнительныеСвойства`, `AdditionalProperties` | только чтение | — | [bsl-orm/src/characteristic_object.rs](../../../crates/bsl-orm/src/characteristic_object.rs) |
| `ОбменДанными`, `DataExchange` | только чтение | — | [bsl-orm/src/characteristic_object.rs](../../../crates/bsl-orm/src/characteristic_object.rs) |
| `Код`, `Code` | чтение и запись | — | [bsl-orm/src/characteristic_object.rs](../../../crates/bsl-orm/src/characteristic_object.rs) |
| `Наименование`, `Description` | чтение и запись | — | [bsl-orm/src/characteristic_object.rs](../../../crates/bsl-orm/src/characteristic_object.rs) |
| `ТипЗначения`, `ValueType` | чтение и запись | — | [bsl-orm/src/characteristic_object.rs](../../../crates/bsl-orm/src/characteristic_object.rs) |
| `ЭтоГруппа`, `IsFolder` | только чтение | — | [bsl-orm/src/characteristic_object.rs](../../../crates/bsl-orm/src/characteristic_object.rs) |
| `Ссылка`, `Ref` | только чтение | — | [bsl-orm/src/characteristic_object.rs](../../../crates/bsl-orm/src/characteristic_object.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `ЭтоНовый`, `IsNew` | — | [bsl-orm/src/characteristic_object.rs](../../../crates/bsl-orm/src/characteristic_object.rs) |

##### `ДокументОбъект`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `Движения`, `RegisterRecords` | только чтение | — | [bsl-orm/src/document_object.rs](../../../crates/bsl-orm/src/document_object.rs) |
| `Номер`, `Number` | чтение и запись | — | [bsl-orm/src/document_object.rs](../../../crates/bsl-orm/src/document_object.rs) |
| `Дата`, `Date` | чтение и запись | — | [bsl-orm/src/document_object.rs](../../../crates/bsl-orm/src/document_object.rs) |
| `ПометкаУдаления`, `DeletionMark` | чтение и запись | — | [bsl-orm/src/document_object.rs](../../../crates/bsl-orm/src/document_object.rs) |
| `Проведен`, `Posted` | чтение и запись | — | [bsl-orm/src/document_object.rs](../../../crates/bsl-orm/src/document_object.rs) |
| `Ссылка`, `Ref` | только чтение | — | [bsl-orm/src/document_object.rs](../../../crates/bsl-orm/src/document_object.rs) |
| `ДополнительныеСвойства`, `AdditionalProperties` | только чтение | — | [bsl-orm/src/document_object.rs](../../../crates/bsl-orm/src/document_object.rs) |
| `ОбменДанными`, `DataExchange` | только чтение | — | [bsl-orm/src/document_object.rs](../../../crates/bsl-orm/src/document_object.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `ПолучитьМакет`, `GetTemplate` | — | [bsl-orm/src/document_object.rs](../../../crates/bsl-orm/src/document_object.rs) |
| `Скопировать`, `Copy` | — | [bsl-orm/src/document_object.rs](../../../crates/bsl-orm/src/document_object.rs) |
| `ПроверитьЗаполнение`, `CheckFilling` | — | [bsl-orm/src/document_object.rs](../../../crates/bsl-orm/src/document_object.rs) |
| `МоментВремени`, `PointInTime` | — | [bsl-orm/src/document_object.rs](../../../crates/bsl-orm/src/document_object.rs) |
| `Записать`, `Write` | — | [bsl-orm/src/document_object.rs](../../../crates/bsl-orm/src/document_object.rs) |
| `УстановитьНовыйНомер`, `SetNewNumber` | — | [bsl-orm/src/document_object.rs](../../../crates/bsl-orm/src/document_object.rs) |
| `ПолучитьСсылкуНового`, `GetNewObjectRef` | — | [bsl-orm/src/document_object.rs](../../../crates/bsl-orm/src/document_object.rs) |
| `УстановитьСсылкуНового`, `SetNewObjectRef` | — | [bsl-orm/src/document_object.rs](../../../crates/bsl-orm/src/document_object.rs) |
| `Заполнить`, `Fill` | — | [bsl-orm/src/document_object.rs](../../../crates/bsl-orm/src/document_object.rs) |
| `ЭтоНовый`, `IsNew` | — | [bsl-orm/src/document_object.rs](../../../crates/bsl-orm/src/document_object.rs) |
| `Модифицированность`, `Modified` | — | [bsl-orm/src/document_object.rs](../../../crates/bsl-orm/src/document_object.rs) |
| `Заблокирован`, `IsLocked` | — | [bsl-orm/src/document_object.rs](../../../crates/bsl-orm/src/document_object.rs) |

##### `БизнесПроцессОбъект`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `ОбменДанными`, `DataExchange` | только чтение | — | [bsl-orm/src/business_process_object.rs](../../../crates/bsl-orm/src/business_process_object.rs) |
| `ВедущаяЗадача`, `HeadTask` | чтение и запись | — | [bsl-orm/src/business_process_object.rs](../../../crates/bsl-orm/src/business_process_object.rs) |
| `Номер`, `Number` | чтение и запись | — | [bsl-orm/src/document_object.rs](../../../crates/bsl-orm/src/document_object.rs) |
| `Дата`, `Date` | чтение и запись | — | [bsl-orm/src/document_object.rs](../../../crates/bsl-orm/src/document_object.rs) |
| `ПометкаУдаления`, `DeletionMark` | чтение и запись | — | [bsl-orm/src/business_process_object.rs](../../../crates/bsl-orm/src/business_process_object.rs) |
| `Стартован`, `Started` | чтение и запись | — | [bsl-orm/src/business_process_object.rs](../../../crates/bsl-orm/src/business_process_object.rs) |
| `Завершен`, `Completed` | чтение и запись | — | [bsl-orm/src/business_process_object.rs](../../../crates/bsl-orm/src/business_process_object.rs) |
| `Ссылка`, `Ref` | только чтение | — | [bsl-orm/src/business_process_object.rs](../../../crates/bsl-orm/src/business_process_object.rs) |
| `ДополнительныеСвойства`, `AdditionalProperties` | только чтение | — | [bsl-orm/src/business_process_object.rs](../../../crates/bsl-orm/src/business_process_object.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `ПолучитьСсылкуНового`, `GetNewObjectRef` | — | [bsl-orm/src/business_process_object.rs](../../../crates/bsl-orm/src/business_process_object.rs) |
| `УстановитьСсылкуНового`, `SetNewObjectRef` | — | [bsl-orm/src/business_process_object.rs](../../../crates/bsl-orm/src/business_process_object.rs) |
| `Модифицированность`, `Modified` | — | [bsl-orm/src/business_process_object.rs](../../../crates/bsl-orm/src/business_process_object.rs) |
| `Прочитать`, `Read` | — | [bsl-orm/src/business_process_object.rs](../../../crates/bsl-orm/src/business_process_object.rs) |
| `Записать`, `Write` | — | [bsl-orm/src/business_process_object.rs](../../../crates/bsl-orm/src/business_process_object.rs) |
| `ПроверитьЗаполнение`, `CheckFilling` | — | [bsl-orm/src/business_process_object.rs](../../../crates/bsl-orm/src/business_process_object.rs) |
| `Заполнить`, `Fill` | — | [bsl-orm/src/business_process_object.rs](../../../crates/bsl-orm/src/business_process_object.rs) |
| `ЭтоНовый`, `IsNew` | — | [bsl-orm/src/business_process_object.rs](../../../crates/bsl-orm/src/business_process_object.rs) |

##### `ОбработкаОбъект`

| Метод | Описание | Реализация |
|---|---|---|
| `Метаданные`, `Metadata` | — | [bsl-orm/src/references.rs](../../../crates/bsl-orm/src/references.rs) |
| `ПолучитьМакет`, `GetTemplate` | — | [bsl-orm/src/managers.rs](../../../crates/bsl-orm/src/managers.rs) |

##### `ОтчетОбъект`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `КомпоновщикНастроек`, `SettingsComposer` | только чтение | — | [bsl-orm/src/data_processor_object.rs](../../../crates/bsl-orm/src/data_processor_object.rs) |
| `СхемаКомпоновкиДанных`, `DataCompositionSchema` | чтение и запись | — | [bsl-orm/src/data_processor_object.rs](../../../crates/bsl-orm/src/data_processor_object.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `Метаданные`, `Metadata` | — | [bsl-orm/src/references.rs](../../../crates/bsl-orm/src/references.rs) |
| `ПолучитьМакет`, `GetTemplate` | — | [bsl-orm/src/managers.rs](../../../crates/bsl-orm/src/managers.rs) |

##### `КоллекцияДвижений`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

| Метод | Описание | Реализация |
|---|---|---|
| `Количество`, `Count` | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |

##### `РегистрСведенийНаборЗаписей`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `Записывать`, `Write` | чтение и запись | — | [bsl-orm/src/document_movements.rs](../../../crates/bsl-orm/src/document_movements.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `Количество`, `Count` | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |

##### `ДокументТабличнаяЧасть`

| Метод | Описание | Реализация |
|---|---|---|
| `Добавить`, `Add` | — | [bsl-orm/src/document_tabular_section.rs](../../../crates/bsl-orm/src/document_tabular_section.rs) |
| `Количество`, `Count` | — | [bsl-orm/src/document_tabular_section.rs](../../../crates/bsl-orm/src/document_tabular_section.rs) |
| `Выгрузить`, `Unload` | — | [bsl-orm/src/document_tabular_section.rs](../../../crates/bsl-orm/src/document_tabular_section.rs) |
| `Удалить`, `Delete` | — | [bsl-orm/src/document_tabular_section.rs](../../../crates/bsl-orm/src/document_tabular_section.rs) |
| `Очистить`, `Clear` | — | [bsl-orm/src/document_tabular_section.rs](../../../crates/bsl-orm/src/document_tabular_section.rs) |

##### `ДокументТабличнаяЧастьСтрока`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `НомерСтроки`, `LineNumber` | только чтение | — | [bsl-rt/src/error_info.rs](../../../crates/bsl-rt/src/error_info.rs) |

##### `ПараметрыОбменаДанными`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `Загрузка`, `Load` | чтение и запись | — | [bsl-orm/src/catalog_object.rs](../../../crates/bsl-orm/src/catalog_object.rs) |
| `Получатели`, `Recipients` | только чтение | — | [bsl-orm/src/catalog_object.rs](../../../crates/bsl-orm/src/catalog_object.rs) |

##### `НаборУзлов`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `АвтоЗаполнение`, `AutoFill` | чтение и запись | — | [bsl-orm/src/catalog_object.rs](../../../crates/bsl-orm/src/catalog_object.rs) |

##### `БиблиотекаКартинок`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

##### `МенеджерРасширенийКонфигурации`

| Метод | Описание | Реализация |
|---|---|---|
| `Получить`, `Get` | — | [bsl-orm/src/configuration_extension.rs](../../../crates/bsl-orm/src/configuration_extension.rs) |
| `ПолучитьИнформациюОПроблемахПримененияВСеансе`, `GetSessionApplicationIssuesInformation` | — | [bsl-orm/src/configuration_extension.rs](../../../crates/bsl-orm/src/configuration_extension.rs) |

##### `МенеджерПользователейИнформационнойБазы`

| Метод | Описание | Реализация |
|---|---|---|
| `ТекущийПользователь`, `CurrentUser` | — | [bsl-orm/src/infobase_users.rs](../../../crates/bsl-orm/src/infobase_users.rs) |

##### `ПользовательИнформационнойБазы`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `Имя`, `Name` | только чтение | — | [bsl-orm/src/infobase_users.rs](../../../crates/bsl-orm/src/infobase_users.rs) |
| `РазделениеДанных`, `DataSeparation` | только чтение | — | [bsl-orm/src/infobase_users.rs](../../../crates/bsl-orm/src/infobase_users.rs) |
| `УникальныйИдентификатор`, `UUID` | только чтение | — | [bsl-orm/src/infobase_users.rs](../../../crates/bsl-orm/src/infobase_users.rs) |

##### `МенеджерПолитикПаролейПользователей`

| Метод | Описание | Реализация |
|---|---|---|
| `НайтиПоИмени`, `FindByName` | — | [bsl-orm/src/infobase_users.rs](../../../crates/bsl-orm/src/infobase_users.rs) |

##### `ПолитикаПаролейПользователей`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `МинимальнаяДлинаПаролей`, `PasswordMinLength` | только чтение | — | [bsl-orm/src/infobase_users.rs](../../../crates/bsl-orm/src/infobase_users.rs) |

##### `МенеджерБлокировкиАутентификацииПользователейИнформационнойБазы`

| Метод | Описание | Реализация |
|---|---|---|
| `ПолучитьНастройки`, `GetSettings` | — | [bsl-orm/src/infobase_users.rs](../../../crates/bsl-orm/src/infobase_users.rs) |
| `УстановитьНастройки`, `SetSettings` | — | [bsl-orm/src/infobase_users.rs](../../../crates/bsl-orm/src/infobase_users.rs) |

##### `НастройкиБлокировкиАутентификацииПользователейИнформационнойБазы`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `ДлительностьБлокировки`, `LockDuration` | чтение и запись | — | [bsl-orm/src/infobase_users.rs](../../../crates/bsl-orm/src/infobase_users.rs) |
| `МаксимальноеКоличествоНеуспешныхПопыток`, `MaxUnsuccessfulAttemptsCount` | чтение и запись | — | [bsl-orm/src/infobase_users.rs](../../../crates/bsl-orm/src/infobase_users.rs) |
| `КодыДополненияИмениПользователя`, `UserNameAdditionCodes` | чтение и запись | — | [bsl-orm/src/infobase_users.rs](../../../crates/bsl-orm/src/infobase_users.rs) |

##### `ГенераторСлучайныхПаролей`

| Метод | Описание | Реализация |
|---|---|---|
| `СлучайныйПароль`, `RandomPassword` | — | [bsl-orm/src/infobase_users.rs](../../../crates/bsl-orm/src/infobase_users.rs) |

##### `МенеджерСервисовИнтеграции`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

| Метод | Описание | Реализация |
|---|---|---|
| `ВыполнитьОбработку`, `ExecuteProcessing` | — | [bsl-orm/src/infobase.rs](../../../crates/bsl-orm/src/infobase.rs) |
| `ОстановитьОбработку`, `StopProcessing` | — | [bsl-orm/src/infobase.rs](../../../crates/bsl-orm/src/infobase.rs) |

##### `РасширениеКонфигурации`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `Назначение`, `Purpose` | только чтение | — | [bsl-orm/src/configuration_extension.rs](../../../crates/bsl-orm/src/configuration_extension.rs) |
| `УникальныйИдентификатор`, `UUID` | только чтение | — | [bsl-orm/src/configuration_extension.rs](../../../crates/bsl-orm/src/configuration_extension.rs) |
| `Имя`, `Name` | только чтение | — | [bsl-orm/src/configuration_extension.rs](../../../crates/bsl-orm/src/configuration_extension.rs) |
| `Версия`, `Version` | только чтение | — | [bsl-orm/src/configuration_extension.rs](../../../crates/bsl-orm/src/configuration_extension.rs) |
| `ХешСумма`, `HashSum` | только чтение | — | [bsl-orm/src/configuration_extension.rs](../../../crates/bsl-orm/src/configuration_extension.rs) |
| `БезопасныйРежим`, `SafeMode` | только чтение | — | [bsl-orm/src/configuration_extension.rs](../../../crates/bsl-orm/src/configuration_extension.rs) |
| `ИспользуетсяВРаспределеннойИнформационнойБазе`, `UsedInDistributedInfoBase` | только чтение | — | [bsl-orm/src/configuration_extension.rs](../../../crates/bsl-orm/src/configuration_extension.rs) |
| `Активно`, `Active` | только чтение | — | [bsl-orm/src/configuration_extension.rs](../../../crates/bsl-orm/src/configuration_extension.rs) |
| `ОбластьДействия`, `Scope` | только чтение | — | [bsl-orm/src/configuration_extension.rs](../../../crates/bsl-orm/src/configuration_extension.rs) |

##### `ИнформацияОПроблемеПримененияРасширенияКонфигурации`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `Расширение`, `Extension` | только чтение | — | [bsl-orm/src/configuration_extension.rs](../../../crates/bsl-orm/src/configuration_extension.rs) |
| `Описание`, `Description` | только чтение | — | [bsl-orm/src/configuration_extension.rs](../../../crates/bsl-orm/src/configuration_extension.rs) |
| `Важность`, `Severity` | только чтение | — | [bsl-orm/src/configuration_extension.rs](../../../crates/bsl-orm/src/configuration_extension.rs) |

##### `Запрос`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `Текст`, `Text` | чтение и запись | — | [bsl-orm/src/query.rs](../../../crates/bsl-orm/src/query.rs) |
| `Параметры`, `Parameters` | только чтение | — | [bsl-orm/src/query.rs](../../../crates/bsl-orm/src/query.rs) |
| `МенеджерВременныхТаблиц`, `TempTablesManager` | чтение и запись | — | [bsl-orm/src/query.rs](../../../crates/bsl-orm/src/query.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `УстановитьПараметр`, `SetParameter` | — | [bsl-orm/src/query.rs](../../../crates/bsl-orm/src/query.rs) |
| `Выполнить`, `Execute` | — | [bsl-orm/src/query.rs](../../../crates/bsl-orm/src/query.rs) |

##### `КомпоновщикМакетаКомпоновкиДанных`

| Метод | Описание | Реализация |
|---|---|---|
| `Выполнить`, `Execute` | — | [bsl-orm/src/data_composition.rs](../../../crates/bsl-orm/src/data_composition.rs) |

##### `ПроцессорКомпоновкиДанных`

| Метод | Описание | Реализация |
|---|---|---|
| `Инициализировать`, `Initialize` | — | [bsl-orm/src/data_composition.rs](../../../crates/bsl-orm/src/data_composition.rs) |
| `Сбросить`, `Reset` | — | [bsl-orm/src/data_composition.rs](../../../crates/bsl-orm/src/data_composition.rs) |
| `Следующий`, `Next` | — | [bsl-orm/src/data_composition.rs](../../../crates/bsl-orm/src/data_composition.rs) |

##### `ПроцессорВыводаРезультатаКомпоновкиДанныхВКоллекциюЗначений`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `ОтображатьПроцентВывода`, `ShowOutputPercent` | чтение и запись | — | [bsl-orm/src/data_composition.rs](../../../crates/bsl-orm/src/data_composition.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `УстановитьОбъект`, `SetObject` | — | [bsl-orm/src/data_composition.rs](../../../crates/bsl-orm/src/data_composition.rs) |
| `Вывести`, `Output` | — | [bsl-orm/src/data_composition.rs](../../../crates/bsl-orm/src/data_composition.rs) |
| `ВывестиЭлемент`, `OutputItem` | — | [bsl-orm/src/data_composition.rs](../../../crates/bsl-orm/src/data_composition.rs) |
| `НачатьВывод`, `BeginOutput` | — | [bsl-orm/src/data_composition.rs](../../../crates/bsl-orm/src/data_composition.rs) |
| `ЗакончитьВывод`, `EndOutput` | — | [bsl-orm/src/data_composition.rs](../../../crates/bsl-orm/src/data_composition.rs) |

##### `МенеджерВременныхТаблиц`

| Метод | Описание | Реализация |
|---|---|---|
| `Закрыть`, `Close` | — | [bsl-orm/src/query.rs](../../../crates/bsl-orm/src/query.rs) |

##### `РезультатЗапроса`

| Метод | Описание | Реализация |
|---|---|---|
| `Пустой`, `IsEmpty` | — | [bsl-orm/src/query.rs](../../../crates/bsl-orm/src/query.rs) |
| `Выбрать`, `Select` | — | [bsl-orm/src/query.rs](../../../crates/bsl-orm/src/query.rs) |
| `Выгрузить`, `Unload` | — | [bsl-orm/src/query.rs](../../../crates/bsl-orm/src/query.rs) |

##### `ВыборкаИзРезультатаЗапроса`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

| Метод | Описание | Реализация |
|---|---|---|
| `Следующий`, `Next` | — | [bsl-orm/src/query.rs](../../../crates/bsl-orm/src/query.rs) |
| `Получить`, `Get` | — | [bsl-orm/src/query.rs](../../../crates/bsl-orm/src/query.rs) |

##### `СправочникВыборка`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

| Метод | Описание | Реализация |
|---|---|---|
| `Следующий`, `Next` | — | [bsl-session/src/client_settings.rs](../../../crates/bsl-session/src/client_settings.rs) |

##### `ХранилищеЗначения`

| Метод | Описание | Реализация |
|---|---|---|
| `Получить`, `Get` | — | [bsl-orm/src/value_storage.rs](../../../crates/bsl-orm/src/value_storage.rs) |

##### `БлокировкаДанных`

| Метод | Описание | Реализация |
|---|---|---|
| `Добавить`, `Add` | — | [bsl-orm/src/infobase.rs](../../../crates/bsl-orm/src/infobase.rs) |
| `Заблокировать`, `Lock` | — | [bsl-orm/src/infobase.rs](../../../crates/bsl-orm/src/infobase.rs) |

##### `СоставОбщегоРеквизита`

| Метод | Описание | Реализация |
|---|---|---|
| `Количество`, `Count` | — | [bsl-orm/src/common_attribute_content.rs](../../../crates/bsl-orm/src/common_attribute_content.rs) |
| `Найти`, `Find` | — | [bsl-orm/src/common_attribute_content.rs](../../../crates/bsl-orm/src/common_attribute_content.rs) |
| `Содержит`, `Contains` | — | [bsl-orm/src/common_attribute_content.rs](../../../crates/bsl-orm/src/common_attribute_content.rs) |

##### `ЭлементСоставаОбщегоРеквизита`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `Метаданные`, `Metadata` | только чтение | — | [bsl-orm/src/common_attribute_content.rs](../../../crates/bsl-orm/src/common_attribute_content.rs) |
| `Использование`, `Use` | только чтение | — | [bsl-orm/src/common_attribute_content.rs](../../../crates/bsl-orm/src/common_attribute_content.rs) |
| `УсловноеРазделение`, `ConditionalSeparation` | только чтение | — | [bsl-orm/src/common_attribute_content.rs](../../../crates/bsl-orm/src/common_attribute_content.rs) |

### `bsl-pdf` 0.3.0

#### Глобальные функции

Нет.

#### Конструкторы

| Имя и псевдонимы | Описание | Реализация |
|---|---|---|
| `ДокументPDF`, `PDFDocument` | — | [bsl-pdf/src/lib.rs](../../../crates/bsl-pdf/src/lib.rs) |
| `КоллекцияВложенийPDF`, `PDFAttachmentCollection` | — | [bsl-pdf/src/lib.rs](../../../crates/bsl-pdf/src/lib.rs) |
| `ОписаниеОтображаемогоОбъектаPDF`, `PDFRepresentationObjectDescription` — *объявлено без реализации* | — | [bsl-pdf/src/lib.rs](../../../crates/bsl-pdf/src/lib.rs) |
| `ЗаписьPDF`, `PDFWriter` — *объявлено без реализации* | — | [bsl-pdf/src/lib.rs](../../../crates/bsl-pdf/src/lib.rs) |
| `ЧтениеPDF`, `PDFReader` — *объявлено без реализации* | — | [bsl-pdf/src/lib.rs](../../../crates/bsl-pdf/src/lib.rs) |

#### Типы объектов

| Имя | Представление типа | Дополнительные имена | Описание | Реализация |
|---|---|---|---|---|
| `КоллекцияВложенийPDF` | `КоллекцияВложенийPDF` | `PDFAttachmentCollection` | — | [bsl-pdf/src/document/surface.rs](../../../crates/bsl-pdf/src/document/surface.rs) |
| `ВложениеPDF` | `ВложениеPDF` | `PDFAttachment` | — | [bsl-pdf/src/document/surface.rs](../../../crates/bsl-pdf/src/document/surface.rs) |
| `ДокументPDF` | `Документ PDF` | `PDFDocument` | — | [bsl-pdf/src/document/surface.rs](../../../crates/bsl-pdf/src/document/surface.rs) |
| `КоллекцияСтраницPDF` | `КоллекцияСтраницPDF` | `PDFPagesCollection` | — | [bsl-pdf/src/document/surface.rs](../../../crates/bsl-pdf/src/document/surface.rs) |
| `СтраницаPDF` | `СтраницаPDF` | `PDFPage` | — | [bsl-pdf/src/document/surface.rs](../../../crates/bsl-pdf/src/document/surface.rs) |

#### Члены объектов

##### `КоллекцияВложенийPDF`

| Метод | Описание | Реализация |
|---|---|---|
| `Количество`, `Count` | — | [bsl-pdf/src/document/objects.rs](../../../crates/bsl-pdf/src/document/objects.rs) |
| `Получить`, `Get` | — | [bsl-pdf/src/document/objects.rs](../../../crates/bsl-pdf/src/document/objects.rs) |
| `Найти`, `Find` | — | [bsl-pdf/src/document/objects.rs](../../../crates/bsl-pdf/src/document/objects.rs) |
| `Добавить`, `Add` | — | [bsl-pdf/src/document/objects.rs](../../../crates/bsl-pdf/src/document/objects.rs) |
| `Удалить`, `Delete` | — | [bsl-pdf/src/document/objects.rs](../../../crates/bsl-pdf/src/document/objects.rs) |
| `Очистить`, `Clear` | — | [bsl-pdf/src/document/objects.rs](../../../crates/bsl-pdf/src/document/objects.rs) |
| `Индекс`, `IndexOf` | — | [bsl-pdf/src/document/objects.rs](../../../crates/bsl-pdf/src/document/objects.rs) |

##### `ВложениеPDF`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `ИмяФайла`, `FileName` | чтение и запись | — | [bsl-pdf/src/document/objects.rs](../../../crates/bsl-pdf/src/document/objects.rs) |
| `ТипСодержимого`, `MIMEType` | чтение и запись | — | [bsl-pdf/src/document/objects.rs](../../../crates/bsl-pdf/src/document/objects.rs) |
| `Содержимое`, `Content` | чтение и запись | — | [bsl-pdf/src/document/objects.rs](../../../crates/bsl-pdf/src/document/objects.rs) |
| `ТипСвязи`, `RelationshipType` | чтение и запись | — | [bsl-pdf/src/document/objects.rs](../../../crates/bsl-pdf/src/document/objects.rs) |

##### `ДокументPDF`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `Вложения`, `Attachments` | только чтение | — | [bsl-pdf/src/document/objects.rs](../../../crates/bsl-pdf/src/document/objects.rs) |
| `Страницы`, `Pages` | только чтение | — | [bsl-pdf/src/document/objects.rs](../../../crates/bsl-pdf/src/document/objects.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `Прочитать`, `Read` | — | [bsl-pdf/src/document/objects.rs](../../../crates/bsl-pdf/src/document/objects.rs) |
| `Записать`, `Write` | — | [bsl-pdf/src/document/objects.rs](../../../crates/bsl-pdf/src/document/objects.rs) |

##### `КоллекцияСтраницPDF`

| Метод | Описание | Реализация |
|---|---|---|
| `Количество`, `Count` | — | [bsl-pdf/src/document/objects.rs](../../../crates/bsl-pdf/src/document/objects.rs) |
| `Получить`, `Get` | — | [bsl-pdf/src/document/objects.rs](../../../crates/bsl-pdf/src/document/objects.rs) |
| `Индекс`, `IndexOf` | — | [bsl-pdf/src/document/objects.rs](../../../crates/bsl-pdf/src/document/objects.rs) |

##### `СтраницаPDF`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `Номер`, `Number` | только чтение | — | [bsl-pdf/src/document/objects.rs](../../../crates/bsl-pdf/src/document/objects.rs) |
| `Ширина`, `Width` | только чтение | — | [bsl-pdf/src/document/objects.rs](../../../crates/bsl-pdf/src/document/objects.rs) |
| `Высота`, `Height` | только чтение | — | [bsl-pdf/src/document/objects.rs](../../../crates/bsl-pdf/src/document/objects.rs) |
| `Ориентация`, `Orientation` | только чтение | — | [bsl-pdf/src/document/objects.rs](../../../crates/bsl-pdf/src/document/objects.rs) |
| `ПолеСлева`, `LeftMargin` | только чтение | — | [bsl-pdf/src/document/objects.rs](../../../crates/bsl-pdf/src/document/objects.rs) |
| `ПолеСправа`, `RightMargin` | только чтение | — | [bsl-pdf/src/document/objects.rs](../../../crates/bsl-pdf/src/document/objects.rs) |
| `ПолеСверху`, `TopMargin` | только чтение | — | [bsl-pdf/src/document/objects.rs](../../../crates/bsl-pdf/src/document/objects.rs) |
| `ПолеСнизу`, `BottomMargin` | только чтение | — | [bsl-pdf/src/document/objects.rs](../../../crates/bsl-pdf/src/document/objects.rs) |

### `bsl-regexp` 0.3.0

#### Глобальные функции

| Имя и псевдонимы | Вид | Описание | Реализация |
|---|---|---|---|
| `СтрНайтиПоРегулярномуВыражению`, `StrFindByRegularExpression` | функция | — | [bsl-regexp/src/lib.rs](../../../crates/bsl-regexp/src/lib.rs) |
| `СтрНайтиВсеПоРегулярномуВыражению`, `StrFindAllByRegularExpression` | функция | — | [bsl-regexp/src/lib.rs](../../../crates/bsl-regexp/src/lib.rs) |
| `СтрЗаменитьПоРегулярномуВыражению`, `StrReplaceByRegularExpression` | функция | — | [bsl-regexp/src/lib.rs](../../../crates/bsl-regexp/src/lib.rs) |
| `СтрПодобнаПоРегулярномуВыражению`, `StrLikeByRegularExpression` | функция | — | [bsl-regexp/src/lib.rs](../../../crates/bsl-regexp/src/lib.rs) |

#### Конструкторы

Нет.

#### Типы объектов

| Имя | Представление типа | Дополнительные имена | Описание | Реализация |
|---|---|---|---|---|
| `ГруппаРезультатаПоискаПоРегулярномуВыражению` | `Группа результата поиска по регулярному выражению` | `ResultOfSearchByRegularExpressionGroup` | — | [bsl-regexp/src/lib.rs](../../../crates/bsl-regexp/src/lib.rs) |
| `РезультатПоискаПоРегулярномуВыражению` | `Результат поиска по регулярному выражению` | `ResultOfSearchByRegularExpression` | — | [bsl-regexp/src/lib.rs](../../../crates/bsl-regexp/src/lib.rs) |

#### Члены объектов

##### `ГруппаРезультатаПоискаПоРегулярномуВыражению`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `Значение`, `Value` | только чтение | — | [bsl-regexp/src/lib.rs](../../../crates/bsl-regexp/src/lib.rs) |
| `НачальнаяПозиция`, `StartIndex` | только чтение | — | [bsl-regexp/src/lib.rs](../../../crates/bsl-regexp/src/lib.rs) |
| `Длина`, `Length` | только чтение | — | [bsl-regexp/src/lib.rs](../../../crates/bsl-regexp/src/lib.rs) |

##### `РезультатПоискаПоРегулярномуВыражению`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `Значение`, `Value` | только чтение | — | [bsl-regexp/src/lib.rs](../../../crates/bsl-regexp/src/lib.rs) |
| `НачальнаяПозиция`, `StartIndex` | только чтение | — | [bsl-regexp/src/lib.rs](../../../crates/bsl-regexp/src/lib.rs) |
| `Длина`, `Length` | только чтение | — | [bsl-regexp/src/lib.rs](../../../crates/bsl-regexp/src/lib.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `ПолучитьГруппы`, `GetGroups` | — | [bsl-regexp/src/lib.rs](../../../crates/bsl-regexp/src/lib.rs) |

### `bsl-session` 0.2.0

#### Глобальные свойства

| Имя и псевдонимы | Функция получения |
|---|---|
| `ДополнительныеНастройкиАутентификации`, `AdditionalAuthenticationSettings` | `ДополнительныеНастройкиАутентификации`, `AdditionalAuthenticationSettings` |
| `ПолнотекстовыйПоиск`, `FullTextSearch` | `ПолнотекстовыйПоиск`, `FullTextSearch` |
| `СредстваКриптографии`, `CryptoTools` | `СредстваКриптографии`, `CryptoTools` |
| `ОтправкаДоставляемыхУведомлений`, `DeliverableNotificationSend` | `ОтправкаДоставляемыхУведомлений`, `DeliverableNotificationSend` |
| `КлиентскоеПриложение`, `ClientApplication` | `КлиентскоеПриложение`, `ClientApplication` |
| `ИсторияРаботыПользователя`, `UserWorkHistory` | `ПолучитьИсториюРаботыПользователяСеанса` |
| `ХранилищеВариантовОтчетов`, `ReportsVariantsStorage` | `ПолучитьХранилищеВариантовОтчетовСеанса` |
| `СистемаВзаимодействия`, `CollaborationSystem` | `СистемаВзаимодействия`, `CollaborationSystem` |
| `РегламентныеЗадания`, `ScheduledJobs` | `РегламентныеЗадания`, `ScheduledJobs` |
| `ПараметрыСеанса`, `SessionParameters` | `ПолучитьПараметрыСеанса` |
| `ХранилищеОбщихНастроек`, `CommonSettingsStorage` | `ПолучитьХранилищеОбщихНастроек` |
| `ХранилищеСистемныхНастроек`, `SystemSettingsStorage` | `ПолучитьХранилищеСистемныхНастроек` |
| `ХранилищеНастроекДанныхФорм`, `FormDataSettingsStorage` | `ПолучитьХранилищеНастроекДанныхФорм` |
| `ХранилищеПользовательскихНастроекОтчетов`, `ReportsUserSettingsStorage` | `ПолучитьСтандартноеХранилищеПользовательскихНастроекОтчетов`, `GetStandardReportsUserSettingsStorage` |
| `ХранилищеПользовательскихНастроекДинамическихСписков`, `DynamicListsUserSettingsStorage` | `ПолучитьСтандартноеХранилищеПользовательскихНастроекДинамическихСписков`, `GetStandardDynamicListsUserSettingsStorage` |
| `ОбработкаОшибок`, `ErrorProcessing` | `ПолучитьОбработкуОшибокСеанса` |

#### Глобальные функции

| Имя и псевдонимы | Вид | Описание | Реализация |
|---|---|---|---|
| `КодЛокализацииИнформационнойБазы`, `InfoBaseLocaleCode` | функция | — | [bsl-session/src/globals.rs](../../../crates/bsl-session/src/globals.rs) |
| `ТекущийВариантИнтерфейсаКлиентскогоПриложения`, `ClientApplicationInterfaceCurrentVariant` | функция | — | [bsl-session/src/client_application.rs](../../../crates/bsl-session/src/client_application.rs) |
| `ПолучитьБлокировкуСеансов`, `GetSessionsLock` | функция | — | [bsl-session/src/infobase_sessions.rs](../../../crates/bsl-session/src/infobase_sessions.rs) |
| `УстановитьБлокировкуСеансов`, `SetSessionsLock` | процедура | — | [bsl-session/src/infobase_sessions.rs](../../../crates/bsl-session/src/infobase_sessions.rs) |
| `ДополнительныеНастройкиАутентификации`, `AdditionalAuthenticationSettings` | функция | — | [bsl-session/src/globals.rs](../../../crates/bsl-session/src/globals.rs) |
| `УстановитьНастройкиКлиентаЛицензирования`, `SetLicensingClientParameters` | процедура | — | [bsl-session/src/globals.rs](../../../crates/bsl-session/src/globals.rs) |
| `ПолучитьДополнительныйПараметрКлиентаЛицензирования`, `GetLicensingClientAdditionalParameter` | функция | — | [bsl-session/src/globals.rs](../../../crates/bsl-session/src/globals.rs) |
| `ПолучитьИдентификаторКонфигурации`, `GetConfigurationID` | функция | — | [bsl-session/src/globals.rs](../../../crates/bsl-session/src/globals.rs) |
| `ПолучитьИмяКлиентаЛицензирования`, `GetLicensingClientName` | функция | — | [bsl-session/src/globals.rs](../../../crates/bsl-session/src/globals.rs) |
| `ПолучитьРазделительПутиСервера`, `GetServerPathSeparator` | функция | — | [bsl-session/src/globals.rs](../../../crates/bsl-session/src/globals.rs) |
| `ПолнотекстовыйПоиск`, `FullTextSearch` | функция | — | [bsl-session/src/globals.rs](../../../crates/bsl-session/src/globals.rs) |
| `ПредставлениеКодаЛокализации`, `LocaleCodePresentation` | функция | — | [bsl-session/src/globals.rs](../../../crates/bsl-session/src/globals.rs) |
| `ПолучитьДопустимыеКодыЛокализации`, `GetAvailableLocaleCodes` | функция | — | [bsl-session/src/globals.rs](../../../crates/bsl-session/src/globals.rs) |
| `ПараметрыДоступа`, `AccessParameters` | функция | — | [bsl-session/src/globals.rs](../../../crates/bsl-session/src/globals.rs) |
| `НайтиПомеченныеНаУдаление`, `FindMarkedForDeletion` | функция | — | [bsl-session/src/globals.rs](../../../crates/bsl-session/src/globals.rs) |
| `ПолучитьИсториюРаботыПользователяСеанса` | функция | — | [bsl-session/src/client_settings.rs](../../../crates/bsl-session/src/client_settings.rs) |
| `ПолучитьХранилищеВариантовОтчетовСеанса` | функция | — | [bsl-session/src/client_settings.rs](../../../crates/bsl-session/src/client_settings.rs) |
| `ВыгрузитьЖурналРегистрации`, `UnloadEventLog` | процедура | — | [bsl-session/src/event_log.rs](../../../crates/bsl-session/src/event_log.rs) |
| `ПолучитьНавигационнуюСсылкуИнформационнойБазы`, `GetInfoBaseURL` | функция | — | [bsl-session/src/infobase_sessions.rs](../../../crates/bsl-session/src/infobase_sessions.rs) |
| `ПолучитьСкоростьКлиентскогоСоединения`, `GetClientConnectionSpeed` | функция | — | [bsl-session/src/client_application.rs](../../../crates/bsl-session/src/client_application.rs) |
| `НомерСеансаИнформационнойБазы`, `InfoBaseSessionNumber` | функция | — | [bsl-session/src/infobase_sessions.rs](../../../crates/bsl-session/src/infobase_sessions.rs) |
| `СистемаВзаимодействия`, `CollaborationSystem` | функция | — | [bsl-session/src/globals.rs](../../../crates/bsl-session/src/globals.rs) |
| `РегламентныеЗадания`, `ScheduledJobs` | функция | — | [bsl-session/src/job_schedule.rs](../../../crates/bsl-session/src/job_schedule.rs) |
| `ПолучитьСеансыИнформационнойБазы`, `GetInfoBaseSessions` | функция | — | [bsl-session/src/infobase_sessions.rs](../../../crates/bsl-session/src/infobase_sessions.rs) |
| `ИмяКомпьютера`, `ComputerName` | функция | — | [bsl-session/src/globals.rs](../../../crates/bsl-session/src/globals.rs) |
| `ПолучитьИнформациюЭкрановКлиента`, `GetClientDisplaysInformation` | функция | — | [bsl-session/src/client_application.rs](../../../crates/bsl-session/src/client_application.rs) |
| `ПолучитьПараметрыСеанса` | функция | — | [bsl-session/src/parameters.rs](../../../crates/bsl-session/src/parameters.rs) |
| `ТекущийРежимЗапуска`, `CurrentRunMode` | функция | — | [bsl-session/src/globals.rs](../../../crates/bsl-session/src/globals.rs) |
| `СтрокаСоединенияИнформационнойБазы`, `InfoBaseConnectionString` | функция | — | [bsl-session/src/infobase_sessions.rs](../../../crates/bsl-session/src/infobase_sessions.rs) |
| `ПолучитьТекущийСеансИнформационнойБазы`, `GetCurrentInfoBaseSession` | функция | — | [bsl-session/src/infobase_sessions.rs](../../../crates/bsl-session/src/infobase_sessions.rs) |
| `НомерСоединенияИнформационнойБазы`, `InfoBaseConnectionNumber` | функция | — | [bsl-session/src/infobase_sessions.rs](../../../crates/bsl-session/src/infobase_sessions.rs) |
| `УстановитьБезопасныйРежим`, `SetSafeMode` | процедура | — | [bsl-session/src/safe_mode.rs](../../../crates/bsl-session/src/safe_mode.rs) |
| `БезопасныйРежим`, `SafeMode` | функция | — | [bsl-session/src/safe_mode.rs](../../../crates/bsl-session/src/safe_mode.rs) |
| `УстановитьПривилегированныйРежим`, `SetPrivilegedMode` | процедура | — | [bsl-session/src/safe_mode.rs](../../../crates/bsl-session/src/safe_mode.rs) |
| `ПривилегированныйРежим`, `PrivilegedMode` | функция | — | [bsl-session/src/safe_mode.rs](../../../crates/bsl-session/src/safe_mode.rs) |
| `ПолучитьСоединенияИнформационнойБазы`, `GetInfoBaseConnections` | функция | — | [bsl-session/src/infobase_sessions.rs](../../../crates/bsl-session/src/infobase_sessions.rs) |
| `УстановитьОтключениеБезопасногоРежима`, `SetSafeModeDisabled` | процедура | — | [bsl-session/src/safe_mode.rs](../../../crates/bsl-session/src/safe_mode.rs) |
| `ОбновитьПовторноИспользуемыеЗначения`, `RefreshReusableValues` | процедура | — | [bsl-session/src/globals.rs](../../../crates/bsl-session/src/globals.rs) |
| `ПолучитьОбработкуОшибокСеанса` | функция | — | [bsl-session/src/error_processing.rs](../../../crates/bsl-session/src/error_processing.rs) |
| `КонфигурацияБазыДанныхИзмененаДинамически`, `DataBaseConfigurationChangedDynamically` | функция | — | [bsl-session/src/globals.rs](../../../crates/bsl-session/src/globals.rs) |
| `ПолучитьОтключениеБезопасногоРежима`, `GetSafeModeDisabled` | функция | — | [bsl-session/src/safe_mode.rs](../../../crates/bsl-session/src/safe_mode.rs) |
| `ЗаписьЖурналаРегистрации`, `WriteLogEvent` | процедура | — | [bsl-session/src/event_log.rs](../../../crates/bsl-session/src/event_log.rs) |
| `ПолучитьХранилищеОбщихНастроек` | функция | — | [bsl-session/src/client_settings.rs](../../../crates/bsl-session/src/client_settings.rs) |
| `ПолучитьХранилищеСистемныхНастроек` | функция | — | [bsl-session/src/client_settings.rs](../../../crates/bsl-session/src/client_settings.rs) |
| `ПолучитьХранилищеНастроекДанныхФорм` | функция | — | [bsl-session/src/client_settings.rs](../../../crates/bsl-session/src/client_settings.rs) |
| `ПолучитьСтандартноеХранилищеПользовательскихНастроекОтчетов`, `GetStandardReportsUserSettingsStorage` | функция | — | [bsl-session/src/client_settings.rs](../../../crates/bsl-session/src/client_settings.rs) |
| `ПолучитьСтандартноеХранилищеПользовательскихНастроекДинамическихСписков`, `GetStandardDynamicListsUserSettingsStorage` | функция | — | [bsl-session/src/client_settings.rs](../../../crates/bsl-session/src/client_settings.rs) |
| `УстановитьБезопасныйРежимРазделенияДанных`, `SetDataSeparationSafeMode` | процедура | — | [bsl-session/src/safe_mode.rs](../../../crates/bsl-session/src/safe_mode.rs) |
| `БезопасныйРежимРазделенияДанных`, `DataSeparationSafeMode` | функция | — | [bsl-session/src/safe_mode.rs](../../../crates/bsl-session/src/safe_mode.rs) |
| `ПодключитьВнешнююКомпоненту`, `AttachAddIn` | функция | — | [bsl-session/src/globals.rs](../../../crates/bsl-session/src/globals.rs) |
| `ПолучитьИспользованиеСобытияЖурналаРегистрации`, `GetEventLogEventUse` | функция | — | [bsl-session/src/event_log.rs](../../../crates/bsl-session/src/event_log.rs) |
| `УстановитьИспользованиеСобытияЖурналаРегистрации`, `SetEventLogEventUse` | процедура | — | [bsl-session/src/event_log.rs](../../../crates/bsl-session/src/event_log.rs) |
| `ПолучитьИспользованиеЖурналаРегистрации`, `GetEventLogUsing` | функция | — | [bsl-session/src/event_log.rs](../../../crates/bsl-session/src/event_log.rs) |
| `КлиентскоеПриложение`, `ClientApplication` | функция | — | [bsl-session/src/client_application.rs](../../../crates/bsl-session/src/client_application.rs) |
| `ТекущийКодЛокализации`, `CurrentLocaleCode` | функция | — | [bsl-session/src/globals.rs](../../../crates/bsl-session/src/globals.rs) |
| `ТекущийЯзыкСистемы`, `CurrentSystemLanguage` | функция | — | [bsl-session/src/globals.rs](../../../crates/bsl-session/src/globals.rs) |
| `ПолучитьРежимВнешнихРесурсов`, `GetExternalResourcesMode` | функция | — | [bsl-session/src/globals.rs](../../../crates/bsl-session/src/globals.rs) |
| `ОтправкаДоставляемыхУведомлений`, `DeliverableNotificationSend` | функция | — | [bsl-session/src/globals.rs](../../../crates/bsl-session/src/globals.rs) |
| `СредстваКриптографии`, `CryptoTools` | функция | — | [bsl-session/src/globals.rs](../../../crates/bsl-session/src/globals.rs) |
| `ПолучитьЧасовойПоясИнформационнойБазы`, `GetInfoBaseTimeZone` | функция | — | [bsl-session/src/globals.rs](../../../crates/bsl-session/src/globals.rs) |

#### Конструкторы

| Имя и псевдонимы | Описание | Реализация |
|---|---|---|
| `АдминистрированиеСервера`, `ServerAdministration` | — | [bsl-session/src/globals.rs](../../../crates/bsl-session/src/globals.rs) |
| `СовместноеИспользованиеПриложенийСистемыВзаимодействия`, `CollaborationSystemApplicationLinks` | — | [bsl-session/src/globals.rs](../../../crates/bsl-session/src/globals.rs) |
| `КоллекцияИдентификаторовПриложенийСистемыВзаимодействия`, `CollaborationSystemApplicationIDCollection` | — | [bsl-session/src/globals.rs](../../../crates/bsl-session/src/globals.rs) |
| `ИдентификаторСообщенияСистемыВзаимодействия`, `CollaborationSystemMessageID` | — | [bsl-session/src/globals.rs](../../../crates/bsl-session/src/globals.rs) |
| `ИдентификаторПриложенияСистемыВзаимодействия`, `CollaborationSystemApplicationID` | — | [bsl-session/src/globals.rs](../../../crates/bsl-session/src/globals.rs) |
| `ИнтернетПочтовоеСообщение`, `InternetMailMessage` | — | [bsl-session/src/globals.rs](../../../crates/bsl-session/src/globals.rs) |
| `КонтекстОбсужденияСистемыВзаимодействия`, `CollaborationSystemConversationContext` | — | [bsl-session/src/globals.rs](../../../crates/bsl-session/src/globals.rs) |
| `ИдентификаторОбсужденияСистемыВзаимодействия`, `CollaborationSystemConversationID` | — | [bsl-session/src/globals.rs](../../../crates/bsl-session/src/globals.rs) |
| `ОтборОбсужденийСистемыВзаимодействия`, `CollaborationSystemConversationsFilter` | — | [bsl-session/src/globals.rs](../../../crates/bsl-session/src/globals.rs) |
| `ОтборСообщенийСистемыВзаимодействия`, `CollaborationSystemMessagesFilter` | — | [bsl-session/src/globals.rs](../../../crates/bsl-session/src/globals.rs) |
| `СистемнаяИнформация`, `SystemInfo` | — | [bsl-session/src/system_information.rs](../../../crates/bsl-session/src/system_information.rs) |
| `НастройкиНачальнойСтраницы`, `HomePageSettings` | — | [bsl-session/src/client_settings.rs](../../../crates/bsl-session/src/client_settings.rs) |
| `СоставФормНачальнойСтраницы`, `HomePageForms` | — | [bsl-session/src/client_settings.rs](../../../crates/bsl-session/src/client_settings.rs) |
| `ОписаниеНастроек`, `SettingsDescription` | — | [bsl-session/src/client_settings.rs](../../../crates/bsl-session/src/client_settings.rs) |
| `РасписаниеРегламентногоЗадания`, `JobSchedule` | — | [bsl-session/src/job_schedule.rs](../../../crates/bsl-session/src/job_schedule.rs) |
| `ОписаниеЗащитыОтОпасныхДействий`, `UnsafeOperationProtectionDescription` | — | [bsl-session/src/safe_mode.rs](../../../crates/bsl-session/src/safe_mode.rs) |
| `ИспользованиеСобытияЖурналаРегистрации`, `EventLogEventUse` | — | [bsl-session/src/event_log.rs](../../../crates/bsl-session/src/event_log.rs) |
| `НастройкиКлиентскогоПриложения`, `ClientSettings` | — | [bsl-session/src/client_application.rs](../../../crates/bsl-session/src/client_application.rs) |
| `НастройкиКомандногоИнтерфейса`, `CommandInterfaceSettings` | — | [bsl-session/src/client_application.rs](../../../crates/bsl-session/src/client_application.rs) |
| `НастройкиИнтерфейсаКлиентскогоПриложения`, `ClientApplicationInterfaceSettings` | — | [bsl-session/src/client_application.rs](../../../crates/bsl-session/src/client_application.rs) |
| `НастройкиСоставаИнтерфейсаКлиентскогоПриложения`, `ClientApplicationInterfaceContentSettings` | — | [bsl-session/src/client_application.rs](../../../crates/bsl-session/src/client_application.rs) |
| `ГруппаНастройкиСоставаИнтерфейсаКлиентскогоПриложения`, `ClientApplicationInterfaceContentSettingsGroup` | — | [bsl-session/src/client_application.rs](../../../crates/bsl-session/src/client_application.rs) |
| `ЭлементНастройкиСоставаИнтерфейсаКлиентскогоПриложения`, `ClientApplicationInterfaceContentSettingsItem` | — | [bsl-session/src/client_application.rs](../../../crates/bsl-session/src/client_application.rs) |
| `ОписаниеИспользованияСобытияДоступЖурналаРегистрации`, `EventLogAccessEventUseDescription` | — | [bsl-session/src/event_log.rs](../../../crates/bsl-session/src/event_log.rs) |
| `ЭлементПланировщика`, `PlannerItem` | — | [bsl-session/src/planner_item.rs](../../../crates/bsl-session/src/planner_item.rs) |

#### Типы объектов

| Имя | Представление типа | Дополнительные имена | Описание | Реализация |
|---|---|---|---|---|
| `ИдентификаторОбсужденияСистемыВзаимодействия` | `ИдентификаторОбсужденияСистемыВзаимодействия` | `CollaborationSystemConversationID` | — | [bsl-session/src/globals.rs](../../../crates/bsl-session/src/globals.rs) |
| `ПараметрыСеанса` | `ПараметрыСеанса` | — | — | [bsl-session/src/parameters.rs](../../../crates/bsl-session/src/parameters.rs) |
| `СистемнаяИнформация` | `СистемнаяИнформация` | — | — | [bsl-session/src/system_information.rs](../../../crates/bsl-session/src/system_information.rs) |
| `СеансИнформационнойБазы` | `Активный сеанс` | `InfoBaseSession` | — | [bsl-session/src/infobase_sessions.rs](../../../crates/bsl-session/src/infobase_sessions.rs) |
| `СоединениеИнформационнойБазы` | `Активное соединение` | `InfoBaseConnection` | — | [bsl-session/src/infobase_sessions.rs](../../../crates/bsl-session/src/infobase_sessions.rs) |
| `СтандартноеХранилищеНастроекМенеджер` | `Стандартное хранилище настроек` | `StandardSettingsStorageManager` | — | [bsl-session/src/client_settings.rs](../../../crates/bsl-session/src/client_settings.rs) |
| `СтандартноеХранилищеНастроекВыборка` | `Standard settings storage Selected data` | `StandardSettingsStorageSelection` | — | [bsl-session/src/client_settings.rs](../../../crates/bsl-session/src/client_settings.rs) |
| `НастройкиНачальнойСтраницы` | `Настройки начальной страницы` | `HomePageSettings` | — | [bsl-session/src/client_settings.rs](../../../crates/bsl-session/src/client_settings.rs) |
| `СоставФормНачальнойСтраницы` | `Состав форм начальной страницы` | `HomePageForms` | — | [bsl-session/src/client_settings.rs](../../../crates/bsl-session/src/client_settings.rs) |
| `МенеджерОбработкиОшибок` | `Менеджер обработки ошибок` | `ErrorProcessingManager` | — | [bsl-session/src/error_processing.rs](../../../crates/bsl-session/src/error_processing.rs) |
| `НастройкиОбработкиОшибок` | `Настройки обработки ошибок на клиенте` | `ErrorProcessingSettings` | — | [bsl-session/src/error_processing.rs](../../../crates/bsl-session/src/error_processing.rs) |
| `ОписаниеНастроек` | `Описание настройки` | `SettingsDescription` | — | [bsl-session/src/client_settings.rs](../../../crates/bsl-session/src/client_settings.rs) |
| `РасписаниеРегламентногоЗадания` | `Расписание задания` | `JobSchedule` | — | [bsl-session/src/job_schedule.rs](../../../crates/bsl-session/src/job_schedule.rs) |
| `ОписаниеЗащитыОтОпасныхДействий` | `Описание защиты от опасных действий` | `UnsafeOperationProtectionDescription` | — | [bsl-session/src/safe_mode.rs](../../../crates/bsl-session/src/safe_mode.rs) |
| `ИспользованиеСобытияЖурналаРегистрации` | `Использование события журнала регистрации` | `EventLogEventUse` | — | [bsl-session/src/event_log.rs](../../../crates/bsl-session/src/event_log.rs) |
| `ОписаниеИспользованияСобытияДоступЖурналаРегистрации` | `Описание использования события доступ журнала регистрации` | `EventLogAccessEventUseDescription` | — | [bsl-session/src/event_log.rs](../../../crates/bsl-session/src/event_log.rs) |
| `НастройкиКлиентскогоПриложения` | `Настройки клиентского приложения` | `ClientSettings` | — | [bsl-session/src/client_application.rs](../../../crates/bsl-session/src/client_application.rs) |
| `КлиентскоеПриложение` | `Клиентское приложение` | `ClientApplication` | — | [bsl-session/src/client_application.rs](../../../crates/bsl-session/src/client_application.rs) |
| `НастройкиКомандногоИнтерфейса` | `Настройки командного интерфейса` | `CommandInterfaceSettings` | — | [bsl-session/src/client_application.rs](../../../crates/bsl-session/src/client_application.rs) |
| `НастройкиИнтерфейсаКлиентскогоПриложения` | `Настройки панелей` | `ClientApplicationInterfaceSettings` | — | [bsl-session/src/client_application.rs](../../../crates/bsl-session/src/client_application.rs) |
| `НастройкиСоставаИнтерфейсаКлиентскогоПриложения` | `Настройки состава интерфейса клиентского приложения` | `ClientApplicationInterfaceContentSettings` | — | [bsl-session/src/client_application.rs](../../../crates/bsl-session/src/client_application.rs) |
| `ГруппаНастройкиСоставаИнтерфейсаКлиентскогоПриложения` | `Группа настройки состава интерфейса клиентского приложения` | `ClientApplicationInterfaceContentSettingsGroup` | — | [bsl-session/src/client_application.rs](../../../crates/bsl-session/src/client_application.rs) |
| `ЭлементНастройкиСоставаИнтерфейсаКлиентскогоПриложения` | `Элемент настройки состава интерфейса клиентского приложения` | `ClientApplicationInterfaceContentSettingsItem` | — | [bsl-session/src/client_application.rs](../../../crates/bsl-session/src/client_application.rs) |
| `ИнформацияЭкранаКлиента` | `ИнформацияЭкранаКлиента` | `ClientDisplayInformation` | — | [bsl-session/src/client_application.rs](../../../crates/bsl-session/src/client_application.rs) |
| `ЭлементПланировщика` | `Элемент планировщика` | `PlannerItem` | — | [bsl-session/src/planner_item.rs](../../../crates/bsl-session/src/planner_item.rs) |

#### Члены объектов

##### `ПараметрыСеанса`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

| Метод | Описание | Реализация |
|---|---|---|
| `Очистить`, `Clear` | — | [bsl-session/src/parameters.rs](../../../crates/bsl-session/src/parameters.rs) |

##### `СистемнаяИнформация`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `ВерсияПриложения`, `AppVersion` | только чтение | — | [bsl-session/src/system_information.rs](../../../crates/bsl-session/src/system_information.rs) |

##### `СеансИнформационнойБазы`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `НомерСеанса`, `SessionNumber` | только чтение | — | [bsl-session/src/infobase_sessions.rs](../../../crates/bsl-session/src/infobase_sessions.rs) |
| `НачалоСеанса`, `SessionStarted` | только чтение | — | [bsl-session/src/infobase_sessions.rs](../../../crates/bsl-session/src/infobase_sessions.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `ПолучитьФоновоеЗадание`, `GetBackgroundJob` | — | [bsl-session/src/infobase_sessions.rs](../../../crates/bsl-session/src/infobase_sessions.rs) |

##### `СоединениеИнформационнойБазы`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `НомерСоединения`, `ConnectionNumber` | только чтение | — | [bsl-session/src/infobase_sessions.rs](../../../crates/bsl-session/src/infobase_sessions.rs) |
| `ИмяПриложения`, `ApplicationName` | только чтение | — | [bsl-session/src/infobase_sessions.rs](../../../crates/bsl-session/src/infobase_sessions.rs) |
| `ИмяКомпьютера`, `ComputerName` | только чтение | — | [bsl-session/src/infobase_sessions.rs](../../../crates/bsl-session/src/infobase_sessions.rs) |

##### `СтандартноеХранилищеНастроекМенеджер`

| Метод | Описание | Реализация |
|---|---|---|
| `Загрузить`, `Load` | — | [bsl-session/src/client_settings.rs](../../../crates/bsl-session/src/client_settings.rs) |
| `Сохранить`, `Save` | — | [bsl-session/src/client_settings.rs](../../../crates/bsl-session/src/client_settings.rs) |
| `Выбрать`, `Select` | — | [bsl-session/src/client_settings.rs](../../../crates/bsl-session/src/client_settings.rs) |
| `ПолучитьСписок`, `GetList` | — | [bsl-session/src/client_settings.rs](../../../crates/bsl-session/src/client_settings.rs) |
| `ПолучитьОписание`, `GetDescription` | — | [bsl-session/src/client_settings.rs](../../../crates/bsl-session/src/client_settings.rs) |
| `Удалить`, `Delete` | — | [bsl-session/src/client_settings.rs](../../../crates/bsl-session/src/client_settings.rs) |

##### `СтандартноеХранилищеНастроекВыборка`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `КлючОбъекта`, `ObjectKey` | только чтение | — | [bsl-session/src/client_settings.rs](../../../crates/bsl-session/src/client_settings.rs) |
| `КлючНастроек`, `SettingsKey` | только чтение | — | [bsl-session/src/client_settings.rs](../../../crates/bsl-session/src/client_settings.rs) |
| `Пользователь`, `User` | только чтение | — | [bsl-session/src/client_settings.rs](../../../crates/bsl-session/src/client_settings.rs) |
| `Настройки`, `Settings` | только чтение | — | [bsl-session/src/client_settings.rs](../../../crates/bsl-session/src/client_settings.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `Следующий`, `Next` | — | [bsl-session/src/client_settings.rs](../../../crates/bsl-session/src/client_settings.rs) |

##### `НастройкиНачальнойСтраницы`

| Метод | Описание | Реализация |
|---|---|---|
| `ПолучитьСоставФорм`, `GetForms` | — | [bsl-session/src/client_settings.rs](../../../crates/bsl-session/src/client_settings.rs) |
| `УстановитьСоставФорм`, `SetForms` | — | [bsl-session/src/client_settings.rs](../../../crates/bsl-session/src/client_settings.rs) |

##### `СоставФормНачальнойСтраницы`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `ЛеваяКолонка`, `LeftColumn` | только чтение | — | [bsl-session/src/client_settings.rs](../../../crates/bsl-session/src/client_settings.rs) |
| `ПраваяКолонка`, `RightColumn` | только чтение | — | [bsl-session/src/client_settings.rs](../../../crates/bsl-session/src/client_settings.rs) |

##### `МенеджерОбработкиОшибок`

| Метод | Описание | Реализация |
|---|---|---|
| `КраткоеПредставлениеОшибки`, `BriefErrorDescription` | — | [bsl-session/src/error_processing.rs](../../../crates/bsl-session/src/error_processing.rs) |
| `ПодробноеПредставлениеОшибки`, `DetailErrorDescription` | — | [bsl-session/src/error_processing.rs](../../../crates/bsl-session/src/error_processing.rs) |
| `ПолучитьНастройкиПользователя`, `GetUserSettings` | — | [bsl-session/src/error_processing.rs](../../../crates/bsl-session/src/error_processing.rs) |

##### `НастройкиОбработкиОшибок`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `ОтправлятьОтчетНаКлиенте`, `SendReportOnClient` | только чтение | — | [bsl-session/src/error_processing.rs](../../../crates/bsl-session/src/error_processing.rs) |
| `АдресСервисаОбработкиОшибок`, `ErrorProcessingServiceAddress` | только чтение | — | [bsl-session/src/error_processing.rs](../../../crates/bsl-session/src/error_processing.rs) |

##### `ОписаниеНастроек`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `КлючОбъекта`, `ObjectKey` | чтение и запись | — | [bsl-session/src/client_settings.rs](../../../crates/bsl-session/src/client_settings.rs) |
| `КлючНастроек`, `SettingsKey` | чтение и запись | — | [bsl-session/src/client_settings.rs](../../../crates/bsl-session/src/client_settings.rs) |
| `Пользователь`, `User` | чтение и запись | — | [bsl-session/src/client_settings.rs](../../../crates/bsl-session/src/client_settings.rs) |
| `Представление`, `Presentation` | чтение и запись | — | [bsl-session/src/client_settings.rs](../../../crates/bsl-session/src/client_settings.rs) |
| `ДополнительныеСвойства`, `AdditionalProperties` | только чтение | — | [bsl-session/src/client_settings.rs](../../../crates/bsl-session/src/client_settings.rs) |

##### `РасписаниеРегламентногоЗадания`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `ВремяЗавершения`, `CompletionTime` | чтение и запись | — | [bsl-session/src/job_schedule.rs](../../../crates/bsl-session/src/job_schedule.rs) |
| `ВремяКонца`, `EndTime` | чтение и запись | — | [bsl-session/src/job_schedule.rs](../../../crates/bsl-session/src/job_schedule.rs) |
| `ВремяНачала`, `BeginTime` | чтение и запись | — | [bsl-session/src/job_schedule.rs](../../../crates/bsl-session/src/job_schedule.rs) |
| `ДатаКонца`, `EndDate` | чтение и запись | — | [bsl-session/src/job_schedule.rs](../../../crates/bsl-session/src/job_schedule.rs) |
| `ДатаНачала`, `BeginDate` | чтение и запись | — | [bsl-session/src/job_schedule.rs](../../../crates/bsl-session/src/job_schedule.rs) |
| `ДеньВМесяце`, `DayInMonth` | чтение и запись | — | [bsl-session/src/job_schedule.rs](../../../crates/bsl-session/src/job_schedule.rs) |
| `ДеньНеделиВМесяце`, `WeekDayInMonth` | чтение и запись | — | [bsl-session/src/job_schedule.rs](../../../crates/bsl-session/src/job_schedule.rs) |
| `ДетальныеРасписанияДня`, `DetailedDailySchedules` | чтение и запись | — | [bsl-session/src/job_schedule.rs](../../../crates/bsl-session/src/job_schedule.rs) |
| `ДниНедели`, `WeekDays` | чтение и запись | — | [bsl-session/src/job_schedule.rs](../../../crates/bsl-session/src/job_schedule.rs) |
| `ИнтервалЗавершения`, `CompletionInterval` | чтение и запись | — | [bsl-session/src/job_schedule.rs](../../../crates/bsl-session/src/job_schedule.rs) |
| `Месяцы`, `Months` | чтение и запись | — | [bsl-session/src/job_schedule.rs](../../../crates/bsl-session/src/job_schedule.rs) |
| `ПаузаПовтора`, `RepeatPause` | чтение и запись | — | [bsl-session/src/job_schedule.rs](../../../crates/bsl-session/src/job_schedule.rs) |
| `ПериодНедель`, `WeeksPeriod` | чтение и запись | — | [bsl-session/src/job_schedule.rs](../../../crates/bsl-session/src/job_schedule.rs) |
| `ПериодПовтораВТечениеДня`, `RepeatPeriodInDay` | чтение и запись | — | [bsl-session/src/job_schedule.rs](../../../crates/bsl-session/src/job_schedule.rs) |
| `ПериодПовтораДней`, `DaysRepeatPeriod` | чтение и запись | — | [bsl-session/src/job_schedule.rs](../../../crates/bsl-session/src/job_schedule.rs) |

##### `ОписаниеЗащитыОтОпасныхДействий`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `ПредупреждатьОбОпасныхДействиях`, `UnsafeOperationWarnings` | чтение и запись | — | [bsl-session/src/safe_mode.rs](../../../crates/bsl-session/src/safe_mode.rs) |

##### `ИспользованиеСобытияЖурналаРегистрации`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `Использование`, `Use` | чтение и запись | — | [bsl-session/src/event_log.rs](../../../crates/bsl-session/src/event_log.rs) |
| `ОписаниеИспользования`, `UseDescription` | чтение и запись | — | [bsl-session/src/event_log.rs](../../../crates/bsl-session/src/event_log.rs) |

##### `ОписаниеИспользованияСобытияДоступЖурналаРегистрации`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `Объект`, `Object` | чтение и запись | — | [bsl-session/src/event_log.rs](../../../crates/bsl-session/src/event_log.rs) |
| `ПоляДоступа`, `AccessFields` | чтение и запись | — | [bsl-session/src/event_log.rs](../../../crates/bsl-session/src/event_log.rs) |
| `ПоляРегистрации`, `LoggedFields` | чтение и запись | — | [bsl-session/src/event_log.rs](../../../crates/bsl-session/src/event_log.rs) |

##### `НастройкиКлиентскогоПриложения`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `ОтображатьПанелиНавигацииИДействий`, `ShowNavigationAndActionsPanels` | чтение и запись | — | [bsl-session/src/client_application.rs](../../../crates/bsl-session/src/client_application.rs) |
| `ОтображатьПанельРазделов`, `ShowSectionsPanel` | чтение и запись | — | [bsl-session/src/client_application.rs](../../../crates/bsl-session/src/client_application.rs) |
| `РежимОткрытияФормПриложения`, `ApplicationFormsOpenningMode` | чтение и запись | — | [bsl-session/src/client_application.rs](../../../crates/bsl-session/src/client_application.rs) |
| `ВариантИнтерфейсаКлиентскогоПриложения`, `ClientApplicationInterfaceVariant` | чтение и запись | — | [bsl-session/src/client_application.rs](../../../crates/bsl-session/src/client_application.rs) |

##### `КлиентскоеПриложение`

| Метод | Описание | Реализация |
|---|---|---|
| `ТипПриложения`, `ApplicationType` | — | [bsl-session/src/client_application.rs](../../../crates/bsl-session/src/client_application.rs) |

##### `НастройкиКомандногоИнтерфейса`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `ОтображениеПанелиРазделов`, `SectionsPanelRepresentation` | чтение и запись | — | [bsl-session/src/client_application.rs](../../../crates/bsl-session/src/client_application.rs) |

##### `НастройкиИнтерфейсаКлиентскогоПриложения`

| Метод | Описание | Реализация |
|---|---|---|
| `ПолучитьСостав`, `GetContent` | — | [bsl-session/src/client_application.rs](../../../crates/bsl-session/src/client_application.rs) |
| `УстановитьСостав`, `SetContent` | — | [bsl-session/src/client_application.rs](../../../crates/bsl-session/src/client_application.rs) |

##### `НастройкиСоставаИнтерфейсаКлиентскогоПриложения`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `Верх`, `Top` | только чтение | — | [bsl-session/src/client_application.rs](../../../crates/bsl-session/src/client_application.rs) |
| `Лево`, `Left` | только чтение | — | [bsl-session/src/client_application.rs](../../../crates/bsl-session/src/client_application.rs) |
| `Низ`, `Bottom` | только чтение | — | [bsl-session/src/client_application.rs](../../../crates/bsl-session/src/client_application.rs) |
| `Право`, `Right` | только чтение | — | [bsl-session/src/client_application.rs](../../../crates/bsl-session/src/client_application.rs) |

##### `ГруппаНастройкиСоставаИнтерфейсаКлиентскогоПриложения`

| Метод | Описание | Реализация |
|---|---|---|
| `Добавить`, `Add` | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |
| `Количество`, `Count` | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |
| `Получить`, `Get` | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |

##### `ЭлементНастройкиСоставаИнтерфейсаКлиентскогоПриложения`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `Имя`, `Name` | только чтение | — | [bsl-orm/src/managers.rs](../../../crates/bsl-orm/src/managers.rs) |

##### `ИнформацияЭкранаКлиента`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `Ширина`, `Width` | только чтение | — | [bsl-session/src/client_application.rs](../../../crates/bsl-session/src/client_application.rs) |
| `Высота`, `Height` | только чтение | — | [bsl-session/src/client_application.rs](../../../crates/bsl-session/src/client_application.rs) |
| `DPI` | только чтение | — | [bsl-session/src/client_application.rs](../../../crates/bsl-session/src/client_application.rs) |

##### `ЭлементПланировщика`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `Значение`, `Value` | чтение и запись | — | [bsl-session/src/planner_item.rs](../../../crates/bsl-session/src/planner_item.rs) |
| `Начало`, `Begin` | чтение и запись | — | [bsl-session/src/planner_item.rs](../../../crates/bsl-session/src/planner_item.rs) |
| `Конец`, `End` | чтение и запись | — | [bsl-session/src/planner_item.rs](../../../crates/bsl-session/src/planner_item.rs) |
| `Текст`, `Text` | чтение и запись | — | [bsl-session/src/planner_item.rs](../../../crates/bsl-session/src/planner_item.rs) |
| `Подсказка`, `ToolTip` | чтение и запись | — | [bsl-session/src/planner_item.rs](../../../crates/bsl-session/src/planner_item.rs) |
| `Удаленный`, `Deleted` | чтение и запись | — | [bsl-session/src/planner_item.rs](../../../crates/bsl-session/src/planner_item.rs) |
| `Родитель`, `Parent` | только чтение | — | [bsl-session/src/planner_item.rs](../../../crates/bsl-session/src/planner_item.rs) |
| `ЗначенияИзмерений`, `DimensionValues` | только чтение | — | [bsl-session/src/planner_item.rs](../../../crates/bsl-session/src/planner_item.rs) |

### `bsl-spreadsheet` 0.3.0

#### Глобальные функции

Нет.

#### Конструкторы

| Имя и псевдонимы | Описание | Реализация |
|---|---|---|
| `ТабличныйДокумент`, `SpreadsheetDocument` | — | [bsl-spreadsheet/src/lib.rs](../../../crates/bsl-spreadsheet/src/lib.rs) |
| `ГеографическаяСхема`, `GeographicalSchema` — *объявлено без реализации* | — | [bsl-spreadsheet/src/lib.rs](../../../crates/bsl-spreadsheet/src/lib.rs) |
| `ГрафическаяСхема`, `GraphicalSchema` | — | [bsl-spreadsheet/src/graphical_schema.rs](../../../crates/bsl-spreadsheet/src/graphical_schema.rs) |
| `Диаграмма`, `Chart` — *объявлено без реализации* | — | [bsl-spreadsheet/src/lib.rs](../../../crates/bsl-spreadsheet/src/lib.rs) |

#### Типы объектов

| Имя | Представление типа | Дополнительные имена | Описание | Реализация |
|---|---|---|---|---|
| `ОбластьЯчеекТабличногоДокумента` | `Область ячеек табличного документа` | `SpreadsheetDocumentRange` | — | [bsl-spreadsheet/src/document/objects.rs](../../../crates/bsl-spreadsheet/src/document/objects.rs) |
| `ТабличныйДокумент` | `Табличный документ` | `SpreadsheetDocument` | — | [bsl-spreadsheet/src/document/objects.rs](../../../crates/bsl-spreadsheet/src/document/objects.rs) |
| `КоллекцияРисунковТабличногоДокумента` | `Коллекция рисунков табличного документа` | `SpreadsheetDocumentDrawingCollection` | — | [bsl-spreadsheet/src/document/objects.rs](../../../crates/bsl-spreadsheet/src/document/objects.rs) |
| `РисунокТабличногоДокумента` | `Рисунок табличного документа` | `SpreadsheetDocumentDrawing` | — | [bsl-spreadsheet/src/document/objects.rs](../../../crates/bsl-spreadsheet/src/document/objects.rs) |
| `ПараметрыМакетаТабличногоДокумента` | `Параметры макета табличного документа` | `SpreadsheetDocumentTemplateParameters` | — | [bsl-spreadsheet/src/document/objects.rs](../../../crates/bsl-spreadsheet/src/document/objects.rs) |
| `ГеографическаяСхема` | `Поле географической схемы` | `GeographicalSchema` | — | [bsl-spreadsheet/src/lib.rs](../../../crates/bsl-spreadsheet/src/lib.rs) |
| `ГрафическаяСхема` | `Графическая схема` | `GraphicalSchema` | — | [bsl-spreadsheet/src/graphical_schema.rs](../../../crates/bsl-spreadsheet/src/graphical_schema.rs) |
| `ЭлементыГрафическойСхемы` | `Элементы графической схемы` | `GraphicalSchemaItems` | — | [bsl-spreadsheet/src/graphical_schema.rs](../../../crates/bsl-spreadsheet/src/graphical_schema.rs) |
| `Диаграмма` | `Диаграмма` | `Chart` | — | [bsl-spreadsheet/src/lib.rs](../../../crates/bsl-spreadsheet/src/lib.rs) |

#### Члены объектов

##### `ОбластьЯчеекТабличногоДокумента`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

| Метод | Описание | Реализация |
|---|---|---|
| `Область`, `Area` | — | [bsl-spreadsheet/src/document/objects.rs](../../../crates/bsl-spreadsheet/src/document/objects.rs) |
| `Объединить`, `Merge` | — | [bsl-spreadsheet/src/document/objects.rs](../../../crates/bsl-spreadsheet/src/document/objects.rs) |
| `Разъединить`, `Unmerge` | — | [bsl-spreadsheet/src/document/objects.rs](../../../crates/bsl-spreadsheet/src/document/objects.rs) |

##### `ТабличныйДокумент`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

| Метод | Описание | Реализация |
|---|---|---|
| `Область`, `Area` | — | [bsl-spreadsheet/src/document/objects.rs](../../../crates/bsl-spreadsheet/src/document/objects.rs) |
| `Объединить`, `Merge` | — | [bsl-spreadsheet/src/document/objects.rs](../../../crates/bsl-spreadsheet/src/document/objects.rs) |
| `Разъединить`, `Unmerge` | — | [bsl-spreadsheet/src/document/objects.rs](../../../crates/bsl-spreadsheet/src/document/objects.rs) |
| `Записать`, `Write` | — | [bsl-spreadsheet/src/document/objects.rs](../../../crates/bsl-spreadsheet/src/document/objects.rs) |
| `Прочитать`, `Read` | — | [bsl-spreadsheet/src/document/objects.rs](../../../crates/bsl-spreadsheet/src/document/objects.rs) |
| `Вывести`, `Output` | — | [bsl-spreadsheet/src/document/objects.rs](../../../crates/bsl-spreadsheet/src/document/objects.rs) |
| `ПолучитьОбласть`, `GetArea` | — | [bsl-spreadsheet/src/document/objects.rs](../../../crates/bsl-spreadsheet/src/document/objects.rs) |
| `Очистить`, `Clear` | — | [bsl-spreadsheet/src/document/objects.rs](../../../crates/bsl-spreadsheet/src/document/objects.rs) |
| `НачатьГруппуСтрок`, `StartRowGroup` | — | [bsl-spreadsheet/src/document/objects.rs](../../../crates/bsl-spreadsheet/src/document/objects.rs) |
| `ЗакончитьГруппуСтрок`, `EndRowGroup` | — | [bsl-spreadsheet/src/document/objects.rs](../../../crates/bsl-spreadsheet/src/document/objects.rs) |

##### `КоллекцияРисунковТабличногоДокумента`

| Метод | Описание | Реализация |
|---|---|---|
| `Добавить`, `Add` | — | [bsl-spreadsheet/src/document/objects.rs](../../../crates/bsl-spreadsheet/src/document/objects.rs) |
| `Количество`, `Count` | — | [bsl-spreadsheet/src/document/objects.rs](../../../crates/bsl-spreadsheet/src/document/objects.rs) |

##### `РисунокТабличногоДокумента`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

##### `ПараметрыМакетаТабличногоДокумента`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

##### `ГрафическаяСхема`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `ЭлементыГрафическойСхемы`, `GraphicalSchemaItems` | только чтение | — | [bsl-spreadsheet/src/graphical_schema.rs](../../../crates/bsl-spreadsheet/src/graphical_schema.rs) |

##### `ЭлементыГрафическойСхемы`

| Метод | Описание | Реализация |
|---|---|---|
| `Количество`, `Count` | — | [bsl-spreadsheet/src/graphical_schema.rs](../../../crates/bsl-spreadsheet/src/graphical_schema.rs) |
| `Найти`, `Find` | — | [bsl-spreadsheet/src/graphical_schema.rs](../../../crates/bsl-spreadsheet/src/graphical_schema.rs) |

### `bsl-stream` 0.3.0

Зависимости: `bsl-binbuf=0.3.0`.

#### Глобальные функции

Нет.

#### Конструкторы

| Имя и псевдонимы | Описание | Реализация |
|---|---|---|
| `ПотокВПамяти`, `MemoryStream` | — | [bsl-stream/src/lib.rs](../../../crates/bsl-stream/src/lib.rs) |
| `ФайловыйПоток`, `FileStream` | — | [bsl-stream/src/lib.rs](../../../crates/bsl-stream/src/lib.rs) |
| `ЧтениеДанных`, `DataReader` | — | [bsl-stream/src/lib.rs](../../../crates/bsl-stream/src/lib.rs) |
| `ЗаписьДанных`, `DataWriter` | — | [bsl-stream/src/lib.rs](../../../crates/bsl-stream/src/lib.rs) |
| `ФайловыеПотоки`, `FileStreams` | — | [bsl-stream/src/lib.rs](../../../crates/bsl-stream/src/lib.rs) |
| `ЧтениеТекста`, `TextReader` | — | [bsl-stream/src/lib.rs](../../../crates/bsl-stream/src/lib.rs) |

#### Типы объектов

| Имя | Представление типа | Дополнительные имена | Описание | Реализация |
|---|---|---|---|---|
| `ЧтениеДанных` | `Чтение данных` | `DataReader` | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |
| `РезультатЧтенияДанных` | `Результат чтения данных` | `ReadDataResult` | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |
| `ЗаписьДанных` | `Запись данных` | `DataWriter` | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |
| `Поток` | `Поток` | `Stream` | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |
| `МенеджерФайловыхПотоков` | `Менеджер файловых потоков` | `FileStreamsManager` | — | [bsl-stream/src/stream.rs](../../../crates/bsl-stream/src/stream.rs) |
| `ФайловыйПоток` | `Файловый поток` | `FileStream` | — | [bsl-stream/src/stream.rs](../../../crates/bsl-stream/src/stream.rs) |
| `ПотокВПамяти` | `Файловый поток` | `MemoryStream` | — | [bsl-stream/src/stream.rs](../../../crates/bsl-stream/src/stream.rs) |
| `ЧтениеТекста` | `ЧтениеТекста` | `TextReader` | — | [bsl-stream/src/textreader.rs](../../../crates/bsl-stream/src/textreader.rs) |

#### Члены объектов

##### `ЧтениеДанных`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `ПорядокБайтов`, `ByteOrder` | чтение и запись | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |
| `КодировкаТекста`, `TextEncoding` | чтение и запись | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |
| `РазделительСтрок`, `LineSplitter` | чтение и запись | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `Прочитать`, `Read` | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |
| `Записать`, `Write` | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |
| `Закрыть`, `Close` | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |
| `Пропустить`, `Skip` | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |
| `ПрочитатьЦелое16`, `ReadInt16` | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |
| `ПрочитатьЦелое32`, `ReadInt32` | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |
| `ПрочитатьЦелое64`, `ReadInt64` | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |
| `ЗаписатьЦелое16`, `WriteInt16` | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |
| `ЗаписатьЦелое32`, `WriteInt32` | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |
| `ЗаписатьЦелое64`, `WriteInt64` | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |
| `ПрочитатьБайт`, `ReadByte` | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |
| `ПрочитатьВБуферДвоичныхДанных`, `ReadIntoBinaryDataBuffer` | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |
| `ПрочитатьСимволы`, `ReadChars` | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |
| `ПрочитатьСтроку`, `ReadLine` | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |
| `ЗаписатьБайт`, `WriteByte` | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |
| `ЗаписатьСимволы`, `WriteChars` | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |
| `ЗаписатьСтроку`, `WriteLine` | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |
| `ИсходныйПоток`, `SourceStream` | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |
| `КопироватьВ`, `CopyTo` | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |
| `ЗаписатьБуферДвоичныхДанных`, `WriteBinaryDataBuffer` | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |

##### `РезультатЧтенияДанных`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `Размер`, `Size` | только чтение | `Размер` результата чтения — только на чтение. | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `ПолучитьДвоичныеДанные`, `GetBinaryData` | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |
| `ПолучитьБуферДвоичныхДанных`, `GetBinaryDataBuffer` | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |

##### `ЗаписьДанных`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `ПорядокБайтов`, `ByteOrder` | чтение и запись | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |
| `КодировкаТекста`, `TextEncoding` | чтение и запись | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |
| `РазделительСтрок`, `LineSplitter` | чтение и запись | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `Прочитать`, `Read` | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |
| `Записать`, `Write` | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |
| `Закрыть`, `Close` | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |
| `Пропустить`, `Skip` | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |
| `ПрочитатьЦелое16`, `ReadInt16` | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |
| `ПрочитатьЦелое32`, `ReadInt32` | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |
| `ПрочитатьЦелое64`, `ReadInt64` | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |
| `ЗаписатьЦелое16`, `WriteInt16` | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |
| `ЗаписатьЦелое32`, `WriteInt32` | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |
| `ЗаписатьЦелое64`, `WriteInt64` | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |
| `ПрочитатьБайт`, `ReadByte` | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |
| `ПрочитатьВБуферДвоичныхДанных`, `ReadIntoBinaryDataBuffer` | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |
| `ПрочитатьСимволы`, `ReadChars` | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |
| `ПрочитатьСтроку`, `ReadLine` | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |
| `ЗаписатьБайт`, `WriteByte` | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |
| `ЗаписатьСимволы`, `WriteChars` | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |
| `ЗаписатьСтроку`, `WriteLine` | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |
| `ИсходныйПоток`, `SourceStream` | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |
| `КопироватьВ`, `CopyTo` | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |
| `ЗаписатьБуферДвоичныхДанных`, `WriteBinaryDataBuffer` | — | [bsl-stream/src/datarw.rs](../../../crates/bsl-stream/src/datarw.rs) |

##### `Поток`

| Метод | Описание | Реализация |
|---|---|---|
| `УстановитьРазмер`, `SetSize` | — | [bsl-stream/src/stream.rs](../../../crates/bsl-stream/src/stream.rs) |
| `Записать`, `Write` | — | [bsl-stream/src/stream.rs](../../../crates/bsl-stream/src/stream.rs) |
| `Прочитать`, `Read` | — | [bsl-stream/src/stream.rs](../../../crates/bsl-stream/src/stream.rs) |
| `Закрыть`, `Close` | — | [bsl-stream/src/stream.rs](../../../crates/bsl-stream/src/stream.rs) |
| `Размер`, `Size` | — | [bsl-stream/src/stream.rs](../../../crates/bsl-stream/src/stream.rs) |
| `ТекущаяПозиция`, `CurrentPosition` | — | [bsl-stream/src/stream.rs](../../../crates/bsl-stream/src/stream.rs) |
| `Перейти`, `Seek` | — | [bsl-stream/src/stream.rs](../../../crates/bsl-stream/src/stream.rs) |
| `КопироватьВ`, `CopyTo` | — | [bsl-stream/src/stream.rs](../../../crates/bsl-stream/src/stream.rs) |
| `ЗакрытьИПолучитьДвоичныеДанные`, `CloseAndGetBinaryData` | — | [bsl-stream/src/stream.rs](../../../crates/bsl-stream/src/stream.rs) |

##### `МенеджерФайловыхПотоков`

| Метод | Описание | Реализация |
|---|---|---|
| `СоздатьВременныйФайлАсинх`, `CreateTempFileAsync` | — | [bsl-stream/src/stream.rs](../../../crates/bsl-stream/src/stream.rs) |
| `СоздатьВременныйФайл`, `CreateTempFile` | — | [bsl-stream/src/stream.rs](../../../crates/bsl-stream/src/stream.rs) |
| `Открыть`, `Open` | — | [bsl-stream/src/stream.rs](../../../crates/bsl-stream/src/stream.rs) |
| `ОткрытьДляЧтения`, `OpenForRead` | — | [bsl-stream/src/stream.rs](../../../crates/bsl-stream/src/stream.rs) |
| `ОткрытьДляЗаписи`, `OpenForWrite` | — | [bsl-stream/src/stream.rs](../../../crates/bsl-stream/src/stream.rs) |
| `ОткрытьДляДописывания`, `OpenForAppend` | — | [bsl-stream/src/stream.rs](../../../crates/bsl-stream/src/stream.rs) |
| `Создать`, `Create` | — | [bsl-stream/src/stream.rs](../../../crates/bsl-stream/src/stream.rs) |

##### `ФайловыйПоток`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `ДоступнаЗапись`, `CanWrite` | только чтение | — | [bsl-stream/src/stream.rs](../../../crates/bsl-stream/src/stream.rs) |
| `ДоступноЧтение`, `CanRead` | только чтение | — | [bsl-stream/src/stream.rs](../../../crates/bsl-stream/src/stream.rs) |
| `ДоступноИзменениеПозиции`, `CanSeek` | только чтение | — | [bsl-stream/src/stream.rs](../../../crates/bsl-stream/src/stream.rs) |
| `ИмяФайла`, `FileName` | только чтение | — | [bsl-stream/src/stream.rs](../../../crates/bsl-stream/src/stream.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `УстановитьРазмер`, `SetSize` | — | [bsl-stream/src/stream.rs](../../../crates/bsl-stream/src/stream.rs) |
| `Записать`, `Write` | — | [bsl-stream/src/stream.rs](../../../crates/bsl-stream/src/stream.rs) |
| `Прочитать`, `Read` | — | [bsl-stream/src/stream.rs](../../../crates/bsl-stream/src/stream.rs) |
| `Закрыть`, `Close` | — | [bsl-stream/src/stream.rs](../../../crates/bsl-stream/src/stream.rs) |
| `Размер`, `Size` | — | [bsl-stream/src/stream.rs](../../../crates/bsl-stream/src/stream.rs) |
| `ТекущаяПозиция`, `CurrentPosition` | — | [bsl-stream/src/stream.rs](../../../crates/bsl-stream/src/stream.rs) |
| `Перейти`, `Seek` | — | [bsl-stream/src/stream.rs](../../../crates/bsl-stream/src/stream.rs) |
| `КопироватьВ`, `CopyTo` | — | [bsl-stream/src/stream.rs](../../../crates/bsl-stream/src/stream.rs) |
| `ЗакрытьИПолучитьДвоичныеДанные`, `CloseAndGetBinaryData` | — | [bsl-stream/src/stream.rs](../../../crates/bsl-stream/src/stream.rs) |

##### `ПотокВПамяти`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `ДоступнаЗапись`, `CanWrite` | только чтение | — | [bsl-stream/src/stream.rs](../../../crates/bsl-stream/src/stream.rs) |
| `ДоступноЧтение`, `CanRead` | только чтение | — | [bsl-stream/src/stream.rs](../../../crates/bsl-stream/src/stream.rs) |
| `ДоступноИзменениеПозиции`, `CanSeek` | только чтение | — | [bsl-stream/src/stream.rs](../../../crates/bsl-stream/src/stream.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `УстановитьРазмер`, `SetSize` | — | [bsl-stream/src/stream.rs](../../../crates/bsl-stream/src/stream.rs) |
| `Записать`, `Write` | — | [bsl-stream/src/stream.rs](../../../crates/bsl-stream/src/stream.rs) |
| `Прочитать`, `Read` | — | [bsl-stream/src/stream.rs](../../../crates/bsl-stream/src/stream.rs) |
| `Закрыть`, `Close` | — | [bsl-stream/src/stream.rs](../../../crates/bsl-stream/src/stream.rs) |
| `Размер`, `Size` | — | [bsl-stream/src/stream.rs](../../../crates/bsl-stream/src/stream.rs) |
| `ТекущаяПозиция`, `CurrentPosition` | — | [bsl-stream/src/stream.rs](../../../crates/bsl-stream/src/stream.rs) |
| `Перейти`, `Seek` | — | [bsl-stream/src/stream.rs](../../../crates/bsl-stream/src/stream.rs) |
| `КопироватьВ`, `CopyTo` | — | [bsl-stream/src/stream.rs](../../../crates/bsl-stream/src/stream.rs) |
| `ЗакрытьИПолучитьДвоичныеДанные`, `CloseAndGetBinaryData` | — | [bsl-stream/src/stream.rs](../../../crates/bsl-stream/src/stream.rs) |

##### `ЧтениеТекста`

| Метод | Описание | Реализация |
|---|---|---|
| `Прочитать`, `Read` | — | [bsl-stream/src/textreader.rs](../../../crates/bsl-stream/src/textreader.rs) |
| `Закрыть`, `Close` | — | [bsl-stream/src/textreader.rs](../../../crates/bsl-stream/src/textreader.rs) |

### `bsl-textdoc` 0.3.0

#### Глобальные функции

Нет.

#### Конструкторы

| Имя и псевдонимы | Описание | Реализация |
|---|---|---|
| `ТекстовыйДокумент`, `TextDocument` | — | [bsl-textdoc/src/lib.rs](../../../crates/bsl-textdoc/src/lib.rs) |

#### Типы объектов

| Имя | Представление типа | Дополнительные имена | Описание | Реализация |
|---|---|---|---|---|
| `ТекстовыйДокумент` | `Текстовый документ` | `TextDocument` | — | [bsl-textdoc/src/lib.rs](../../../crates/bsl-textdoc/src/lib.rs) |
| `ПараметрыМакетаТекстовогоДокумента` | `Параметры макета текстового документа` | `TextTemplateParameters` | — | [bsl-textdoc/src/lib.rs](../../../crates/bsl-textdoc/src/lib.rs) |

#### Члены объектов

##### `ТекстовыйДокумент`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `Параметры`, `Parameters` | только чтение | Единственное свойство документа — коллекция параметров макета, только на чтение. | [bsl-textdoc/src/lib.rs](../../../crates/bsl-textdoc/src/lib.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `УстановитьТекст`, `SetText` | — | [bsl-textdoc/src/lib.rs](../../../crates/bsl-textdoc/src/lib.rs) |
| `ПолучитьТекст`, `GetText` | — | [bsl-textdoc/src/lib.rs](../../../crates/bsl-textdoc/src/lib.rs) |
| `КоличествоСтрок`, `LineCount` | — | [bsl-textdoc/src/lib.rs](../../../crates/bsl-textdoc/src/lib.rs) |
| `ПолучитьСтроку`, `GetLine` | — | [bsl-textdoc/src/lib.rs](../../../crates/bsl-textdoc/src/lib.rs) |
| `ДобавитьСтроку`, `AddLine` | — | [bsl-textdoc/src/lib.rs](../../../crates/bsl-textdoc/src/lib.rs) |
| `ВставитьСтроку`, `InsertLine` | — | [bsl-textdoc/src/lib.rs](../../../crates/bsl-textdoc/src/lib.rs) |
| `ЗаменитьСтроку`, `ReplaceLine` | — | [bsl-textdoc/src/lib.rs](../../../crates/bsl-textdoc/src/lib.rs) |
| `УдалитьСтроку`, `DeleteLine` | — | [bsl-textdoc/src/lib.rs](../../../crates/bsl-textdoc/src/lib.rs) |
| `Очистить`, `Clear` | — | [bsl-textdoc/src/lib.rs](../../../crates/bsl-textdoc/src/lib.rs) |
| `Прочитать`, `Read` | — | [bsl-textdoc/src/lib.rs](../../../crates/bsl-textdoc/src/lib.rs) |
| `Записать`, `Write` | — | [bsl-textdoc/src/lib.rs](../../../crates/bsl-textdoc/src/lib.rs) |
| `ПолучитьОбласть`, `GetArea` | — | [bsl-textdoc/src/lib.rs](../../../crates/bsl-textdoc/src/lib.rs) |
| `Вывести`, `Output` | — | [bsl-textdoc/src/lib.rs](../../../crates/bsl-textdoc/src/lib.rs) |

##### `ПараметрыМакетаТекстовогоДокумента`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

### `bsl-xml` 0.3.0

#### Глобальные свойства

| Имя и псевдонимы | Функция получения |
|---|---|
| `СериализаторXDTO`, `XDTOSerializer` | `ПолучитьСериализаторXDTO` |
| `ОбработкаСтрокиXML`, `XMLStringProcessing` | `ПолучитьОбработкуСтрокиXML` |

#### Глобальные функции

| Имя и псевдонимы | Вид | Описание | Реализация |
|---|---|---|---|
| `СоздатьФабрикуXDTO`, `CreateXDTOFactory` | функция | — | [bsl-xml/src/lib.rs](../../../crates/bsl-xml/src/lib.rs) |
| `ФабрикаXDTO`, `XDTOFactory` | функция | — | [bsl-xml/src/lib.rs](../../../crates/bsl-xml/src/lib.rs) |
| `XMLСтрока`, `XMLString` | функция | — | [bsl-xml/src/lib.rs](../../../crates/bsl-xml/src/lib.rs) |
| `НайтиНедопустимыеСимволыXML`, `FindDisallowedXMLCharacters` | функция | — | [bsl-xml/src/lib.rs](../../../crates/bsl-xml/src/lib.rs) |
| `XMLТипЗнч`, `XMLTypeOf` | функция | — | [bsl-xml/src/lib.rs](../../../crates/bsl-xml/src/lib.rs) |
| `ПолучитьСериализаторXDTO` | функция | — | [bsl-xml/src/lib.rs](../../../crates/bsl-xml/src/lib.rs) |
| `ПолучитьОбработкуСтрокиXML` | функция | — | [bsl-xml/src/lib.rs](../../../crates/bsl-xml/src/lib.rs) |
| `XMLЗначение`, `XMLValue` | функция | — | [bsl-xml/src/lib.rs](../../../crates/bsl-xml/src/lib.rs) |
| `ЗаписатьXML`, `WriteXML` — *объявлено без реализации* | процедура | — | [bsl-xml/src/lib.rs](../../../crates/bsl-xml/src/lib.rs) |
| `ПрочитатьXML`, `ReadXML` — *объявлено без реализации* | функция | — | [bsl-xml/src/lib.rs](../../../crates/bsl-xml/src/lib.rs) |
| `ВозможностьЧтенияXML`, `CanReadXML` — *объявлено без реализации* | функция | — | [bsl-xml/src/lib.rs](../../../crates/bsl-xml/src/lib.rs) |
| `XMLТип`, `XMLType` — *объявлено без реализации* | функция | — | [bsl-xml/src/lib.rs](../../../crates/bsl-xml/src/lib.rs) |
| `ИзXMLТипа`, `FromXMLType` | функция | — | [bsl-xml/src/lib.rs](../../../crates/bsl-xml/src/lib.rs) |
| `ПолучитьXMLТип`, `GetXMLType` | функция | — | [bsl-xml/src/lib.rs](../../../crates/bsl-xml/src/lib.rs) |

#### Конструкторы

| Имя и псевдонимы | Описание | Реализация |
|---|---|---|
| `ФабрикаXDTO`, `XDTOFactory` | — | [bsl-xml/src/lib.rs](../../../crates/bsl-xml/src/lib.rs) |
| `СериализаторXDTO`, `XDTOSerializer` | — | [bsl-xml/src/lib.rs](../../../crates/bsl-xml/src/lib.rs) |
| `ПостроительСхемXML`, `XMLSchemaBuilder` | — | [bsl-xml/src/lib.rs](../../../crates/bsl-xml/src/lib.rs) |
| `СхемаXML`, `XMLSchema` | — | [bsl-xml/src/lib.rs](../../../crates/bsl-xml/src/lib.rs) |
| `НаборСхемXML`, `XMLSchemaSet` | — | [bsl-xml/src/lib.rs](../../../crates/bsl-xml/src/lib.rs) |
| `РасширенноеИмяXML` | — | [bsl-xml/src/lib.rs](../../../crates/bsl-xml/src/lib.rs) |
| `ПостроительDOM`, `DOMBuilder` | — | [bsl-xml/src/lib.rs](../../../crates/bsl-xml/src/lib.rs) |
| `ДокументDOM`, `DOMDocument` | — | [bsl-xml/src/lib.rs](../../../crates/bsl-xml/src/lib.rs) |
| `ЗаписьDOM`, `DOMWriter` | — | [bsl-xml/src/lib.rs](../../../crates/bsl-xml/src/lib.rs) |
| `РазыменовательПространствИменDOM`, `DOMNamespaceResolver` | — | [bsl-xml/src/lib.rs](../../../crates/bsl-xml/src/lib.rs) |
| `ЧтениеXML`, `XMLReader` | — | [bsl-xml/src/lib.rs](../../../crates/bsl-xml/src/lib.rs) |
| `ЗаписьXML`, `XMLWriter` | — | [bsl-xml/src/lib.rs](../../../crates/bsl-xml/src/lib.rs) |
| `ПараметрыЗаписиXML`, `XMLWriterSettings` | — | [bsl-xml/src/lib.rs](../../../crates/bsl-xml/src/lib.rs) |
| `ТипДанныхXML`, `XMLDataType` | — | [bsl-xml/src/lib.rs](../../../crates/bsl-xml/src/lib.rs) |
| `ПараметрыЧтенияXML`, `XMLReaderSettings` — *объявлено без реализации* | — | [bsl-xml/src/lib.rs](../../../crates/bsl-xml/src/lib.rs) |
| `ЗаписьFastInfoset`, `FastInfosetWriter` — *объявлено без реализации* | — | [bsl-xml/src/lib.rs](../../../crates/bsl-xml/src/lib.rs) |
| `ЧтениеFastInfoset`, `FastInfosetReader` — *объявлено без реализации* | — | [bsl-xml/src/lib.rs](../../../crates/bsl-xml/src/lib.rs) |
| `ПреобразованиеXSL`, `XSLTransform` — *объявлено без реализации* | — | [bsl-xml/src/lib.rs](../../../crates/bsl-xml/src/lib.rs) |
| `ДокументHTML`, `HTMLDocument` | — | [bsl-xml/src/lib.rs](../../../crates/bsl-xml/src/lib.rs) |
| `ЗаписьHTML`, `HTMLWriter` | — | [bsl-xml/src/lib.rs](../../../crates/bsl-xml/src/lib.rs) |
| `ПреобразованиеККаноническомуXML`, `ConversionToCanonicalXML` | — | [bsl-xml/src/canonicalization.rs](../../../crates/bsl-xml/src/canonicalization.rs) |
| `КаноническаяЗаписьXML`, `XMLCanonicalizingWriter` | — | [bsl-xml/src/legacy_canonicalization.rs](../../../crates/bsl-xml/src/legacy_canonicalization.rs) |
| `КаноническийDOM`, `DOMCanonicalization` | — | [bsl-xml/src/legacy_canonicalization.rs](../../../crates/bsl-xml/src/legacy_canonicalization.rs) |

#### Типы объектов

| Имя | Представление типа | Дополнительные имена | Описание | Реализация |
|---|---|---|---|---|
| `КаноническаяЗаписьXML` | `Каноническая Запись XML` | `XMLCanonicalizingWriter` | — | [bsl-xml/src/legacy_canonicalization.rs](../../../crates/bsl-xml/src/legacy_canonicalization.rs) |
| `КаноническийDOM` | `Каноническая Запись Узла DOM` | `DOMCanonicalization` | — | [bsl-xml/src/legacy_canonicalization.rs](../../../crates/bsl-xml/src/legacy_canonicalization.rs) |
| `ПреобразованиеККаноническомуXML` | `Каноническая Запись XML Документа DOM` | `ConversionToCanonicalXML` | — | [bsl-xml/src/canonicalization.rs](../../../crates/bsl-xml/src/canonicalization.rs) |
| `ДокументHTML` | `Документ HTML` | `HTMLDocument` | — | [bsl-xml/src/dom/html.rs](../../../crates/bsl-xml/src/dom/html.rs) |
| `ОпределениеТипаДокументаDOM` | `Определение типа документа DOM` | `DOMDocumentType` | — | [bsl-xml/src/dom/html.rs](../../../crates/bsl-xml/src/dom/html.rs) |
| `ЗаписьHTML` | `Запись HTML` | `HTMLWriter` | — | [bsl-xml/src/dom/html.rs](../../../crates/bsl-xml/src/dom/html.rs) |
| `ЭлементТелоHTML` | `Элемент тело HTML` | `HTMLBodyElement` | — | [bsl-xml/src/dom/html.rs](../../../crates/bsl-xml/src/dom/html.rs) |
| `ЭлементРазметкаHTML` | `Элемент разметка HTML` | `HTMLHtmlElement` | — | [bsl-xml/src/dom/html.rs](../../../crates/bsl-xml/src/dom/html.rs) |
| `ЭлементЗаголовокHTML` | `Элемент заголовок HTML` | `HTMLHeadElement` | — | [bsl-xml/src/dom/html.rs](../../../crates/bsl-xml/src/dom/html.rs) |
| `ЭлементБлокHTML` | `Элемент блок HTML` | `HTMLDivElement` | — | [bsl-xml/src/dom/html.rs](../../../crates/bsl-xml/src/dom/html.rs) |
| `ЭлементHTML` | `Элемент HTML` | `HTMLElement` | — | [bsl-xml/src/dom/html.rs](../../../crates/bsl-xml/src/dom/html.rs) |
| `ТипДанныхXML` | `Тип данных XML` | `XMLDataType` | — | [bsl-xml/src/xml_data_type.rs](../../../crates/bsl-xml/src/xml_data_type.rs) |
| `МенеджерОбработкиСтрокиXML` | `Менеджер обработки строки XML` | `XMLStringProcessingManager` | — | [bsl-xml/src/xml_string_processing.rs](../../../crates/bsl-xml/src/xml_string_processing.rs) |
| `АннотацияXS` | `Аннотация XML Schema` | `XSAnnotation` | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |
| `ИнформацияДляПриложенияXS` | `Информация для приложения XML Schema` | `XSAppInfo` | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |
| `ОбъявлениеАтрибутаXS` | `Объявление атрибута XML Schema` | `XSAttributeDeclaration` | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |
| `ИспользованиеАтрибутаXS` | `Использование атрибута XML Schema` | `XSAttributeUse` | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |
| `ПостроительСхемXML` | `Построитель схем XML` | `XMLSchemaBuilder` | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |
| `ОпределениеСоставногоТипаXS` | `Определение составного типа XML Schema` | `XSComplexTypeDefinition` | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |
| `ДокументацияXS` | `Документация XML Schema` | `XSDocumentation` | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |
| `АтрибутDOM` | `Атрибут DOM` | `DOMAttribute` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `КоллекцияАтрибутовDOM` | `Коллекция атрибутов DOM` | `DOMAttributeMap` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПостроительDOM` | `Построитель DOM` | `DOMBuilder` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СекцияCDATADOM` | `Секция CDATA DOM` | `DOMCDATASection` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `КомментарийDOM` | `Комментарий  DOM` | `DOMComment` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ДокументDOM` | `Документ  DOM` | `DOMDocument` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СписокЭлементовDOM` | `Список элементов DOM` | `DOMElementList` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЭлементDOM` | `Элемент DOM` | `DOMElement` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СсылкаНаСущностьDOM` | `Ссылка на сущность DOM` | `DOMEntityReference` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СписокУзловDOM` | `Список узлов DOM` | `DOMNodeList` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ИнструкцияОбработкиDOM` | `Инструкция обработки DOM` | `DOMProcessingInstruction` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ТекстDOM` | `Текст DOM` | `DOMText` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЗаписьDOM` | `Запись DOM` | `DOMWriter` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ОбъявлениеЭлементаXS` | `Объявление элемента XML Schema` | `XSElementDeclaration` | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |
| `РасширенноеИмяXML` | `Расширенное имя XML` | `XMLExpandedName` | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |
| `ВыражениеXPath` | `Выражение DOM XPath` | `XPathExpression` | — | [bsl-xml/src/xpath.rs](../../../crates/bsl-xml/src/xpath.rs) |
| `КоллекцияФасетовXDTO` | `Коллекция фасетов XDTO` | `XDTOFacetCollection` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ФасетПеречисленияXS` | `Фасет перечисления значения XML Schema` | `XSEnumerationFacet` | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |
| `ФасетКоличестваРазрядовДробнойЧастиXS` | `Фасет количества разрядов дробной части значения XML Schema` | `XSFractionDigitsFacet` | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |
| `ФасетДлиныXS` | `Фасет длины значения XML Schema` | `XSLengthFacet` | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |
| `ФасетМаксимальногоИсключающегоЗначенияXS` | `Фасет максимального исключающего значения XML Schema` | `XSMaxExclusiveFacet` | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |
| `ФасетМаксимальногоВключающегоЗначенияXS` | `Фасет максимального включающего значения XML Schema` | `XSMaxInclusiveFacet` | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |
| `ФасетМаксимальнойДлиныXS` | `Фасет максимальной длины значения XML Schema` | `XSMaxLengthFacet` | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |
| `ФасетМинимальногоИсключающегоЗначенияXS` | `Фасет минимального исключающего значения XML Schema` | `XSMinExclusiveFacet` | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |
| `ФасетМинимальногоВключающегоЗначенияXS` | `Фасет минимального включающего значения XML Schema` | `XSMinInclusiveFacet` | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |
| `ФасетМинимальнойДлиныXS` | `Фасет минимальной длины значения XML Schema` | `XSMinLengthFacet` | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |
| `ФасетОбразцаXS` | `Фасет образца значения XML Schema` | `XSPatternFacet` | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |
| `ФасетОбщегоКоличестваРазрядовXS` | `Фасет общего количества разрядов значения XML Schema` | `XSTotalDigitsFacet` | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |
| `ФасетXDTO` | `Фасет XDTO` | `XDTOFacet` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ФасетПробельныхСимволовXS` | `Фасет пробельных символов XML Schema` | `XSWhitespaceFacet` | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |
| `ФабрикаXDTO` | `Фабрика XDTO` | `XDTOFactory` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ФиксированныйСписокКомпонентXS` | `Фиксированный список компонент XML Schema` | `XSComponentFixedList` | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |
| `КоллекцияИменованныхКомпонентXS` | `Коллекция именованных компонент XML Schema` | `XSNamedComponentMap` | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |
| `СписокКомпонентXS` | `Список компонент XML Schema` | `XSComponentList` | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |
| `СписокXDTO` | `Список XDTO` | `XDTOList` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ГруппаМоделиXS` | `Группа модели XML Schema` | `XSModelGroup` | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |
| `СписокРасширенныхИменXML` | `Список расширенных имен XML` | `XMLExpandedNameList` | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |
| `ОбъектXDTO` | `Объект XDTO` | `XDTODataObject` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ТипОбъектаXDTO` | `Тип объекта XDTO` | `XDTOObjectType` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ФрагментXS` | `Фрагмент XML Schema` | `XSParticle` | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |
| `КоллекцияСвойствXDTO` | `Коллекция свойств XDTO` | `XDTOPropertyCollection` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `СвойствоXDTO` | `Свойство XDTO` | `XDTOProperty` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ЧтениеXML` | `Чтение XML` | `XMLReader` | — | [bsl-xml/src/xml.rs](../../../crates/bsl-xml/src/xml.rs) |
| `РазыменовательПространствИменDOM` | `Разыменователь пространств имен DOM XPath` | `DOMNamespaceResolver` | — | [bsl-xml/src/xpath.rs](../../../crates/bsl-xml/src/xpath.rs) |
| `РезультатXPath` | `Результат DOM XPath` | `XPathResult` | — | [bsl-xml/src/xpath.rs](../../../crates/bsl-xml/src/xpath.rs) |
| `НаборСхемXML` | `Набор схем XML` | `XMLSchemaSet` | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |
| `СхемаXML` | `Схема XML` | `XMLSchema` | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |
| `ПоследовательностьXDTO` | `Последовательность XDTO` | `XDTOSequence` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `СериализаторXDTO` | `Сериализатор XDTO` | `XDTOSerializer` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПараметрыЗаписиXML` | `Параметры записи XML` | `XMLWriterSettings` | — | [bsl-xml/src/xml.rs](../../../crates/bsl-xml/src/xml.rs) |
| `ОпределениеПростогоТипаXS` | `Определение простого типа XML Schema` | `XSSimpleTypeDefinition` | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |
| `ЗначениеXDTO` | `Значение XDTO` | `XDTODataValue` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ТипЗначенияXDTO` | `Тип значения XDTO` | `XDTOValueType` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ЗаписьXML` | `Запись XML` | `XMLWriter` | — | [bsl-xml/src/xml.rs](../../../crates/bsl-xml/src/xml.rs) |

#### Члены объектов

##### `КаноническаяЗаписьXML`

| Метод | Описание | Реализация |
|---|---|---|
| `УстановитьСтроку`, `SetString` | — | [bsl-xml/src/legacy_canonicalization.rs](../../../crates/bsl-xml/src/legacy_canonicalization.rs) |
| `Закрыть`, `Close` | — | [bsl-xml/src/legacy_canonicalization.rs](../../../crates/bsl-xml/src/legacy_canonicalization.rs) |

##### `КаноническийDOM`

| Метод | Описание | Реализация |
|---|---|---|
| `Канонизировать`, `Canonicalize` | — | [bsl-xml/src/legacy_canonicalization.rs](../../../crates/bsl-xml/src/legacy_canonicalization.rs) |

##### `ПреобразованиеККаноническомуXML`

| Метод | Описание | Реализация |
|---|---|---|
| `КанонизироватьВСтроку`, `CanonicalizeToString` | — | [bsl-xml/src/canonicalization.rs](../../../crates/bsl-xml/src/canonicalization.rs) |
| `КанонизироватьВФайл`, `CanonicalizeToFile` | — | [bsl-xml/src/canonicalization.rs](../../../crates/bsl-xml/src/canonicalization.rs) |
| `КанонизироватьВПоток`, `CanonicalizeToStream` | — | [bsl-xml/src/canonicalization.rs](../../../crates/bsl-xml/src/canonicalization.rs) |

##### `ДокументHTML`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `ИмяУзла`, `NodeName` | чтение и запись | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `РодительскийУзел`, `ParentNode` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ДочерниеУзлы`, `ChildNodes` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЭлементДокумента`, `DocumentElement` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Тело`, `Body` | чтение и запись | — | [bsl-xml/src/dom/html.rs](../../../crates/bsl-xml/src/dom/html.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `СоздатьЭлемент`, `CreateElement` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьТекстовыйУзел`, `CreateTextNode` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ДобавитьДочерний`, `AppendChild` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |

##### `ОпределениеТипаДокументаDOM`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `ИмяУзла`, `NodeName` | чтение и запись | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ТипУзла`, `NodeType` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `РодительскийУзел`, `ParentNode` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |

##### `ЗаписьHTML`

| Метод | Описание | Реализация |
|---|---|---|
| `УстановитьСтроку`, `SetString` | — | [bsl-xml/src/legacy_canonicalization.rs](../../../crates/bsl-xml/src/legacy_canonicalization.rs) |
| `Закрыть`, `Close` | — | [bsl-xml/src/legacy_canonicalization.rs](../../../crates/bsl-xml/src/legacy_canonicalization.rs) |

##### `ЭлементТелоHTML`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `ИмяУзла`, `NodeName` | чтение и запись | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ТипУзла`, `NodeType` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `РодительскийУзел`, `ParentNode` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ДочерниеУзлы`, `ChildNodes` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `ДобавитьДочерний`, `AppendChild` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |

##### `ЭлементРазметкаHTML`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `ИмяУзла`, `NodeName` | чтение и запись | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ТипУзла`, `NodeType` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `РодительскийУзел`, `ParentNode` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ДочерниеУзлы`, `ChildNodes` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `ДобавитьДочерний`, `AppendChild` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |

##### `ЭлементЗаголовокHTML`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `ИмяУзла`, `NodeName` | чтение и запись | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ТипУзла`, `NodeType` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `РодительскийУзел`, `ParentNode` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ДочерниеУзлы`, `ChildNodes` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `ДобавитьДочерний`, `AppendChild` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |

##### `ЭлементБлокHTML`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `ИмяУзла`, `NodeName` | чтение и запись | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ТипУзла`, `NodeType` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `РодительскийУзел`, `ParentNode` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ДочерниеУзлы`, `ChildNodes` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `ДобавитьДочерний`, `AppendChild` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |

##### `ЭлементHTML`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `ИмяУзла`, `NodeName` | чтение и запись | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ТипУзла`, `NodeType` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `РодительскийУзел`, `ParentNode` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ДочерниеУзлы`, `ChildNodes` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `ДобавитьДочерний`, `AppendChild` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |

##### `ТипДанныхXML`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `ИмяТипа`, `TypeName` | только чтение | — | [bsl-xml/src/xml_data_type.rs](../../../crates/bsl-xml/src/xml_data_type.rs) |
| `URIПространстваИмен`, `NamespaceURI` | только чтение | — | [bsl-xml/src/xml_data_type.rs](../../../crates/bsl-xml/src/xml_data_type.rs) |

##### `МенеджерОбработкиСтрокиXML`

| Метод | Описание | Реализация |
|---|---|---|
| `НайтиНедопустимыеСимволыXML`, `FindDisallowedXMLCharacters` | — | [bsl-xml/src/xml_string_processing.rs](../../../crates/bsl-xml/src/xml_string_processing.rs) |

##### `АннотацияXS`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

##### `ИнформацияДляПриложенияXS`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

##### `ОбъявлениеАтрибутаXS`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

##### `ИспользованиеАтрибутаXS`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

##### `ПостроительСхемXML`

| Метод | Описание | Реализация |
|---|---|---|
| `СоздатьСхемуXML`, `CreateXMLSchema` | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |

##### `ОпределениеСоставногоТипаXS`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

##### `ДокументацияXS`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

##### `АтрибутDOM`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `ИмяУзла`, `NodeName` | чтение и запись | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ТипУзла`, `NodeType` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЗначениеУзла`, `NodeValue` | чтение и запись | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ТекстовоеСодержимое`, `TextContent` | чтение и запись | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЛокальноеИмя`, `LocalName` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Префикс`, `Prefix` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `URIПространстваИмен`, `NamespaceURI` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `РодительскийУзел`, `ParentNode` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ДокументВладелец`, `OwnerDocument` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПервыйДочерний`, `FirstChild` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПоследнийДочерний`, `LastChild` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СледующийСоседний`, `NextSibling` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПредыдущийСоседний`, `PreviousSibling` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ДочерниеУзлы`, `ChildNodes` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Атрибуты`, `Attributes` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЭлементДокумента`, `DocumentElement` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ВерсияXML`, `XmlVersion` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Имя`, `Name` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Значение`, `Value` | чтение и запись | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЭлементВладелец`, `OwnerElement` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Данные`, `Data` | чтение и запись | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Цель`, `Target` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `ЕстьДочерниеУзлы`, `HasChildNodes` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЕстьАтрибуты`, `HasAttributes` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПолучитьАтрибут`, `GetAttribute` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЕстьАтрибут`, `HasAttribute` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПолучитьУзелАтрибута`, `GetAttributeNode` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПолучитьЭлементыПоИмени`, `GetElementByTagName` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПолучитьЭлементПоИдентификатору`, `GetElementById` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьЭлемент`, `CreateElement` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьАтрибут`, `CreateAttribute` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьТекстовыйУзел`, `CreateTextNode` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьСекциюCDATA`, `CreateCDATASection` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьКомментарий`, `CreateComment` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьИнструкциюОбработки`, `CreateProcessingInstruction` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ДобавитьДочерний`, `AppendChild` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ВставитьПеред`, `InsertBefore` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `УдалитьДочерний`, `RemoveChild` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЗаменитьДочерний`, `ReplaceChild` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `УстановитьАтрибут`, `SetAttribute` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `УдалитьАтрибут`, `RemoveAttribute` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `УстановитьУзелАтрибута`, `SetAttributeNode` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `УдалитьУзелАтрибута`, `RemoveAttributeNode` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ВычислитьВыражениеXPath`, `EvaluateXPathExpression` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьВыражениеXPath`, `CreateXPathExpression` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьРазыменовательПИ`, `CreateNSResolver` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |

##### `КоллекцияАтрибутовDOM`

| Метод | Описание | Реализация |
|---|---|---|
| `Количество`, `Count` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Получить`, `Get` | `Получить(i)` — тот же путь, что `i` (измерено у списков DOM). | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |

##### `ПостроительDOM`

| Метод | Описание | Реализация |
|---|---|---|
| `Прочитать`, `Read` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |

##### `СекцияCDATADOM`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `ИмяУзла`, `NodeName` | чтение и запись | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ТипУзла`, `NodeType` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЗначениеУзла`, `NodeValue` | чтение и запись | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ТекстовоеСодержимое`, `TextContent` | чтение и запись | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЛокальноеИмя`, `LocalName` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Префикс`, `Prefix` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `URIПространстваИмен`, `NamespaceURI` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `РодительскийУзел`, `ParentNode` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ДокументВладелец`, `OwnerDocument` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПервыйДочерний`, `FirstChild` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПоследнийДочерний`, `LastChild` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СледующийСоседний`, `NextSibling` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПредыдущийСоседний`, `PreviousSibling` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ДочерниеУзлы`, `ChildNodes` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Атрибуты`, `Attributes` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЭлементДокумента`, `DocumentElement` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ВерсияXML`, `XmlVersion` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Имя`, `Name` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Значение`, `Value` | чтение и запись | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЭлементВладелец`, `OwnerElement` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Данные`, `Data` | чтение и запись | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Цель`, `Target` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `ЕстьДочерниеУзлы`, `HasChildNodes` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЕстьАтрибуты`, `HasAttributes` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПолучитьАтрибут`, `GetAttribute` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЕстьАтрибут`, `HasAttribute` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПолучитьУзелАтрибута`, `GetAttributeNode` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПолучитьЭлементыПоИмени`, `GetElementByTagName` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПолучитьЭлементПоИдентификатору`, `GetElementById` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьЭлемент`, `CreateElement` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьАтрибут`, `CreateAttribute` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьТекстовыйУзел`, `CreateTextNode` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьСекциюCDATA`, `CreateCDATASection` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьКомментарий`, `CreateComment` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьИнструкциюОбработки`, `CreateProcessingInstruction` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ДобавитьДочерний`, `AppendChild` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ВставитьПеред`, `InsertBefore` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `УдалитьДочерний`, `RemoveChild` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЗаменитьДочерний`, `ReplaceChild` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `УстановитьАтрибут`, `SetAttribute` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `УдалитьАтрибут`, `RemoveAttribute` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `УстановитьУзелАтрибута`, `SetAttributeNode` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `УдалитьУзелАтрибута`, `RemoveAttributeNode` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ВычислитьВыражениеXPath`, `EvaluateXPathExpression` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьВыражениеXPath`, `CreateXPathExpression` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьРазыменовательПИ`, `CreateNSResolver` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |

##### `КомментарийDOM`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `ИмяУзла`, `NodeName` | чтение и запись | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ТипУзла`, `NodeType` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЗначениеУзла`, `NodeValue` | чтение и запись | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ТекстовоеСодержимое`, `TextContent` | чтение и запись | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЛокальноеИмя`, `LocalName` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Префикс`, `Prefix` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `URIПространстваИмен`, `NamespaceURI` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `РодительскийУзел`, `ParentNode` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ДокументВладелец`, `OwnerDocument` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПервыйДочерний`, `FirstChild` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПоследнийДочерний`, `LastChild` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СледующийСоседний`, `NextSibling` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПредыдущийСоседний`, `PreviousSibling` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ДочерниеУзлы`, `ChildNodes` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Атрибуты`, `Attributes` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЭлементДокумента`, `DocumentElement` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ВерсияXML`, `XmlVersion` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Имя`, `Name` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Значение`, `Value` | чтение и запись | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЭлементВладелец`, `OwnerElement` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Данные`, `Data` | чтение и запись | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Цель`, `Target` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `ЕстьДочерниеУзлы`, `HasChildNodes` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЕстьАтрибуты`, `HasAttributes` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПолучитьАтрибут`, `GetAttribute` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЕстьАтрибут`, `HasAttribute` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПолучитьУзелАтрибута`, `GetAttributeNode` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПолучитьЭлементыПоИмени`, `GetElementByTagName` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПолучитьЭлементПоИдентификатору`, `GetElementById` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьЭлемент`, `CreateElement` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьАтрибут`, `CreateAttribute` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьТекстовыйУзел`, `CreateTextNode` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьСекциюCDATA`, `CreateCDATASection` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьКомментарий`, `CreateComment` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьИнструкциюОбработки`, `CreateProcessingInstruction` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ДобавитьДочерний`, `AppendChild` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ВставитьПеред`, `InsertBefore` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `УдалитьДочерний`, `RemoveChild` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЗаменитьДочерний`, `ReplaceChild` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `УстановитьАтрибут`, `SetAttribute` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `УдалитьАтрибут`, `RemoveAttribute` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `УстановитьУзелАтрибута`, `SetAttributeNode` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `УдалитьУзелАтрибута`, `RemoveAttributeNode` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ВычислитьВыражениеXPath`, `EvaluateXPathExpression` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьВыражениеXPath`, `CreateXPathExpression` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьРазыменовательПИ`, `CreateNSResolver` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |

##### `ДокументDOM`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `ИмяУзла`, `NodeName` | чтение и запись | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ТипУзла`, `NodeType` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЗначениеУзла`, `NodeValue` | чтение и запись | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ТекстовоеСодержимое`, `TextContent` | чтение и запись | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЛокальноеИмя`, `LocalName` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Префикс`, `Prefix` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `URIПространстваИмен`, `NamespaceURI` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `РодительскийУзел`, `ParentNode` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ДокументВладелец`, `OwnerDocument` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПервыйДочерний`, `FirstChild` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПоследнийДочерний`, `LastChild` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СледующийСоседний`, `NextSibling` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПредыдущийСоседний`, `PreviousSibling` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ДочерниеУзлы`, `ChildNodes` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Атрибуты`, `Attributes` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЭлементДокумента`, `DocumentElement` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ВерсияXML`, `XmlVersion` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Имя`, `Name` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Значение`, `Value` | чтение и запись | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЭлементВладелец`, `OwnerElement` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Данные`, `Data` | чтение и запись | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Цель`, `Target` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `ЕстьДочерниеУзлы`, `HasChildNodes` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЕстьАтрибуты`, `HasAttributes` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПолучитьАтрибут`, `GetAttribute` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЕстьАтрибут`, `HasAttribute` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПолучитьУзелАтрибута`, `GetAttributeNode` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПолучитьЭлементыПоИмени`, `GetElementByTagName` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПолучитьЭлементПоИдентификатору`, `GetElementById` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьЭлемент`, `CreateElement` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьАтрибут`, `CreateAttribute` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьТекстовыйУзел`, `CreateTextNode` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьСекциюCDATA`, `CreateCDATASection` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьКомментарий`, `CreateComment` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьИнструкциюОбработки`, `CreateProcessingInstruction` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ДобавитьДочерний`, `AppendChild` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ВставитьПеред`, `InsertBefore` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `УдалитьДочерний`, `RemoveChild` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЗаменитьДочерний`, `ReplaceChild` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `УстановитьАтрибут`, `SetAttribute` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `УдалитьАтрибут`, `RemoveAttribute` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `УстановитьУзелАтрибута`, `SetAttributeNode` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `УдалитьУзелАтрибута`, `RemoveAttributeNode` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ВычислитьВыражениеXPath`, `EvaluateXPathExpression` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьВыражениеXPath`, `CreateXPathExpression` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьРазыменовательПИ`, `CreateNSResolver` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |

##### `СписокЭлементовDOM`

| Метод | Описание | Реализация |
|---|---|---|
| `Количество`, `Count` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Получить`, `Get` | `Получить(i)` — тот же путь, что `i` (измерено у списков DOM). | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |

##### `ЭлементDOM`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `ИмяУзла`, `NodeName` | чтение и запись | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ТипУзла`, `NodeType` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЗначениеУзла`, `NodeValue` | чтение и запись | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ТекстовоеСодержимое`, `TextContent` | чтение и запись | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЛокальноеИмя`, `LocalName` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Префикс`, `Prefix` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `URIПространстваИмен`, `NamespaceURI` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `РодительскийУзел`, `ParentNode` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ДокументВладелец`, `OwnerDocument` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПервыйДочерний`, `FirstChild` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПоследнийДочерний`, `LastChild` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СледующийСоседний`, `NextSibling` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПредыдущийСоседний`, `PreviousSibling` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ДочерниеУзлы`, `ChildNodes` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Атрибуты`, `Attributes` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЭлементДокумента`, `DocumentElement` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ВерсияXML`, `XmlVersion` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Имя`, `Name` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Значение`, `Value` | чтение и запись | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЭлементВладелец`, `OwnerElement` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Данные`, `Data` | чтение и запись | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Цель`, `Target` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `ЕстьДочерниеУзлы`, `HasChildNodes` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЕстьАтрибуты`, `HasAttributes` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПолучитьАтрибут`, `GetAttribute` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЕстьАтрибут`, `HasAttribute` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПолучитьУзелАтрибута`, `GetAttributeNode` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПолучитьЭлементыПоИмени`, `GetElementByTagName` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПолучитьЭлементПоИдентификатору`, `GetElementById` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьЭлемент`, `CreateElement` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьАтрибут`, `CreateAttribute` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьТекстовыйУзел`, `CreateTextNode` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьСекциюCDATA`, `CreateCDATASection` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьКомментарий`, `CreateComment` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьИнструкциюОбработки`, `CreateProcessingInstruction` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ДобавитьДочерний`, `AppendChild` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ВставитьПеред`, `InsertBefore` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `УдалитьДочерний`, `RemoveChild` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЗаменитьДочерний`, `ReplaceChild` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `УстановитьАтрибут`, `SetAttribute` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `УдалитьАтрибут`, `RemoveAttribute` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `УстановитьУзелАтрибута`, `SetAttributeNode` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `УдалитьУзелАтрибута`, `RemoveAttributeNode` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ВычислитьВыражениеXPath`, `EvaluateXPathExpression` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьВыражениеXPath`, `CreateXPathExpression` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьРазыменовательПИ`, `CreateNSResolver` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |

##### `СсылкаНаСущностьDOM`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `ИмяУзла`, `NodeName` | чтение и запись | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ТипУзла`, `NodeType` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЗначениеУзла`, `NodeValue` | чтение и запись | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ТекстовоеСодержимое`, `TextContent` | чтение и запись | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЛокальноеИмя`, `LocalName` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Префикс`, `Prefix` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `URIПространстваИмен`, `NamespaceURI` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `РодительскийУзел`, `ParentNode` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ДокументВладелец`, `OwnerDocument` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПервыйДочерний`, `FirstChild` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПоследнийДочерний`, `LastChild` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СледующийСоседний`, `NextSibling` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПредыдущийСоседний`, `PreviousSibling` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ДочерниеУзлы`, `ChildNodes` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Атрибуты`, `Attributes` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЭлементДокумента`, `DocumentElement` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ВерсияXML`, `XmlVersion` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Имя`, `Name` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Значение`, `Value` | чтение и запись | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЭлементВладелец`, `OwnerElement` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Данные`, `Data` | чтение и запись | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Цель`, `Target` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `ЕстьДочерниеУзлы`, `HasChildNodes` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЕстьАтрибуты`, `HasAttributes` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПолучитьАтрибут`, `GetAttribute` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЕстьАтрибут`, `HasAttribute` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПолучитьУзелАтрибута`, `GetAttributeNode` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПолучитьЭлементыПоИмени`, `GetElementByTagName` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПолучитьЭлементПоИдентификатору`, `GetElementById` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьЭлемент`, `CreateElement` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьАтрибут`, `CreateAttribute` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьТекстовыйУзел`, `CreateTextNode` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьСекциюCDATA`, `CreateCDATASection` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьКомментарий`, `CreateComment` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьИнструкциюОбработки`, `CreateProcessingInstruction` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ДобавитьДочерний`, `AppendChild` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ВставитьПеред`, `InsertBefore` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `УдалитьДочерний`, `RemoveChild` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЗаменитьДочерний`, `ReplaceChild` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `УстановитьАтрибут`, `SetAttribute` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `УдалитьАтрибут`, `RemoveAttribute` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `УстановитьУзелАтрибута`, `SetAttributeNode` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `УдалитьУзелАтрибута`, `RemoveAttributeNode` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ВычислитьВыражениеXPath`, `EvaluateXPathExpression` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьВыражениеXPath`, `CreateXPathExpression` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьРазыменовательПИ`, `CreateNSResolver` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |

##### `СписокУзловDOM`

| Метод | Описание | Реализация |
|---|---|---|
| `Количество`, `Count` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Получить`, `Get` | `Получить(i)` — тот же путь, что `i` (измерено у списков DOM). | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |

##### `ИнструкцияОбработкиDOM`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `ИмяУзла`, `NodeName` | чтение и запись | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ТипУзла`, `NodeType` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЗначениеУзла`, `NodeValue` | чтение и запись | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ТекстовоеСодержимое`, `TextContent` | чтение и запись | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЛокальноеИмя`, `LocalName` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Префикс`, `Prefix` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `URIПространстваИмен`, `NamespaceURI` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `РодительскийУзел`, `ParentNode` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ДокументВладелец`, `OwnerDocument` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПервыйДочерний`, `FirstChild` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПоследнийДочерний`, `LastChild` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СледующийСоседний`, `NextSibling` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПредыдущийСоседний`, `PreviousSibling` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ДочерниеУзлы`, `ChildNodes` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Атрибуты`, `Attributes` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЭлементДокумента`, `DocumentElement` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ВерсияXML`, `XmlVersion` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Имя`, `Name` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Значение`, `Value` | чтение и запись | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЭлементВладелец`, `OwnerElement` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Данные`, `Data` | чтение и запись | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Цель`, `Target` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `ЕстьДочерниеУзлы`, `HasChildNodes` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЕстьАтрибуты`, `HasAttributes` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПолучитьАтрибут`, `GetAttribute` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЕстьАтрибут`, `HasAttribute` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПолучитьУзелАтрибута`, `GetAttributeNode` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПолучитьЭлементыПоИмени`, `GetElementByTagName` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПолучитьЭлементПоИдентификатору`, `GetElementById` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьЭлемент`, `CreateElement` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьАтрибут`, `CreateAttribute` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьТекстовыйУзел`, `CreateTextNode` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьСекциюCDATA`, `CreateCDATASection` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьКомментарий`, `CreateComment` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьИнструкциюОбработки`, `CreateProcessingInstruction` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ДобавитьДочерний`, `AppendChild` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ВставитьПеред`, `InsertBefore` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `УдалитьДочерний`, `RemoveChild` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЗаменитьДочерний`, `ReplaceChild` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `УстановитьАтрибут`, `SetAttribute` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `УдалитьАтрибут`, `RemoveAttribute` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `УстановитьУзелАтрибута`, `SetAttributeNode` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `УдалитьУзелАтрибута`, `RemoveAttributeNode` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ВычислитьВыражениеXPath`, `EvaluateXPathExpression` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьВыражениеXPath`, `CreateXPathExpression` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьРазыменовательПИ`, `CreateNSResolver` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |

##### `ТекстDOM`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `ИмяУзла`, `NodeName` | чтение и запись | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ТипУзла`, `NodeType` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЗначениеУзла`, `NodeValue` | чтение и запись | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ТекстовоеСодержимое`, `TextContent` | чтение и запись | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЛокальноеИмя`, `LocalName` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Префикс`, `Prefix` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `URIПространстваИмен`, `NamespaceURI` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `РодительскийУзел`, `ParentNode` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ДокументВладелец`, `OwnerDocument` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПервыйДочерний`, `FirstChild` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПоследнийДочерний`, `LastChild` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СледующийСоседний`, `NextSibling` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПредыдущийСоседний`, `PreviousSibling` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ДочерниеУзлы`, `ChildNodes` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Атрибуты`, `Attributes` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЭлементДокумента`, `DocumentElement` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ВерсияXML`, `XmlVersion` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Имя`, `Name` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Значение`, `Value` | чтение и запись | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЭлементВладелец`, `OwnerElement` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Данные`, `Data` | чтение и запись | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `Цель`, `Target` | только чтение | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `ЕстьДочерниеУзлы`, `HasChildNodes` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЕстьАтрибуты`, `HasAttributes` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПолучитьАтрибут`, `GetAttribute` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЕстьАтрибут`, `HasAttribute` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПолучитьУзелАтрибута`, `GetAttributeNode` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПолучитьЭлементыПоИмени`, `GetElementByTagName` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ПолучитьЭлементПоИдентификатору`, `GetElementById` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьЭлемент`, `CreateElement` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьАтрибут`, `CreateAttribute` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьТекстовыйУзел`, `CreateTextNode` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьСекциюCDATA`, `CreateCDATASection` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьКомментарий`, `CreateComment` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьИнструкциюОбработки`, `CreateProcessingInstruction` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ДобавитьДочерний`, `AppendChild` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ВставитьПеред`, `InsertBefore` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `УдалитьДочерний`, `RemoveChild` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ЗаменитьДочерний`, `ReplaceChild` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `УстановитьАтрибут`, `SetAttribute` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `УдалитьАтрибут`, `RemoveAttribute` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `УстановитьУзелАтрибута`, `SetAttributeNode` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `УдалитьУзелАтрибута`, `RemoveAttributeNode` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `ВычислитьВыражениеXPath`, `EvaluateXPathExpression` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьВыражениеXPath`, `CreateXPathExpression` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |
| `СоздатьРазыменовательПИ`, `CreateNSResolver` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |

##### `ЗаписьDOM`

| Метод | Описание | Реализация |
|---|---|---|
| `Записать`, `Write` | — | [bsl-xml/src/dom/objects.rs](../../../crates/bsl-xml/src/dom/objects.rs) |

##### `ОбъявлениеЭлементаXS`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

##### `РасширенноеИмяXML`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `ЛокальноеИмя`, `LocalName` | только чтение | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |
| `URIПространстваИмен`, `NamespaceURI` | только чтение | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |

##### `ВыражениеXPath`

| Метод | Описание | Реализация |
|---|---|---|
| `Вычислить`, `Evaluate` | — | [bsl-xml/src/xpath.rs](../../../crates/bsl-xml/src/xpath.rs) |

##### `КоллекцияФасетовXDTO`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

| Метод | Описание | Реализация |
|---|---|---|
| `Тип`, `Type` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Создать`, `Create` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПрочитатьXML`, `ReadXML` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ЗаписатьXML`, `WriteXML` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Получить`, `Get` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Установить`, `Set` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Добавить`, `Add` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Вставить`, `Insert` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Удалить`, `Delete` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Очистить`, `Clear` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Количество`, `Count` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПолучитьСписок`, `GetList` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Установлено`, `IsSet` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Сбросить`, `Unset` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Проверить`, `Validate` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Свойства`, `Properties` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Владелец`, `Owner` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Последовательность`, `Sequence` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПолучитьЗначение`, `GetValue` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПолучитьСвойство`, `GetProperty` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `XMLТип`, `XMLType` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `XMLТипЗнч`, `XMLTypeOf` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ВозможностьЧтенияXML`, `CanReadXML` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |

##### `ФасетПеречисленияXS`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

##### `ФасетКоличестваРазрядовДробнойЧастиXS`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

##### `ФасетДлиныXS`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

##### `ФасетМаксимальногоИсключающегоЗначенияXS`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

##### `ФасетМаксимальногоВключающегоЗначенияXS`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

##### `ФасетМаксимальнойДлиныXS`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

##### `ФасетМинимальногоИсключающегоЗначенияXS`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

##### `ФасетМинимальногоВключающегоЗначенияXS`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

##### `ФасетМинимальнойДлиныXS`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

##### `ФасетОбразцаXS`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

##### `ФасетОбщегоКоличестваРазрядовXS`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

##### `ФасетXDTO`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

| Метод | Описание | Реализация |
|---|---|---|
| `Тип`, `Type` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Создать`, `Create` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПрочитатьXML`, `ReadXML` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ЗаписатьXML`, `WriteXML` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Получить`, `Get` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Установить`, `Set` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Добавить`, `Add` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Вставить`, `Insert` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Удалить`, `Delete` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Очистить`, `Clear` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Количество`, `Count` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПолучитьСписок`, `GetList` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Установлено`, `IsSet` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Сбросить`, `Unset` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Проверить`, `Validate` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Свойства`, `Properties` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Владелец`, `Owner` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Последовательность`, `Sequence` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПолучитьЗначение`, `GetValue` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПолучитьСвойство`, `GetProperty` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `XMLТип`, `XMLType` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `XMLТипЗнч`, `XMLTypeOf` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ВозможностьЧтенияXML`, `CanReadXML` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |

##### `ФасетПробельныхСимволовXS`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

##### `ФабрикаXDTO`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

| Метод | Описание | Реализация |
|---|---|---|
| `Тип`, `Type` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Создать`, `Create` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПрочитатьXML`, `ReadXML` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ЗаписатьXML`, `WriteXML` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Получить`, `Get` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Установить`, `Set` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Добавить`, `Add` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Вставить`, `Insert` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Удалить`, `Delete` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Очистить`, `Clear` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Количество`, `Count` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПолучитьСписок`, `GetList` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Установлено`, `IsSet` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Сбросить`, `Unset` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Проверить`, `Validate` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Свойства`, `Properties` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Владелец`, `Owner` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Последовательность`, `Sequence` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПолучитьЗначение`, `GetValue` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПолучитьСвойство`, `GetProperty` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `XMLТип`, `XMLType` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `XMLТипЗнч`, `XMLTypeOf` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ВозможностьЧтенияXML`, `CanReadXML` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |

##### `ФиксированныйСписокКомпонентXS`

| Метод | Описание | Реализация |
|---|---|---|
| `Получить`, `Get` | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |
| `Количество`, `Count` | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |

##### `КоллекцияИменованныхКомпонентXS`

| Метод | Описание | Реализация |
|---|---|---|
| `Получить`, `Get` | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |
| `Количество`, `Count` | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |

##### `СписокКомпонентXS`

| Метод | Описание | Реализация |
|---|---|---|
| `Получить`, `Get` | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |
| `Количество`, `Count` | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |

##### `СписокXDTO`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

| Метод | Описание | Реализация |
|---|---|---|
| `Тип`, `Type` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Создать`, `Create` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПрочитатьXML`, `ReadXML` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ЗаписатьXML`, `WriteXML` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Получить`, `Get` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Установить`, `Set` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Добавить`, `Add` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Вставить`, `Insert` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Удалить`, `Delete` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Очистить`, `Clear` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Количество`, `Count` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПолучитьСписок`, `GetList` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Установлено`, `IsSet` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Сбросить`, `Unset` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Проверить`, `Validate` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Свойства`, `Properties` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Владелец`, `Owner` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Последовательность`, `Sequence` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПолучитьЗначение`, `GetValue` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПолучитьСвойство`, `GetProperty` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `XMLТип`, `XMLType` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `XMLТипЗнч`, `XMLTypeOf` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ВозможностьЧтенияXML`, `CanReadXML` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |

##### `ГруппаМоделиXS`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

##### `СписокРасширенныхИменXML`

| Метод | Описание | Реализация |
|---|---|---|
| `Количество`, `Count` | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |

##### `ОбъектXDTO`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

| Метод | Описание | Реализация |
|---|---|---|
| `Тип`, `Type` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Создать`, `Create` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПрочитатьXML`, `ReadXML` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ЗаписатьXML`, `WriteXML` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Получить`, `Get` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Установить`, `Set` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Добавить`, `Add` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Вставить`, `Insert` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Удалить`, `Delete` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Очистить`, `Clear` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Количество`, `Count` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПолучитьСписок`, `GetList` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Установлено`, `IsSet` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Сбросить`, `Unset` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Проверить`, `Validate` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Свойства`, `Properties` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Владелец`, `Owner` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Последовательность`, `Sequence` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПолучитьЗначение`, `GetValue` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПолучитьСвойство`, `GetProperty` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `XMLТип`, `XMLType` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `XMLТипЗнч`, `XMLTypeOf` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ВозможностьЧтенияXML`, `CanReadXML` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |

##### `ТипОбъектаXDTO`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

| Метод | Описание | Реализация |
|---|---|---|
| `Тип`, `Type` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Создать`, `Create` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПрочитатьXML`, `ReadXML` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ЗаписатьXML`, `WriteXML` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Получить`, `Get` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Установить`, `Set` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Добавить`, `Add` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Вставить`, `Insert` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Удалить`, `Delete` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Очистить`, `Clear` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Количество`, `Count` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПолучитьСписок`, `GetList` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Установлено`, `IsSet` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Сбросить`, `Unset` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Проверить`, `Validate` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Свойства`, `Properties` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Владелец`, `Owner` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Последовательность`, `Sequence` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПолучитьЗначение`, `GetValue` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПолучитьСвойство`, `GetProperty` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `XMLТип`, `XMLType` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `XMLТипЗнч`, `XMLTypeOf` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ВозможностьЧтенияXML`, `CanReadXML` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |

##### `ФрагментXS`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

##### `КоллекцияСвойствXDTO`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

| Метод | Описание | Реализация |
|---|---|---|
| `Тип`, `Type` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Создать`, `Create` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПрочитатьXML`, `ReadXML` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ЗаписатьXML`, `WriteXML` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Получить`, `Get` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Установить`, `Set` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Добавить`, `Add` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Вставить`, `Insert` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Удалить`, `Delete` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Очистить`, `Clear` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Количество`, `Count` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПолучитьСписок`, `GetList` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Установлено`, `IsSet` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Сбросить`, `Unset` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Проверить`, `Validate` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Свойства`, `Properties` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Владелец`, `Owner` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Последовательность`, `Sequence` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПолучитьЗначение`, `GetValue` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПолучитьСвойство`, `GetProperty` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `XMLТип`, `XMLType` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `XMLТипЗнч`, `XMLTypeOf` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ВозможностьЧтенияXML`, `CanReadXML` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |

##### `СвойствоXDTO`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

| Метод | Описание | Реализация |
|---|---|---|
| `Тип`, `Type` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Создать`, `Create` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПрочитатьXML`, `ReadXML` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ЗаписатьXML`, `WriteXML` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Получить`, `Get` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Установить`, `Set` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Добавить`, `Add` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Вставить`, `Insert` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Удалить`, `Delete` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Очистить`, `Clear` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Количество`, `Count` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПолучитьСписок`, `GetList` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Установлено`, `IsSet` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Сбросить`, `Unset` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Проверить`, `Validate` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Свойства`, `Properties` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Владелец`, `Owner` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Последовательность`, `Sequence` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПолучитьЗначение`, `GetValue` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПолучитьСвойство`, `GetProperty` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `XMLТип`, `XMLType` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `XMLТипЗнч`, `XMLTypeOf` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ВозможностьЧтенияXML`, `CanReadXML` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |

##### `ЧтениеXML`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `ТипУзла`, `NodeType` | только чтение | — | [bsl-xml/src/xml.rs](../../../crates/bsl-xml/src/xml.rs) |
| `Имя`, `Name` | только чтение | — | [bsl-xml/src/xml.rs](../../../crates/bsl-xml/src/xml.rs) |
| `Значение`, `Value` | только чтение | — | [bsl-xml/src/xml.rs](../../../crates/bsl-xml/src/xml.rs) |
| `ЛокальноеИмя`, `LocalName` | только чтение | — | [bsl-xml/src/xml.rs](../../../crates/bsl-xml/src/xml.rs) |
| `Префикс`, `Prefix` | только чтение | — | [bsl-xml/src/xml.rs](../../../crates/bsl-xml/src/xml.rs) |
| `URIПространстваИмен`, `NamespaceURI` | только чтение | — | [bsl-xml/src/xml.rs](../../../crates/bsl-xml/src/xml.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `УстановитьСтроку`, `SetString` | — | [bsl-xml/src/xml.rs](../../../crates/bsl-xml/src/xml.rs) |
| `ОткрытьФайл`, `OpenFile` | — | [bsl-xml/src/xml.rs](../../../crates/bsl-xml/src/xml.rs) |
| `Прочитать`, `Read` | — | [bsl-xml/src/xml.rs](../../../crates/bsl-xml/src/xml.rs) |
| `Пропустить`, `Skip` | — | [bsl-xml/src/xml.rs](../../../crates/bsl-xml/src/xml.rs) |
| `ПрочитатьАтрибут`, `ReadAttribute` | — | [bsl-xml/src/xml.rs](../../../crates/bsl-xml/src/xml.rs) |
| `КоличествоАтрибутов`, `AttributeCount` | — | [bsl-xml/src/xml.rs](../../../crates/bsl-xml/src/xml.rs) |
| `ИмяАтрибута`, `AttributeName` | — | [bsl-xml/src/xml.rs](../../../crates/bsl-xml/src/xml.rs) |
| `ЗначениеАтрибута`, `AttributeValue` | — | [bsl-xml/src/xml.rs](../../../crates/bsl-xml/src/xml.rs) |
| `ПерейтиКСодержимому`, `MoveToContent` | — | [bsl-xml/src/xml.rs](../../../crates/bsl-xml/src/xml.rs) |
| `Закрыть`, `Close` | — | [bsl-xml/src/xml.rs](../../../crates/bsl-xml/src/xml.rs) |

##### `РазыменовательПространствИменDOM`

| Метод | Описание | Реализация |
|---|---|---|
| `НайтиURIПространстваИмен`, `LookupNamespaceURI` | — | [bsl-xml/src/xpath.rs](../../../crates/bsl-xml/src/xpath.rs) |

##### `РезультатXPath`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `ТипРезультата`, `ResultType` | только чтение | — | [bsl-xml/src/xpath.rs](../../../crates/bsl-xml/src/xpath.rs) |
| `ЧисловоеЗначение`, `NumberValue` | только чтение | — | [bsl-xml/src/xpath.rs](../../../crates/bsl-xml/src/xpath.rs) |
| `СтроковоеЗначение`, `StringValue` | только чтение | — | [bsl-xml/src/xpath.rs](../../../crates/bsl-xml/src/xpath.rs) |
| `БулевоЗначение`, `BooleanValue` | только чтение | — | [bsl-xml/src/xpath.rs](../../../crates/bsl-xml/src/xpath.rs) |
| `РазмерСнимка`, `SnapshotLength` | только чтение | — | [bsl-xml/src/xpath.rs](../../../crates/bsl-xml/src/xpath.rs) |
| `НеверноеСостояниеИтератора`, `InvalidIteratorState` | только чтение | — | [bsl-xml/src/xpath.rs](../../../crates/bsl-xml/src/xpath.rs) |
| `SingleNodeValue` | только чтение | — | [bsl-xml/src/xpath.rs](../../../crates/bsl-xml/src/xpath.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `ПолучитьСледующий`, `IterateNext` | — | [bsl-xml/src/xpath.rs](../../../crates/bsl-xml/src/xpath.rs) |
| `ЭлементСнимка`, `SnapshotItem` | — | [bsl-xml/src/xpath.rs](../../../crates/bsl-xml/src/xpath.rs) |

##### `НаборСхемXML`

| Метод | Описание | Реализация |
|---|---|---|
| `Добавить`, `Add` | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |
| `Получить`, `Get` | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |
| `Количество`, `Count` | — | [bsl-xml/src/xsd/objects.rs](../../../crates/bsl-xml/src/xsd/objects.rs) |

##### `СхемаXML`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

##### `ПоследовательностьXDTO`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

| Метод | Описание | Реализация |
|---|---|---|
| `Тип`, `Type` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Создать`, `Create` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПрочитатьXML`, `ReadXML` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ЗаписатьXML`, `WriteXML` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Получить`, `Get` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Установить`, `Set` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Добавить`, `Add` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Вставить`, `Insert` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Удалить`, `Delete` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Очистить`, `Clear` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Количество`, `Count` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПолучитьСписок`, `GetList` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Установлено`, `IsSet` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Сбросить`, `Unset` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Проверить`, `Validate` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Свойства`, `Properties` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Владелец`, `Owner` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Последовательность`, `Sequence` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПолучитьЗначение`, `GetValue` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПолучитьСвойство`, `GetProperty` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `XMLТип`, `XMLType` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `XMLТипЗнч`, `XMLTypeOf` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ВозможностьЧтенияXML`, `CanReadXML` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |

##### `СериализаторXDTO`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

| Метод | Описание | Реализация |
|---|---|---|
| `Тип`, `Type` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Создать`, `Create` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПрочитатьXML`, `ReadXML` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ЗаписатьXML`, `WriteXML` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Получить`, `Get` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Установить`, `Set` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Добавить`, `Add` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Вставить`, `Insert` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Удалить`, `Delete` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Очистить`, `Clear` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Количество`, `Count` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПолучитьСписок`, `GetList` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Установлено`, `IsSet` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Сбросить`, `Unset` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Проверить`, `Validate` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Свойства`, `Properties` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Владелец`, `Owner` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Последовательность`, `Sequence` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПолучитьЗначение`, `GetValue` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПолучитьСвойство`, `GetProperty` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `XMLТип`, `XMLType` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `XMLТипЗнч`, `XMLTypeOf` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ВозможностьЧтенияXML`, `CanReadXML` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ИзXMLТипа`, `FromXMLType` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |

##### `ПараметрыЗаписиXML`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `Кодировка`, `Encoding` | только чтение | — | [bsl-xml/src/xml.rs](../../../crates/bsl-xml/src/xml.rs) |
| `Версия`, `Version` | только чтение | — | [bsl-xml/src/xml.rs](../../../crates/bsl-xml/src/xml.rs) |
| `Отступ`, `Indent` | только чтение | — | [bsl-xml/src/xml.rs](../../../crates/bsl-xml/src/xml.rs) |
| `ОтступАтрибутов`, `IndentAttributes` | только чтение | — | [bsl-xml/src/xml.rs](../../../crates/bsl-xml/src/xml.rs) |
| `СимволыОтступа`, `IndentChars` | только чтение | — | [bsl-xml/src/xml.rs](../../../crates/bsl-xml/src/xml.rs) |

##### `ОпределениеПростогоТипаXS`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

##### `ЗначениеXDTO`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

| Метод | Описание | Реализация |
|---|---|---|
| `Тип`, `Type` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Создать`, `Create` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПрочитатьXML`, `ReadXML` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ЗаписатьXML`, `WriteXML` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Получить`, `Get` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Установить`, `Set` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Добавить`, `Add` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Вставить`, `Insert` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Удалить`, `Delete` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Очистить`, `Clear` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Количество`, `Count` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПолучитьСписок`, `GetList` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Установлено`, `IsSet` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Сбросить`, `Unset` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Проверить`, `Validate` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Свойства`, `Properties` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Владелец`, `Owner` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Последовательность`, `Sequence` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПолучитьЗначение`, `GetValue` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПолучитьСвойство`, `GetProperty` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `XMLТип`, `XMLType` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `XMLТипЗнч`, `XMLTypeOf` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ВозможностьЧтенияXML`, `CanReadXML` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |

##### `ТипЗначенияXDTO`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

| Метод | Описание | Реализация |
|---|---|---|
| `Тип`, `Type` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Создать`, `Create` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПрочитатьXML`, `ReadXML` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ЗаписатьXML`, `WriteXML` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Получить`, `Get` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Установить`, `Set` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Добавить`, `Add` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Вставить`, `Insert` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Удалить`, `Delete` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Очистить`, `Clear` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Количество`, `Count` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПолучитьСписок`, `GetList` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Установлено`, `IsSet` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Сбросить`, `Unset` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Проверить`, `Validate` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Свойства`, `Properties` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Владелец`, `Owner` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `Последовательность`, `Sequence` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПолучитьЗначение`, `GetValue` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ПолучитьСвойство`, `GetProperty` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `XMLТип`, `XMLType` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `XMLТипЗнч`, `XMLTypeOf` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |
| `ВозможностьЧтенияXML`, `CanReadXML` | — | [bsl-xml/src/xdto/objects.rs](../../../crates/bsl-xml/src/xdto/objects.rs) |

##### `ЗаписьXML`

| Метод | Описание | Реализация |
|---|---|---|
| `УстановитьСтроку`, `SetString` | — | [bsl-xml/src/xml.rs](../../../crates/bsl-xml/src/xml.rs) |
| `ОткрытьФайл`, `OpenFile` | — | [bsl-xml/src/xml.rs](../../../crates/bsl-xml/src/xml.rs) |
| `ЗаписатьОбъявлениеXML`, `WriteXMLDeclaration` | — | [bsl-xml/src/xml.rs](../../../crates/bsl-xml/src/xml.rs) |
| `ЗаписатьНачалоЭлемента`, `WriteStartElement` | — | [bsl-xml/src/xml.rs](../../../crates/bsl-xml/src/xml.rs) |
| `ЗаписатьКонецЭлемента`, `WriteEndElement` | — | [bsl-xml/src/xml.rs](../../../crates/bsl-xml/src/xml.rs) |
| `ЗаписатьАтрибут`, `WriteAttribute` | — | [bsl-xml/src/xml.rs](../../../crates/bsl-xml/src/xml.rs) |
| `ЗаписатьТекст`, `WriteText` | — | [bsl-xml/src/xml.rs](../../../crates/bsl-xml/src/xml.rs) |
| `ЗаписатьКомментарий`, `WriteComment` | — | [bsl-xml/src/xml.rs](../../../crates/bsl-xml/src/xml.rs) |
| `ЗаписатьСекциюCDATA`, `WriteCDATASection` | — | [bsl-xml/src/xml.rs](../../../crates/bsl-xml/src/xml.rs) |
| `ЗаписатьИнструкциюОбработки`, `WriteProcessingInstruction` | — | [bsl-xml/src/xml.rs](../../../crates/bsl-xml/src/xml.rs) |
| `ЗаписатьБезОбработки`, `WriteRaw` | — | [bsl-xml/src/xml.rs](../../../crates/bsl-xml/src/xml.rs) |
| `Закрыть`, `Close` | — | [bsl-xml/src/xml.rs](../../../crates/bsl-xml/src/xml.rs) |

### `bsl-zip` 0.3.0

#### Глобальные функции

Нет.

#### Конструкторы

| Имя и псевдонимы | Описание | Реализация |
|---|---|---|
| `ЧтениеZipФайла`, `ZipFileReader` | — | [bsl-zip/src/lib.rs](../../../crates/bsl-zip/src/lib.rs) |
| `ЧтениеФайлаАрхива`, `ArchiveFileReader` | — | [bsl-zip/src/lib.rs](../../../crates/bsl-zip/src/lib.rs) |
| `ЗаписьZipФайла`, `ZipFileWriter` | — | [bsl-zip/src/lib.rs](../../../crates/bsl-zip/src/lib.rs) |
| `ЗаписьФайлаАрхива`, `ArchiveFileWriter` | — | [bsl-zip/src/lib.rs](../../../crates/bsl-zip/src/lib.rs) |

#### Типы объектов

| Имя | Представление типа | Дополнительные имена | Описание | Реализация |
|---|---|---|---|---|
| `ЭлементыФайлаАрхива` | `Элементы файла архива` | `ArchiveFileEntries` | — | [bsl-zip/src/archive/reader.rs](../../../crates/bsl-zip/src/archive/reader.rs) |
| `ЭлементФайлаАрхива` | `Элемент файла архива` | `ArchiveFileEntry` | — | [bsl-zip/src/archive/reader.rs](../../../crates/bsl-zip/src/archive/reader.rs) |
| `ЧтениеФайлаАрхива` | `Чтение файла архива` | `ArchiveFileReader` | — | [bsl-zip/src/archive/reader.rs](../../../crates/bsl-zip/src/archive/reader.rs) |
| `ЗаписьФайлаАрхива` | `Запись файла архива` | `ArchiveFileWriter` | — | [bsl-zip/src/archive/reader.rs](../../../crates/bsl-zip/src/archive/reader.rs) |
| `ЭлементыZipФайла` | `Элементы ZIP файла` | `ZipFileEntries` | — | [bsl-zip/src/archive/reader.rs](../../../crates/bsl-zip/src/archive/reader.rs) |
| `ЭлементZipФайла` | `Элемент ZIP файла` | `ZipFileEntry` | — | [bsl-zip/src/archive/reader.rs](../../../crates/bsl-zip/src/archive/reader.rs) |
| `ЧтениеZipФайла` | `Чтение ZIP файла` | `ZipFileReader` | — | [bsl-zip/src/archive/reader.rs](../../../crates/bsl-zip/src/archive/reader.rs) |
| `ЗаписьZipФайла` | `Запись ZIP файла` | `ZipFileWriter` | — | [bsl-zip/src/archive/reader.rs](../../../crates/bsl-zip/src/archive/reader.rs) |

#### Члены объектов

##### `ЭлементыФайлаАрхива`

| Метод | Описание | Реализация |
|---|---|---|
| `Количество`, `Count` | — | [bsl-zip/src/archive/objects.rs](../../../crates/bsl-zip/src/archive/objects.rs) |
| `Получить`, `Get` | — | [bsl-zip/src/archive/objects.rs](../../../crates/bsl-zip/src/archive/objects.rs) |
| `Найти`, `Find` | — | [bsl-zip/src/archive/objects.rs](../../../crates/bsl-zip/src/archive/objects.rs) |
| `Извлечь`, `Extract` | — | [bsl-zip/src/archive/objects.rs](../../../crates/bsl-zip/src/archive/objects.rs) |
| `ИзвлечьВсе`, `ExtractAll` | — | [bsl-zip/src/archive/objects.rs](../../../crates/bsl-zip/src/archive/objects.rs) |

##### `ЭлементФайлаАрхива`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

| Метод | Описание | Реализация |
|---|---|---|
| `Извлечь`, `Extract` | — | [bsl-zip/src/archive/objects.rs](../../../crates/bsl-zip/src/archive/objects.rs) |
| `ИзвлечьВсе`, `ExtractAll` | — | [bsl-zip/src/archive/objects.rs](../../../crates/bsl-zip/src/archive/objects.rs) |

##### `ЧтениеФайлаАрхива`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `Элементы`, `Items` | только чтение | — | [bsl-zip/src/archive/objects.rs](../../../crates/bsl-zip/src/archive/objects.rs) |
| `Комментарий`, `Comment` | только чтение | — | [bsl-zip/src/archive/objects.rs](../../../crates/bsl-zip/src/archive/objects.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `Открыть`, `Open` | — | [bsl-zip/src/archive/objects.rs](../../../crates/bsl-zip/src/archive/objects.rs) |
| `Закрыть`, `Close` | — | [bsl-zip/src/archive/objects.rs](../../../crates/bsl-zip/src/archive/objects.rs) |
| `Извлечь`, `Extract` | — | [bsl-zip/src/archive/objects.rs](../../../crates/bsl-zip/src/archive/objects.rs) |
| `ИзвлечьВсе`, `ExtractAll` | — | [bsl-zip/src/archive/objects.rs](../../../crates/bsl-zip/src/archive/objects.rs) |

##### `ЗаписьФайлаАрхива`

| Метод | Описание | Реализация |
|---|---|---|
| `Открыть`, `Open` | — | [bsl-zip/src/archive/objects.rs](../../../crates/bsl-zip/src/archive/objects.rs) |
| `Добавить`, `Add` | — | [bsl-zip/src/archive/objects.rs](../../../crates/bsl-zip/src/archive/objects.rs) |
| `Записать`, `Write` | — | [bsl-zip/src/archive/objects.rs](../../../crates/bsl-zip/src/archive/objects.rs) |
| `ПолучитьДвоичныеДанные`, `GetBinaryData` | — | [bsl-zip/src/archive/objects.rs](../../../crates/bsl-zip/src/archive/objects.rs) |

##### `ЭлементыZipФайла`

| Метод | Описание | Реализация |
|---|---|---|
| `Количество`, `Count` | — | [bsl-zip/src/archive/objects.rs](../../../crates/bsl-zip/src/archive/objects.rs) |
| `Получить`, `Get` | — | [bsl-zip/src/archive/objects.rs](../../../crates/bsl-zip/src/archive/objects.rs) |
| `Найти`, `Find` | — | [bsl-zip/src/archive/objects.rs](../../../crates/bsl-zip/src/archive/objects.rs) |
| `Извлечь`, `Extract` | — | [bsl-zip/src/archive/objects.rs](../../../crates/bsl-zip/src/archive/objects.rs) |
| `ИзвлечьВсе`, `ExtractAll` | — | [bsl-zip/src/archive/objects.rs](../../../crates/bsl-zip/src/archive/objects.rs) |

##### `ЭлементZipФайла`

Свойства зависят от экземпляра; ниже перечислена только статическая часть.

| Метод | Описание | Реализация |
|---|---|---|
| `Извлечь`, `Extract` | — | [bsl-zip/src/archive/objects.rs](../../../crates/bsl-zip/src/archive/objects.rs) |
| `ИзвлечьВсе`, `ExtractAll` | — | [bsl-zip/src/archive/objects.rs](../../../crates/bsl-zip/src/archive/objects.rs) |

##### `ЧтениеZipФайла`

| Свойство | Доступ | Описание | Реализация |
|---|---|---|---|
| `Элементы`, `Items` | только чтение | — | [bsl-zip/src/archive/objects.rs](../../../crates/bsl-zip/src/archive/objects.rs) |
| `Комментарий`, `Comment` | только чтение | — | [bsl-zip/src/archive/objects.rs](../../../crates/bsl-zip/src/archive/objects.rs) |

| Метод | Описание | Реализация |
|---|---|---|
| `Открыть`, `Open` | — | [bsl-zip/src/archive/objects.rs](../../../crates/bsl-zip/src/archive/objects.rs) |
| `Закрыть`, `Close` | — | [bsl-zip/src/archive/objects.rs](../../../crates/bsl-zip/src/archive/objects.rs) |
| `Извлечь`, `Extract` | — | [bsl-zip/src/archive/objects.rs](../../../crates/bsl-zip/src/archive/objects.rs) |
| `ИзвлечьВсе`, `ExtractAll` | — | [bsl-zip/src/archive/objects.rs](../../../crates/bsl-zip/src/archive/objects.rs) |

##### `ЗаписьZipФайла`

| Метод | Описание | Реализация |
|---|---|---|
| `Открыть`, `Open` | — | [bsl-zip/src/archive/objects.rs](../../../crates/bsl-zip/src/archive/objects.rs) |
| `Добавить`, `Add` | — | [bsl-zip/src/archive/objects.rs](../../../crates/bsl-zip/src/archive/objects.rs) |
| `Записать`, `Write` | — | [bsl-zip/src/archive/objects.rs](../../../crates/bsl-zip/src/archive/objects.rs) |
| `ПолучитьДвоичныеДанные`, `GetBinaryData` | — | [bsl-zip/src/archive/objects.rs](../../../crates/bsl-zip/src/archive/objects.rs) |
