//! Модель типов XDTO поверх компонентной модели XSD: `ФабрикаXDTO`,
//! `ТипЗначенияXDTO`, `ТипОбъектаXDTO`, `СвойствоXDTO` и соответствие
//! встроенных типов XML Schema типам BSL.
//!
//! Второго разборщика схем здесь нет: на вход идёт готовая `XsSchemaData`
//! из [`crate::xsd`], а этот модуль превращает ЛЕКСИЧЕСКУЮ модель схемы в
//! РАЗРЕШЁННУЮ модель типов — ту, где ссылки уже связаны, наследование
//! сплющено, а границы вхождения посчитаны. Ровно этим две модели и
//! различаются: `ОбъявлениеЭлементаXS` показывает написанное, а
//! `СвойствоXDTO` — вычисленное.
//!
//! # Что ИЗМЕРЕНО на 8.3.27
//!
//! Всё перечисленное снято пробами (`tests/conformance/measure/measure-xdto.bsl`,
//! рядом лежит снятый `measure-xdto.platform.txt`), а не взято из справки.
//! Проб потребовалось много, потому что справка щедра на члены, которых у
//! платформы нет: у `ТипЗначенияXDTO` отвергнуты `Вариант`,
//! `ВариантПростогоТипа`, `Абстрактный`, `Длина`, `МинимальнаяДлина`,
//! `МаксимальнаяДлина`, `Образцы`, `Перечисления`, `ПробельныеСимволы`,
//! `ТипЭлемента`, `ТипыОбъединения`, `Пакет`, `Фабрика`, `Создать`,
//! `ПроверитьЗначение`, `ЭтоНаследник`, у `ТипОбъектаXDTO` — `Фабрика`,
//! `Пакет`, `Создать`, `ЭтоНаследник`, `ПолучитьСвойство`, у
//! `СвойствоXDTO` — `Владелец`, `Нулевой`, `Обязательный`, `Фиксированный`,
//! `Локальный`, `Порядок`, `ЛексическоеЗначениеПоУмолчанию`, `ФормаXML`.
//!
//! * **фабрика строится ровно двумя способами, и они не взаимозаменяемы.**
//!   `СоздатьФабрикуXDTO(Путь)` берёт ТОЛЬКО путь к файлу XSD: от схемы, от
//!   набора схем, от текста схемы, от числа и без аргументов — ошибка, от
//!   двух аргументов — тоже, и на несуществующем пути — тоже ошибка (текст
//!   отказа не снят: обвязка замеров видит только сам факт исключения).
//!   `Новый ФабрикаXDTO(...)` — наоборот, берёт ТОЛЬКО `НаборСхемXML`
//!   (или ничего): путь, схема, текст, массив и два аргумента отвергаются.
//!   Английские написания есть у обоих
//!   (`CreateXDTOFactory`, `Новый XDTOFactory`). Фабрика — СНИМОК:
//!   схема, добавленная в набор после её создания, ей не видна, а две
//!   фабрики от одного файла НЕ равны;
//! * `Строка()` фабрики — `ФабрикаXDTO`, `ТипЗнч()` — «Фабрика XDTO»;
//!   `ЗначениеЗаполнено` от неё — ошибка, посторонний член — ошибка.
//!   Методов у фабрики два, оба с английскими написаниями: `Тип`/`Type` и
//!   `Создать`/`Create`;
//! * `Создать` от типа ЗНАЧЕНИЯ без лексической формы — `Неопределено`, с
//!   формой — `ЗначениеXDTO`; третий аргумент платформа принимает
//!   (четвёртый уже нет), и что он значит — не измерено. `Создать` от типа
//!   ОБЪЕКТА даёт `ОбъектXDTO` и лексической формы не терпит, а
//!   абстрактный тип отвергает совсем. У самого экземпляра `Строка()` —
//!   `ОбъектXDTO`, свой тип он отдаёт МЕТОДОМ `Тип()` (обращение к `Тип`
//!   как к свойству — ошибка), и два экземпляра одного типа не равны;
//! * `ЗначениеXDTO` — только на ЧТЕНИЕ: присваивание и в `Значение`, и в
//!   `ЛексическоеЗначение` платформа отвергает. Свой тип оно отдаёт
//!   методом `Тип()`, а `Владелец` у него нет ни членом, ни методом
//!   (измерено все пять проб);
//! * все три типа существуют под двумя написаниями и печатаются с
//!   пробелами: `ТипЗначенияXDTO`/`XDTOValueType` -> «Тип значения XDTO»,
//!   `ТипОбъектаXDTO`/`XDTOObjectType` -> «Тип объекта XDTO»,
//!   `СвойствоXDTO`/`XDTOProperty` -> «Свойство XDTO». `Новый` ни один из
//!   них не строит (ошибка на всех трёх), а `Новый ФабрикаXDTO` —
//!   наоборот, работает;
//! * `Строка()` от типа — это `{URI}Имя` (`{urn:test}RootType`), но у
//!   АНОНИМНОГО типа, чьё имя пусто, — пустая строка, хотя `URI` у него при
//!   этом целевое пространство схемы. `Строка()` от свойства — его имя;
//! * тип ищется в фабрике парой (URI, имя) или `РасширенноеИмяXML`; одной
//!   строкой — ошибка, неизвестное имя — `Неопределено`, а объявление
//!   глобального ЭЛЕМЕНТА типом не является (`Тип("urn:test", "root")` —
//!   `Неопределено`). Два обращения к `Тип` за одним именем дают РАВНЫЕ
//!   значения, то есть тип — это ссылка на место в модели;
//! * `БазовыйТип` у типа объекта без явного базового — `{...}anyType`, а у
//!   типа значения без явного — `{...}anySimpleType`. У составного типа с
//!   ПРОСТЫМ содержимым (`xs:simpleContent`) базовый тип тоже `anyType`:
//!   простой базовый тип виден не через `БазовыйТип`, а через свойство
//!   `__content`;
//! * `Свойства` типа объекта — СПЛЮЩЕННЫЙ список: сначала свойства
//!   базового типа, потом собственные АТРИБУТЫ, потом собственные
//!   ЭЛЕМЕНТЫ. Измерено на схеме, где у базового типа четыре атрибута и
//!   девять элементов, а у наследника по одному своему:
//!   `id color opt q fx name code дата def many5 notype efx uq anon` и у
//!   наследника `… anon ea extra`;
//! * границы вхождения свойства — `НижняяГраница`/`ВерхняяГраница`,
//!   ЧИСЛА, и `unbounded` показывается числом `-1`. Лексическая модель
//!   схемы отвечает на то же самое иначе: `МаксимальноВходит` частицы с
//!   `maxOccurs="unbounded"` — это СТРОКА `unbounded` (измерено, см.
//!   [`crate::xsd`]). Границы ПЕРЕМНОЖАЮТСЯ по вложенным группам модели:
//!   `<xs:choice minOccurs="0">` превращает `1..1` вложенного элемента в
//!   `0..1`, а `<xs:sequence maxOccurs="unbounded">` — `0..1` в `0..-1`;
//! * `Форма` — член `ФормаXML` (`Элемент`, `Атрибут`, `Текст`), у
//!   `ФормаXML` есть и английские написания членов. `URIПространстваИмен`
//!   свойства подчиняется тому же правилу форм, что и в модели XSD:
//!   квалифицированное объявление даёт целевое пространство, неквалифи-
//!   цированное — пустую строку;
//! * составной тип с `xs:simpleContent` получает свойство с именем
//!   `__content`, формой `Текст`, границами `1..1`, пустым URI и типом
//!   базового простого типа. У СМЕШАННОГО типа (`mixed="true"`) такого
//!   свойства НЕТ — там только объявленные элементы;
//! * `Упорядоченный` — «Да» у последовательности, у пустого типа и у
//!   простого содержимого, «Нет» у `xs:choice` и `xs:all`;
//!   `Последовательный` во всех измеренных случаях равен «НЕ
//!   Упорядоченный ИЛИ Смешанный» (проверено на семи типах, включая
//!   `anyType`, у которого «Да» и то и другое). `Открытый` — «Да» ровно у
//!   `anyType`;
//! * `Фасеты` типа значения — `КоллекцияФасетовXDTO`, но у типа БЕЗ
//!   фасетов это `Неопределено`, а не пустая коллекция (измерено на
//!   `xs:date`). `ФасетXDTO` отдаёт ровно два члена: `Вид` (член
//!   `ВидФасетаXDTO`) и `Значение`, причём `Значение` — всегда СТРОКА,
//!   даже у числовых фасетов (`Минимальное включающее значение=[0]`);
//! * `ЗначениеПоУмолчанию` свойства — `ЗначениеXDTO` с членами `Значение`
//!   (значение BSL) и `ЛексическоеЗначение` (строка). Заполняется и от
//!   `default`, и от `fixed`; у свойства без того и другого —
//!   `Неопределено`;
//! * встроенные типы XML Schema образуют ИЕРАРХИЮ с фасетами: `string`
//!   наследует `anySimpleType` и несёт `Пробельные символы=[preserve]`,
//!   `int` наследует `long` и несёт границы диапазона
//!   `[-2147483648, 2147483647]`, `integer` наследует `decimal` с
//!   `Количество разрядов дробной части=[0]`. Вся таблица снята поимённо —
//!   см. [`BUILTIN_TYPES`];
//! * отображение встроенного типа в тип BSL снято через
//!   `ФабрикаXDTO.Создать(Тип, Лексика).Значение`. Свой тип наследует
//!   отображение базового (`Code` от `xs:string` -> `Строка`, `Small` от
//!   `xs:decimal` -> `Число`), СПИСОК даёт `ФиксированныйМассив`, а
//!   ОБЪЕДИНЕНИЕ выбирает первый член, который принимает лексическую
//!   форму: у `union memberTypes="xs:int xs:string"` запись «5» дала
//!   `Число`, а «аб» — `Строка`.
//!
//! # Экземпляр: что ИЗМЕРЕНО про хранилище
//!
//! * **чтение свойства** идёт по СПЛЮЩЕННОМУ списку типа и регистра не
//!   различает (`О.NAME` читает `name`). Незаполненное свойство —
//!   `Неопределено`, а объявленное с `default` или `fixed` — сразу
//!   значение (`def` -> 7, `fx` -> 9, `color` -> «red», `efx` -> 3).
//!   Постороннее имя — ошибка. МНОЖЕСТВЕННОЕ свойство (верхняя граница не
//!   единица: и `0..-1`, и `1..5`) отдаёт `СписокXDTO`, а не значение,
//!   даже когда список пуст;
//! * **запись** приводит значение к типу свойства ЧЕРЕЗ ЛЕКСИЧЕСКУЮ ФОРМУ:
//!   `О.name = 5` даёт строку «5», `О.name = Дата(2026,8,13)` —
//!   «2026-08-13T00:00:00», `О.name = Истина` — «true», `О.id = "5"` —
//!   число 5, `О.дата = "2026-08-13"` — дату. Отказы — с той же стороны:
//!   `О.id = Истина` и `О.id = 1.5` — ошибка, потому что «true» и «1.5» не
//!   лексические формы `xs:int`. `Неопределено`, `Null` и посторонний
//!   объект в свойство простого типа — тоже ошибка (сброс делает
//!   `Сбросить`, а не присваивание `Неопределено`). `ЗначениеXDTO`
//!   принимается, и берётся из него ЗНАЧЕНИЕ: `Создать(xs:string, "5")` в
//!   свойство `xs:int` дало число 5. Присваивание в множественное
//!   свойство — ошибка;
//! * **свойство типа ОБЪЕКТА** принимает экземпляр своего типа и его
//!   НАСЛЕДНИКА (`ExtType` в свойство типа `RootType` прошёл), а
//!   посторонний тип — ошибка. Записанный объект остаётся ТЕМ ЖЕ (`О.anon
//!   = А; О.anon = А` — «Да») и получает владельца: `А.Владелец() = О` —
//!   «Да». Свойство типа `anyType` — исключение: оно принимает что угодно
//!   как есть (строку, число, объект);
//! * **`СписокXDTO` — окно, а не снимок.** `С = О.code; С.Добавить(...)`
//!   видно через `О.code`, а два отдельных чтения равны (`О.code = О.code`
//!   — «Да»). `Владелец` у него ЧЛЕН (`Владелец()` — ошибка). Методы:
//!   `Количество`/`Count`, `Добавить`/`Add`, `Получить`/`Get`,
//!   `Установить`/`Set`, `Вставить` (АНГЛИЙСКОГО написания у неё нет —
//!   `Insert` отвергнут), `Удалить`/`Delete` (`Remove` отвергнут),
//!   `Очистить`/`Clear`; `Индекс`, `Найти` и `ВГраница` отвергнуты.
//!   Индексация и `Для Каждого` работают, номер за границей — ошибка, как
//!   и `Установить`, `Удалить` за границей и `Добавить` без аргументов.
//!   `Вставить` требует ЗАНЯТОЙ позиции: и в пустой список, и на место
//!   сразу за последним элементом платформа вставлять отказывается, так
//!   что дописать в конец умеет только `Добавить`. Значение в списке — обычное значение BSL, не
//!   `ЗначениеXDTO`. Верхняя граница вхождения при `Добавить` НЕ
//!   проверяется: в свойство `1..5` шесть значений легли молча;
//! * **члены экземпляра**: `Получить`/`Get` и `Установить`/`Set` берут имя
//!   строкой ИЛИ `СвойствоXDTO`, но только у одиночного свойства (у
//!   множественного — ошибка, для него есть `ПолучитьСписок`/`GetList`);
//!   `Установлено`/`IsSet` показывает именно ЗАПИСЬ, а не наличие значения
//!   (у свежего объекта `Установлено("def")` — «Нет», хотя `О.def` отдаёт
//!   7); `Сбросить`/`Unset` возвращает свойство в незаполненное состояние
//!   (`Сброс` отвергнут); `Свойства()`/`Properties()` — коллекция свойств
//!   СВОЕГО ТИПА (14 и 14); `Владелец()`/`Owner()` — метод, у отдельно
//!   созданного объекта `Неопределено`; `Проверить()`/`Validate()`
//!   проверяет границы вхождения и делает это РЕКУРСИВНО (пустой тип
//!   прошёл, недозаполненный `RootType` отвергнут, объект с пустым
//!   вложенным — отвергнут, с заполненным — прошёл, шесть значений в
//!   свойстве `1..5` — отвергнут, недозаполненный объект в списке —
//!   отвергнут). Отвергнуты `ЭтоNull`, `УстановитьNull` и `Владелец` как
//!   член;
//! * **`ПоследовательностьXDTO` достижима** — методом
//!   `Последовательность()`/`Sequence()`, и только у ПОСЛЕДОВАТЕЛЬНОГО
//!   типа: `xs:choice` и `xs:all` дают объект, а тип-последовательность и
//!   простое содержимое — `Неопределено`, а не ошибку. Это порядок
//!   ЗАПОЛНЕНИЯ свойств-элементов: запись атрибута её не удлиняет,
//!   повторная запись одиночного свойства сохраняет своё место
//!   (`ca = "вг"; cb.Добавить("аб"); ca = "де"` -> `[ca=де][cb=аб]`), а
//!   `Сбросить` с последующей записью отправляет свойство в конец.
//!   `Вставить` в список кладёт значение перед тем, на чьё место оно
//!   встало (`[cb=де][cb=аб][ca=вг]`). Члены: `Количество`/`Count`,
//!   `ПолучитьЗначение`/`GetValue`, `ПолучитьСвойство`/`GetProperty`,
//!   `Добавить`/`Add` (берёт именно `СвойствоXDTO` и именно элемент —
//!   имя строкой и атрибут отвергнуты; заполнение видно и через само
//!   свойство), `Очистить`/`Clear` (чистит элементы, атрибуты уцелевают),
//!   `Владелец` членом. Отвергнуты `Получить`, `Свойство`, `ЭтоТекст`,
//!   индексация, `Для Каждого` и `ДобавитьЗначение`;
//! * **`Последовательность().Удалить(0)` РОНЯЕТ ПЛАТФОРМУ.** 8.3.27
//!   падает по сигналу сегментации — не исключение, которое ловится
//!   `Попытка`, а смерть процесса вместе со всем прогоном замеров. Эту
//!   пробу в `measure-xdto.bsl` возвращать нельзя (см. её шапку), а
//!   значит, поведение `Удалить` у последовательности не измерено и
//!   здесь его нет.
//!
//! # Сознательные расхождения и незакрытые углы
//!
//! * **Фасеты только хранятся.** Платформа ПРОВЕРЯЕТ по ним лексическую
//!   форму (измерено: `Создать` от `Small` с «1000» и от `Code` с «аб» —
//!   ошибка). Здесь фасет только читается: проверка образца требует
//!   движка регулярных выражений и делается отдельной задачей.
//! * **Двоичные лексические формы.** `base64Binary` и `hexBinary`
//!   отображаются в `ДвоичныеДанные` (измерено), и разбор обеих записей
//!   здесь есть, но обратной операции — двоичные данные в лексическую
//!   форму — нет: она нужна записи XML, а не модели типов.
//! * **QName с префиксом.** Платформа принимает только запись БЕЗ
//!   префикса (`Создать` от `xs:string` — ошибка, от `просто` —
//!   расширенное имя с пустым URI). Здесь так же: префикс — ошибка, а не
//!   попытка разрешить его по объявлениям схемы.
//! * **Лексическая форма — только строка.** Платформа принимает и число
//!   (`Создать(Тип, 42)` отдаёт `ЗначениеXDTO`), но какую именно запись она
//!   из числа делает — не измерено, а разница видна сразу: `Строка(12.75)`
//!   в 1С — это «12,75» с запятой, а лексическая форма `xs:decimal` — с
//!   точкой. Гадать здесь дороже, чем отказать, поэтому нестроковый
//!   аргумент — ошибка.
//! * **`Пакеты` фабрики не поддержаны.** Платформа отдаёт
//!   `КоллекциюПакетовXDTO` (у фабрики от нашей схемы их два: своё
//!   пространство имён и пространство XML Schema), но пакет — это отдельная
//!   сущность со своим содержимым, и она сюда не входит.
//! * **Фасеты не проверяются и на записи**, и это ровно та же отложенная
//!   работа, что и в `Создать`. Ими объясняются ПЯТЬ из шести расхождений
//!   экземплярной части `measure-xdto.bsl`: платформа отвергает
//!   `О.color = "синий"` (перечисление `red|green`), `code.Добавить("аб")`
//!   и `code.Добавить(5)` (образец `[A-Z]+` и длина 2..5), `О.hl = 5` (тот
//!   же образец у элемента списочного типа) и `О.id = 1.5` (у
//!   `xs:integer` фасет «разрядов дробной части» — ноль), а здесь все пять
//!   проходят. Шестое к фасетам отношения не имеет: в пробе «объект запись
//!   в список строкой» платформа отдаёт `ФиксированныйМассив`, а здесь
//!   получается обычный `Массив` — то самое списочное расхождение,
//!   описанное у `value_from_lexical_at` (неизменяемого вида в этой
//!   реализации нет).
//! * **Английские написания разбираются по имени, а не по получателю.**
//!   Таблица `BUILTIN_METHOD_NAMES` — одна на весь рантайм, и «Вставить»
//!   делит вариант с `Insert`, поэтому `Список.Insert(...)` здесь имя
//!   находит и дальше идёт тем же путём, что `Вставить`, тогда как
//!   платформа у `СписокXDTO` английского написания не знает вовсе.
//!   Ошибка в сторону разрешённого: программа, которая работает на
//!   платформе, работает и здесь. `Remove` в таблице нет ни у одного
//!   получателя, так что `Список.Remove(...)` отвергается и здесь — это
//!   совпадение с платформой, а не разбор по получателю. В самих пробах
//!   расхождения не видно: «список Insert» вставляет в ПУСТОЙ список и
//!   потому ошибка с обеих сторон.
//! * **Порядок обхода `Для Каждого` по списку** — порядок заполнения, тот
//!   же, что у индексации. Отдельной пробы на «список после `Вставить` в
//!   середину» нет: измерено только, куда `Вставить` встаёт в
//!   ПОСЛЕДОВАТЕЛЬНОСТИ.

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use crate::object::BslObject;
use crate::string::BslString;
use crate::types::TypeId;
use crate::xsd::{FacetKind, XName, XsKind, XsSchemaData, XSD_NS};
use crate::{BslValue, EnumValue, RtError, RtResult};

// --- встроенные типы XML Schema ------------------------------------------

/// Тип BSL, в который платформа отображает лексическую форму встроенного
/// типа. Варианты различают не только результат, но и РАЗБОР: у трёх
/// временных типов и у двух двоичных лексические формы разные.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinBsl {
    /// `Строка` — лексическая форма как есть.
    Str,
    /// `Число` без показателя степени: `xs:decimal` и все целые
    /// (измерено: `Создать` от `decimal` с «1.5E3» и от `int` с «1E2» —
    /// ошибка, как и от `int` с «1.5»).
    Number,
    /// `Число` с показателем степени: `xs:double` и `xs:float`
    /// (измерено: «1.5E3» -> 1500).
    Double,
    /// `Булево`: `true`/`1` и `false`/`0`.
    Boolean,
    /// `Дата` из `xs:date` — время суток нулевое.
    Date,
    /// `Дата` из `xs:dateTime`.
    DateTime,
    /// `Дата` из `xs:time` — дата 01.01.0001.
    Time,
    /// `ДвоичныеДанные` из `base64Binary`.
    Base64,
    /// `ДвоичныеДанные` из `hexBinary`.
    Hex,
    /// `РасширенноеИмяXML` из `QName`.
    QName,
}

impl BuiltinBsl {
    /// `ТипЗнч()` значения, которое получится из лексической формы.
    pub fn type_id(self) -> TypeId {
        match self {
            BuiltinBsl::Str => TypeId::String,
            BuiltinBsl::Number | BuiltinBsl::Double => TypeId::Number,
            BuiltinBsl::Boolean => TypeId::Boolean,
            BuiltinBsl::Date | BuiltinBsl::DateTime | BuiltinBsl::Time => TypeId::Date,
            BuiltinBsl::Base64 | BuiltinBsl::Hex => TypeId::BinaryData,
            BuiltinBsl::QName => TypeId::XmlExpandedName,
        }
    }
}

/// Строка таблицы встроенных типов пространства
/// `http://www.w3.org/2001/XMLSchema`.
struct BuiltinType {
    name: &'static str,
    /// Имя базового встроенного типа; `None` только у `anyType`.
    base: Option<&'static str>,
    /// Отображение в тип BSL; `None` — это ТИП ОБЪЕКТА (`anyType`),
    /// значения из лексической формы он не строит.
    bsl: Option<BuiltinBsl>,
    /// Фасеты в том порядке, в каком их отдаёт платформа.
    facets: &'static [(FacetKind, &'static str)],
}

/// Встроенные типы XML Schema: имя, базовый тип, отображение в BSL и
/// фасеты.
///
/// Таблица ИЗМЕРЕНА целиком и пришпилена к строкам
/// `measure-xdto.platform.txt`: столбец «базовый тип» и фасеты — к строкам
/// `фв <имя>`, столбец «тип BSL» — к строкам `вст <имя>`, где значение
/// строилось `ФабрикаXDTO.Создать(Тип, Лексика).Значение`. Ни одна строка
/// не выведена из спецификации W3C: например `фв unsignedInt` показал
/// базовым `unsignedLong` (а не `unsignedShort`, как можно было бы
/// достроить по убыванию разрядности), и здесь стоит измеренное.
///
/// Порядок строк — от корня иерархии вниз, чтобы связывание базовых типов
/// читалось глазами; на работу порядок не влияет.
static BUILTIN_TYPES: &[BuiltinType] = &[
    BuiltinType {
        name: "anyType",
        base: None,
        bsl: None,
        facets: &[],
    },
    BuiltinType {
        name: "anySimpleType",
        base: Some("anyType"),
        bsl: Some(BuiltinBsl::Str),
        facets: &[],
    },
    BuiltinType {
        name: "string",
        base: Some("anySimpleType"),
        bsl: Some(BuiltinBsl::Str),
        facets: &[(FacetKind::WhiteSpace, "preserve")],
    },
    BuiltinType {
        name: "normalizedString",
        base: Some("string"),
        bsl: Some(BuiltinBsl::Str),
        facets: &[(FacetKind::WhiteSpace, "replace")],
    },
    BuiltinType {
        name: "token",
        base: Some("normalizedString"),
        bsl: Some(BuiltinBsl::Str),
        facets: &[(FacetKind::WhiteSpace, "collapse")],
    },
    BuiltinType {
        name: "Name",
        base: Some("token"),
        bsl: Some(BuiltinBsl::Str),
        facets: &[(FacetKind::Pattern, r"\i\c*")],
    },
    BuiltinType {
        name: "NCName",
        base: Some("Name"),
        bsl: Some(BuiltinBsl::Str),
        facets: &[(FacetKind::Pattern, r"[\i-[:]][\c-[:]]*")],
    },
    BuiltinType {
        name: "ID",
        base: Some("NCName"),
        bsl: Some(BuiltinBsl::Str),
        facets: &[],
    },
    BuiltinType {
        name: "NMTOKEN",
        base: Some("token"),
        bsl: Some(BuiltinBsl::Str),
        facets: &[],
    },
    BuiltinType {
        name: "language",
        base: Some("token"),
        bsl: Some(BuiltinBsl::Str),
        facets: &[(FacetKind::Pattern, "[a-zA-Z]{1,8}(-[a-zA-Z0-9]{1,8})*")],
    },
    BuiltinType {
        name: "decimal",
        base: Some("anySimpleType"),
        bsl: Some(BuiltinBsl::Number),
        facets: &[],
    },
    BuiltinType {
        name: "integer",
        base: Some("decimal"),
        bsl: Some(BuiltinBsl::Number),
        facets: &[
            (FacetKind::FractionDigits, "0"),
            (FacetKind::Pattern, r"[\-+]?[0-9]+"),
        ],
    },
    BuiltinType {
        name: "long",
        base: Some("integer"),
        bsl: Some(BuiltinBsl::Number),
        facets: &[
            (FacetKind::MinInclusive, "-9223372036854775808"),
            (FacetKind::MaxInclusive, "9223372036854775807"),
        ],
    },
    BuiltinType {
        name: "int",
        base: Some("long"),
        bsl: Some(BuiltinBsl::Number),
        facets: &[
            (FacetKind::MinInclusive, "-2147483648"),
            (FacetKind::MaxInclusive, "2147483647"),
        ],
    },
    BuiltinType {
        name: "short",
        base: Some("int"),
        bsl: Some(BuiltinBsl::Number),
        facets: &[
            (FacetKind::MinInclusive, "-32768"),
            (FacetKind::MaxInclusive, "32767"),
        ],
    },
    BuiltinType {
        name: "byte",
        base: Some("short"),
        bsl: Some(BuiltinBsl::Number),
        facets: &[
            (FacetKind::MinInclusive, "-128"),
            (FacetKind::MaxInclusive, "127"),
        ],
    },
    BuiltinType {
        name: "nonNegativeInteger",
        base: Some("integer"),
        bsl: Some(BuiltinBsl::Number),
        facets: &[(FacetKind::MinInclusive, "0")],
    },
    BuiltinType {
        name: "positiveInteger",
        base: Some("nonNegativeInteger"),
        bsl: Some(BuiltinBsl::Number),
        facets: &[(FacetKind::MinInclusive, "1")],
    },
    BuiltinType {
        name: "nonPositiveInteger",
        base: Some("integer"),
        bsl: Some(BuiltinBsl::Number),
        facets: &[(FacetKind::MaxInclusive, "0")],
    },
    BuiltinType {
        name: "negativeInteger",
        base: Some("nonPositiveInteger"),
        bsl: Some(BuiltinBsl::Number),
        facets: &[(FacetKind::MaxInclusive, "-1")],
    },
    BuiltinType {
        name: "unsignedLong",
        base: Some("nonNegativeInteger"),
        bsl: Some(BuiltinBsl::Number),
        facets: &[(FacetKind::MaxInclusive, "18446744073709551615")],
    },
    BuiltinType {
        name: "unsignedInt",
        base: Some("unsignedLong"),
        bsl: Some(BuiltinBsl::Number),
        facets: &[(FacetKind::MaxInclusive, "4294967295")],
    },
    BuiltinType {
        name: "unsignedShort",
        base: Some("unsignedInt"),
        bsl: Some(BuiltinBsl::Number),
        facets: &[(FacetKind::MaxInclusive, "65535")],
    },
    BuiltinType {
        name: "unsignedByte",
        base: Some("unsignedShort"),
        bsl: Some(BuiltinBsl::Number),
        facets: &[(FacetKind::MaxInclusive, "255")],
    },
    BuiltinType {
        name: "double",
        base: Some("anySimpleType"),
        bsl: Some(BuiltinBsl::Double),
        facets: &[],
    },
    BuiltinType {
        name: "float",
        base: Some("anySimpleType"),
        bsl: Some(BuiltinBsl::Double),
        facets: &[],
    },
    BuiltinType {
        name: "boolean",
        base: Some("anySimpleType"),
        bsl: Some(BuiltinBsl::Boolean),
        facets: &[],
    },
    BuiltinType {
        name: "date",
        base: Some("anySimpleType"),
        bsl: Some(BuiltinBsl::Date),
        facets: &[],
    },
    BuiltinType {
        name: "dateTime",
        base: Some("anySimpleType"),
        bsl: Some(BuiltinBsl::DateTime),
        facets: &[],
    },
    BuiltinType {
        name: "time",
        base: Some("anySimpleType"),
        bsl: Some(BuiltinBsl::Time),
        facets: &[],
    },
    BuiltinType {
        name: "duration",
        base: Some("anySimpleType"),
        bsl: Some(BuiltinBsl::Str),
        facets: &[],
    },
    BuiltinType {
        name: "gYear",
        base: Some("anySimpleType"),
        bsl: Some(BuiltinBsl::Str),
        facets: &[],
    },
    BuiltinType {
        name: "base64Binary",
        base: Some("anySimpleType"),
        bsl: Some(BuiltinBsl::Base64),
        facets: &[],
    },
    BuiltinType {
        name: "hexBinary",
        base: Some("anySimpleType"),
        bsl: Some(BuiltinBsl::Hex),
        facets: &[],
    },
    BuiltinType {
        name: "anyURI",
        base: Some("anySimpleType"),
        bsl: Some(BuiltinBsl::Str),
        facets: &[],
    },
    BuiltinType {
        name: "QName",
        base: Some("anySimpleType"),
        bsl: Some(BuiltinBsl::QName),
        facets: &[],
    },
];

// --- модель --------------------------------------------------------------

/// Устройство типа ЗНАЧЕНИЯ — то, от чего зависит разбор лексической
/// формы.
#[derive(Debug, Clone)]
enum ValueShape {
    /// Встроенный тип с прямым отображением в тип BSL.
    Builtin(BuiltinBsl),
    /// Атомарный производный тип: отображение берётся у базового
    /// (измерено на `Code` и `Small`).
    Atomic,
    /// Список: значение — `ФиксированныйМассив` значений типа элемента,
    /// лексическая форма разделяется пробельными символами.
    List(Option<usize>),
    /// Объединение: тип выбирается ПЕРВЫМ членом, который принимает
    /// лексическую форму (измерено).
    Union(Vec<usize>),
}

/// Тип модели: и тип значения, и тип объекта — разницу несёт `shape`.
#[derive(Debug)]
struct XdtoTypeData {
    name: String,
    ns: String,
    /// Базовый тип; `None` только у `anyType`.
    base: Option<usize>,
    /// `None` — это тип ОБЪЕКТА.
    shape: Option<ValueShape>,
    /// Фасеты типа значения: вид и лексическая запись значения.
    facets: Vec<(FacetKind, String)>,
    /// Свойства типа объекта — уже сплющенные вместе с унаследованными.
    properties: Vec<usize>,
    open: bool,
    is_abstract: bool,
    ordered: bool,
    mixed: bool,
}

impl XdtoTypeData {
    fn is_value(&self) -> bool {
        self.shape.is_some()
    }

    /// `Последовательный` — во всех измеренных случаях «НЕ Упорядоченный
    /// ИЛИ Смешанный»: последовательность даёт «Нет», `xs:choice` и
    /// `xs:all` — «Да», смешанный тип и `anyType` — «Да».
    fn sequenced(&self) -> bool {
        !self.ordered || self.mixed
    }
}

/// Свойство типа объекта.
#[derive(Debug)]
struct XdtoPropertyData {
    name: String,
    ns: String,
    type_index: usize,
    /// `None` — `unbounded`; наружу обе границы уходят числом, где
    /// `unbounded` — это `-1` (измерено).
    lower: Option<u32>,
    upper: Option<u32>,
    /// Член `ФормаXML`.
    form: EnumValue,
    default: Option<Rc<XdtoValueData>>,
}

/// `ЗначениеXDTO` — значение BSL вместе с лексической формой, из которой
/// оно получено, и номером типа, по которому разбиралось.
#[derive(Debug)]
pub struct XdtoValueData {
    value: BslValue,
    lexical: String,
    /// Номер типа в модели — его отдаёт метод `Тип()` (измерено:
    /// `Создать(xs:int, "42").Тип()` -> «Тип значения XDTO
    /// [{...}int]`»).
    type_index: usize,
}

/// Разрешённая модель типов одной схемы вместе со встроенными типами XML
/// Schema. Значение `ТипЗначенияXDTO` — это `Rc` на неё плюс номер типа,
/// `СвойствоXDTO` — тот же `Rc` плюс номер свойства.
#[derive(Debug)]
pub struct XdtoModel {
    types: Vec<XdtoTypeData>,
    properties: Vec<XdtoPropertyData>,
}

impl XdtoModel {
    fn type_at(&self, i: usize) -> RtResult<&XdtoTypeData> {
        self.types.get(i).ok_or_else(|| broken("тип"))
    }

    fn property_at(&self, i: usize) -> RtResult<&XdtoPropertyData> {
        self.properties.get(i).ok_or_else(|| broken("свойство"))
    }

    /// Тип по расширенному имени — то, что делает `ФабрикаXDTO.Тип(URI,
    /// Имя)`. Анонимные типы сюда не попадают: у них нет имени.
    pub fn find(&self, uri: &str, name: &str) -> Option<usize> {
        if name.is_empty() {
            return None;
        }
        self.types
            .iter()
            .position(|t| t.name == name && t.ns == uri)
    }

    /// Тип BSL, в который отображается тип значения; `None` — у типа
    /// объекта либо у списка и объединения, где тип зависит от значения.
    pub fn builtin_of(&self, index: usize) -> Option<BuiltinBsl> {
        let mut cur = index;
        // Цепочка базовых типов конечна: длина модели — верхняя граница,
        // и она же страхует от цикла в испорченной схеме.
        for _ in 0..=self.types.len() {
            match self.types.get(cur)?.shape.as_ref()? {
                ValueShape::Builtin(b) => return Some(*b),
                ValueShape::List(_) | ValueShape::Union(_) => return None,
                ValueShape::Atomic => cur = self.types.get(cur)?.base?,
            }
        }
        None
    }
}

fn broken(what: &str) -> RtError {
    RtError::Xdto(format!("модель типов XDTO повреждена: нет узла «{what}»"))
}

// --- построение ----------------------------------------------------------

/// Модель типов по одной разобранной схеме.
///
/// # Errors
///
/// [`RtError::Xdto`], если схема ссылается на неизвестный тип, содержит
/// цикл наследования либо значение по умолчанию, которое не разбирается в
/// объявленном типе.
pub fn model_of_schema(schema: &Rc<XsSchemaData>) -> RtResult<Rc<XdtoModel>> {
    model_of_schemas(std::slice::from_ref(schema))
}

/// Модель типов по НАБОРУ схем — то, что стоит за `Новый
/// ФабрикаXDTO(НаборСхемXML)`.
///
/// Встроенные типы XML Schema объявляются один раз на всю модель, а имена в
/// ссылках (`base`, `type`, `itemType`, `memberTypes`) разрешаются по всем
/// схемам набора сразу: схема из одного пространства имён вправе ссылаться
/// на тип из другой схемы того же набора. Двусмысленности это не создаёт —
/// `НаборСхемXML` держит не больше одной схемы на пространство имён
/// (измерено, см. [`crate::xsd`]).
///
/// Пустой набор даёт фабрику с одними встроенными типами — ровно то же, что
/// `Новый ФабрикаXDTO` без аргументов (измерено: у такой фабрики
/// `Тип({...}string)` есть, а `Тип({urn:test}RootType)` — `Неопределено`).
///
/// # Errors
///
/// [`RtError::Xdto`], если какая-нибудь схема набора ссылается на неизвестный
/// тип, содержит цикл наследования либо значение по умолчанию, которое не
/// разбирается в объявленном типе.
pub fn model_of_schemas(schemas: &[Rc<XsSchemaData>]) -> RtResult<Rc<XdtoModel>> {
    let mut builder = Builder::new(schemas);
    builder.declare_builtins();
    builder.declare_schema_types()?;
    builder.link_bases()?;
    builder.build_properties()?;
    Ok(Rc::new(builder.model))
}

/// Место узла в наборе схем: номер схемы и номер узла в ней. Номера узлов
/// у схем свои, поэтому по всему набору однозначна только пара.
type XsPlace = (usize, usize);

struct Builder<'a> {
    schemas: &'a [Rc<XsSchemaData>],
    model: XdtoModel,
    /// Место узла XSD -> номер типа модели, для типов, объявленных схемами.
    from_xs: Vec<(XsPlace, usize)>,
    /// Номер типа модели -> место узла XSD, откуда он построен.
    to_xs: Vec<Option<XsPlace>>,
    /// Тип, чьи свойства сейчас считаются, — страховка от цикла
    /// наследования.
    busy: Vec<bool>,
    done: Vec<bool>,
}

impl<'a> Builder<'a> {
    fn new(schemas: &'a [Rc<XsSchemaData>]) -> Builder<'a> {
        Builder {
            schemas,
            model: XdtoModel {
                types: Vec::new(),
                properties: Vec::new(),
            },
            from_xs: Vec::new(),
            to_xs: Vec::new(),
            busy: Vec::new(),
            done: Vec::new(),
        }
    }

    /// Схема по номеру. Номер приходит только изнутри — из `to_xs` или из
    /// перебора набора, — но подтверждать это `unwrap`ом на
    /// пользовательских данных незачем: испорченный номер значит, что
    /// испорчена сама модель, и об этом есть [`broken`].
    fn schema_at(&self, si: usize) -> RtResult<&XsSchemaData> {
        self.schemas
            .get(si)
            .map(Rc::as_ref)
            .ok_or_else(|| broken("схема"))
    }

    fn push_type(&mut self, data: XdtoTypeData, xs: Option<XsPlace>) -> usize {
        self.model.types.push(data);
        self.to_xs.push(xs);
        self.busy.push(false);
        self.done.push(false);
        if let Some(place) = xs {
            self.from_xs.push((place, self.model.types.len() - 1));
        }
        self.model.types.len() - 1
    }

    /// Встроенные типы пространства XML Schema — они есть у любой фабрики
    /// (измерено: `Новый ФабрикаXDTO` уже знает `{...}string`).
    fn declare_builtins(&mut self) {
        for row in BUILTIN_TYPES {
            // Единственный встроенный тип ОБЪЕКТА — `anyType`, и три его
            // флага измерены разом: открытый, упорядоченный и смешанный.
            // У всех остальных встроенных (это типы значения) те же флаги
            // не читаются вовсе.
            let is_any_type = row.bsl.is_none();
            self.push_type(
                XdtoTypeData {
                    name: row.name.to_string(),
                    ns: XSD_NS.to_string(),
                    base: None,
                    shape: row.bsl.map(ValueShape::Builtin),
                    facets: row
                        .facets
                        .iter()
                        .map(|(k, v)| (*k, (*v).to_string()))
                        .collect(),
                    properties: Vec::new(),
                    open: is_any_type,
                    is_abstract: false,
                    ordered: is_any_type,
                    mixed: is_any_type,
                },
                None,
            );
        }
        for (i, row) in BUILTIN_TYPES.iter().enumerate() {
            let base = row.base.and_then(|name| self.model.find(XSD_NS, name));
            self.model.types[i].base = base;
        }
    }

    /// Именованные глобальные типы каждой схемы набора. Анонимные
    /// объявляются позже, при разборе свойств: на них ссылается только своё
    /// свойство.
    fn declare_schema_types(&mut self) -> RtResult<()> {
        for si in 0..self.schemas.len() {
            // Номера копируются, потому что `declare_type` берёт `&mut
            // self`, а список живёт в схеме за общей ссылкой.
            let nodes: Vec<usize> = self.schema_at(si)?.global_types().to_vec();
            for node in nodes {
                self.declare_type(si, node)?;
            }
        }
        Ok(())
    }

    fn declare_type(&mut self, si: usize, node: usize) -> RtResult<usize> {
        if let Some((_, idx)) = self.from_xs.iter().find(|(p, _)| *p == (si, node)) {
            return Ok(*idx);
        }
        let schema = self.schema_at(si)?;
        // Пространство имён у типа модели — ЦЕЛЕВОЕ пространство схемы, и
        // у анонимного тоже, хотя имя у него пусто (измерено: у типа
        // безымянного `<xs:complexType>` внутри объявления `URI` —
        // `urn:test`). Лексическая модель XSD здесь другая: там у
        // анонимного типа пространство имён пусто.
        let target_ns = schema.target_namespace().to_string();
        let name = schema.name_of(node).to_string();
        let data = match schema.kind_of(node) {
            XsKind::SimpleType => {
                let shape = match schema.simple_variety_of(node) {
                    Some((EnumValue::XsVarietyList, _, _)) => ValueShape::List(None),
                    Some((EnumValue::XsVarietyUnion, _, _)) => ValueShape::Union(Vec::new()),
                    _ => ValueShape::Atomic,
                };
                XdtoTypeData {
                    name,
                    ns: target_ns,
                    base: None,
                    shape: Some(shape),
                    facets: schema
                        .facets_of(node)
                        .into_iter()
                        .map(|(k, v)| (k, v.to_string()))
                        .collect(),
                    properties: Vec::new(),
                    // Четыре флага ниже читаются только у типа ОБЪЕКТА
                    // (у типа значения обращение к ним платформа
                    // отвергает), поэтому у типов значения они выключены
                    // все и одинаково — и у схемных, и у встроенных.
                    open: false,
                    is_abstract: false,
                    ordered: false,
                    mixed: false,
                }
            }
            XsKind::ComplexType => {
                let (mixed, is_abstract) = schema.complex_flags_of(node);
                XdtoTypeData {
                    name,
                    ns: target_ns,
                    base: None,
                    shape: None,
                    facets: Vec::new(),
                    properties: Vec::new(),
                    open: false,
                    is_abstract,
                    ordered: content_is_ordered(schema, node),
                    mixed,
                }
            }
            other => {
                return Err(RtError::Xdto(format!(
                    "типом XDTO может стать только определение типа, а не «{}»",
                    other.type_name()
                )))
            }
        };
        Ok(self.push_type(data, Some((si, node))))
    }

    /// Базовые типы схемных типов: имя из `base` разрешается в номер. Имя
    /// ищется по ВСЕМУ набору, а не только в своей схеме, — иначе ссылка на
    /// соседнее пространство имён обрывалась бы.
    fn link_bases(&mut self) -> RtResult<()> {
        for i in 0..self.model.types.len() {
            let Some((si, node)) = self.to_xs[i] else {
                continue;
            };
            let base = if self.model.types[i].is_value() {
                let name = self.schema_at(si)?.simple_base_of(node).cloned();
                match name {
                    Some(n) => Some(self.require_type(&n)?),
                    // Тип значения без явного базового наследует
                    // `anySimpleType` (измерено на списке и объединении).
                    None => self.model.find(XSD_NS, "anySimpleType"),
                }
            } else {
                // У типа объекта базовым становится только ОБЪЕКТНЫЙ
                // базовый тип: у составного типа с простым содержимым
                // платформа отдаёт `anyType`, а простой базовый тип
                // виден свойством `__content` (измерено).
                let name = self.schema_at(si)?.complex_base_of(node).cloned();
                let resolved = match name {
                    Some(n) => Some(self.require_type(&n)?),
                    None => None,
                };
                match resolved {
                    Some(b) if !self.model.types[b].is_value() => Some(b),
                    _ => self.model.find(XSD_NS, "anyType"),
                }
            };
            self.model.types[i].base = base;
        }
        // Тип элемента списка и члены объединения — по тем же именам.
        for i in 0..self.model.types.len() {
            let Some((si, node)) = self.to_xs[i] else {
                continue;
            };
            let Some((variety, item, members)) = self.schema_at(si)?.simple_variety_of(node) else {
                continue;
            };
            let shape = match variety {
                EnumValue::XsVarietyList => {
                    let item = match item.cloned() {
                        Some(n) => Some(self.require_type(&n)?),
                        None => None,
                    };
                    ValueShape::List(item)
                }
                EnumValue::XsVarietyUnion => {
                    let names: Vec<XName> = members.to_vec();
                    let mut resolved = Vec::with_capacity(names.len());
                    for n in &names {
                        resolved.push(self.require_type(n)?);
                    }
                    ValueShape::Union(resolved)
                }
                _ => continue,
            };
            self.model.types[i].shape = Some(shape);
        }
        Ok(())
    }

    /// Тип по имени — с ошибкой вместо `Неопределено`: ссылка на
    /// несуществующий тип делает модель неполной, и молчать об этом хуже,
    /// чем отказать. Ищется по всем схемам набора сразу.
    fn require_type(&self, name: &XName) -> RtResult<usize> {
        self.model.find(&name.uri, &name.local).ok_or_else(|| {
            RtError::Xdto(format!(
                "в схеме нет типа «{}», на который ссылается модель",
                name.display_text()
            ))
        })
    }

    fn build_properties(&mut self) -> RtResult<()> {
        for i in 0..self.model.types.len() {
            self.ensure_properties(i)?;
        }
        Ok(())
    }

    /// Свойства типа объекта: сначала унаследованные, потом собственные
    /// атрибуты, потом собственные элементы (измеренный порядок).
    fn ensure_properties(&mut self, index: usize) -> RtResult<()> {
        if self.done[index] {
            return Ok(());
        }
        if self.busy[index] {
            return Err(RtError::Xdto(format!(
                "циклическое наследование типов XDTO вокруг «{}»",
                self.model.types[index].name
            )));
        }
        self.busy[index] = true;
        let mut props = Vec::new();
        if let Some(base) = self.model.types[index].base {
            if !self.model.types[base].is_value() {
                self.ensure_properties(base)?;
                props.extend_from_slice(&self.model.types[base].properties);
            }
        }
        if let Some((si, node)) = self.to_xs[index] {
            if !self.model.types[index].is_value() {
                self.collect_attributes(si, node, &mut props)?;
                self.collect_content(si, node, &mut props)?;
            }
        }
        self.model.types[index].properties = props;
        self.busy[index] = false;
        self.done[index] = true;
        Ok(())
    }

    /// Собственные атрибуты составного типа. Обязательный атрибут даёт
    /// границы `1..1`, необязательный — `0..1` (измерено).
    fn collect_attributes(&mut self, si: usize, node: usize, out: &mut Vec<usize>) -> RtResult<()> {
        let uses: Vec<usize> = self.schema_at(si)?.complex_attribute_uses_of(node).to_vec();
        for use_node in uses {
            let schema = self.schema_at(si)?;
            let Some(view) = schema.attribute_use_of(use_node) else {
                continue;
            };
            let (decl_node, required, lexical, has_constraint) = (
                view.declaration,
                view.required,
                view.lexical.to_string(),
                view.has_constraint,
            );
            let Some(decl) = schema.decl_of(decl_node) else {
                continue;
            };
            let (name, ns) = (decl.name.to_string(), decl.ns.to_string());
            let type_index = self.property_type(si, decl_node)?;
            let default = if has_constraint {
                Some(self.value_of(type_index, &lexical)?)
            } else {
                None
            };
            let property = XdtoPropertyData {
                name,
                ns,
                type_index,
                lower: Some(u32::from(required)),
                upper: Some(1),
                form: EnumValue::XmlFormAttribute,
                default,
            };
            self.model.properties.push(property);
            out.push(self.model.properties.len() - 1);
        }
        Ok(())
    }

    /// Собственное содержимое: либо элементы модели содержимого, либо
    /// текстовое свойство `__content` у типа с простым содержимым.
    fn collect_content(&mut self, si: usize, node: usize, out: &mut Vec<usize>) -> RtResult<()> {
        if let Some(particle) = self.schema_at(si)?.complex_content_of(node) {
            return self.collect_elements(si, particle, Some(1), Some(1), out);
        }
        // Простое содержимое: базовый тип — простой, и платформа
        // показывает его свойством `__content` с формой `Текст`
        // (измерено). Отличать `xs:simpleContent` от `xs:complexContent`
        // отдельным признаком не нужно: у простого содержимого нет модели
        // содержимого, а базовый тип — тип ЗНАЧЕНИЯ, и обе проверки уже
        // сделаны выше. СМЕШАННЫЙ тип сюда не доходит: модель содержимого
        // у него есть, и своего текстового свойства платформа ему не даёт
        // (измерено на `mixed="true"` — там только объявленный элемент).
        let Some(base_name) = self.schema_at(si)?.complex_base_of(node).cloned() else {
            return Ok(());
        };
        let base = self.require_type(&base_name)?;
        if !self.model.types[base].is_value() {
            return Ok(());
        }
        self.model.properties.push(XdtoPropertyData {
            name: CONTENT_PROPERTY.to_string(),
            ns: String::new(),
            type_index: base,
            lower: Some(1),
            upper: Some(1),
            form: EnumValue::XmlFormText,
            default: None,
        });
        out.push(self.model.properties.len() - 1);
        Ok(())
    }

    /// Разложить фрагмент в свойства, перемножая границы вхождения по
    /// вложенным группам модели.
    fn collect_elements(
        &mut self,
        si: usize,
        particle: usize,
        outer_lower: Option<u32>,
        outer_upper: Option<u32>,
        out: &mut Vec<usize>,
    ) -> RtResult<()> {
        let schema = self.schema_at(si)?;
        let Some((term, min, max)) = schema.particle_of(particle) else {
            return Ok(());
        };
        let lower = fold_bounds(outer_lower, bound_of(min, 1));
        let upper = fold_bounds(outer_upper, bound_of(max, 1));
        if let Some((_, particles)) = schema.model_group_of(term) {
            let inner: Vec<usize> = particles.to_vec();
            for p in inner {
                self.collect_elements(si, p, lower, upper, out)?;
            }
            return Ok(());
        }
        let Some(decl) = schema.decl_of(term) else {
            return Err(RtError::Xdto(
                "термом фрагмента может быть объявление элемента или группа модели".to_string(),
            ));
        };
        let (name, ns, lexical, has_constraint) = (
            decl.name.to_string(),
            decl.ns.to_string(),
            decl.lexical.to_string(),
            decl.has_constraint,
        );
        let type_index = self.property_type(si, term)?;
        let default = if has_constraint {
            Some(self.value_of(type_index, &lexical)?)
        } else {
            None
        };
        self.model.properties.push(XdtoPropertyData {
            name,
            ns,
            type_index,
            lower,
            upper,
            form: EnumValue::XmlFormElement,
            default,
        });
        out.push(self.model.properties.len() - 1);
        Ok(())
    }

    /// Тип свойства: объявленный `type`, встроенный анонимный тип или —
    /// если ни того, ни другого нет — `anyType` (измерено на
    /// `<xs:element name="notype"/>`).
    fn property_type(&mut self, si: usize, decl_node: usize) -> RtResult<usize> {
        let (type_name, anonymous) = match self.schema_at(si)?.decl_of(decl_node) {
            Some(d) => (d.type_name.cloned(), d.anonymous_type),
            None => (None, None),
        };
        if let Some(name) = type_name {
            return self.require_type(&name);
        }
        if let Some(node) = anonymous {
            let index = self.declare_type(si, node)?;
            // Анонимный тип объявлен уже после связывания базовых типов,
            // поэтому его база и свойства достраиваются здесь же.
            self.link_one_base(si, index, node)?;
            self.ensure_properties(index)?;
            return Ok(index);
        }
        self.model
            .find(XSD_NS, "anyType")
            .ok_or_else(|| broken("anyType"))
    }

    /// Базовый тип одного (анонимного) типа — та же логика, что в
    /// [`Builder::link_bases`], но для типа, объявленного позже.
    fn link_one_base(&mut self, si: usize, index: usize, node: usize) -> RtResult<()> {
        if self.model.types[index].base.is_some() {
            return Ok(());
        }
        let base = if self.model.types[index].is_value() {
            match self.schema_at(si)?.simple_base_of(node).cloned() {
                Some(n) => Some(self.require_type(&n)?),
                None => self.model.find(XSD_NS, "anySimpleType"),
            }
        } else {
            let resolved = match self.schema_at(si)?.complex_base_of(node).cloned() {
                Some(n) => Some(self.require_type(&n)?),
                None => None,
            };
            match resolved {
                Some(b) if !self.model.types[b].is_value() => Some(b),
                _ => self.model.find(XSD_NS, "anyType"),
            }
        };
        self.model.types[index].base = base;
        Ok(())
    }

    fn value_of(&self, type_index: usize, lexical: &str) -> RtResult<Rc<XdtoValueData>> {
        Ok(Rc::new(XdtoValueData {
            value: value_from_lexical(&self.model, type_index, lexical)?,
            lexical: lexical.to_string(),
            type_index,
        }))
    }
}

/// Имя свойства, которым платформа показывает текст типа с простым
/// содержимым (измерено).
const CONTENT_PROPERTY: &str = "__content";

/// `Упорядоченный` — «Да» у последовательности и у типа без модели
/// содержимого, «Нет» у `xs:choice` и `xs:all` (измерено на пяти типах).
fn content_is_ordered(schema: &XsSchemaData, node: usize) -> bool {
    let Some(particle) = schema.complex_content_of(node) else {
        return true;
    };
    let Some((term, _, _)) = schema.particle_of(particle) else {
        return true;
    };
    match schema.model_group_of(term) {
        Some((EnumValue::XsGroupSequence, _)) => true,
        Some(_) => false,
        None => true,
    }
}

/// Граница вхождения из лексической модели XSD: отсутствующий атрибут —
/// это `default`, а `unbounded` (то есть `u32::MAX`) — `None`.
fn bound_of(raw: Option<u32>, default: u32) -> Option<u32> {
    match raw {
        None => Some(default),
        Some(u32::MAX) => None,
        Some(n) => Some(n),
    }
}

/// Границы перемножаются по вложенным группам модели (измерено:
/// `<xs:choice minOccurs="0">` делает `1..1` вложенного элемента `0..1`, а
/// `<xs:sequence maxOccurs="unbounded">` делает `0..1` -> `0..-1`). Ноль
/// поглощает бесконечность: вхождений всё равно ноль.
fn fold_bounds(outer: Option<u32>, inner: Option<u32>) -> Option<u32> {
    match (outer, inner) {
        (Some(0), _) | (_, Some(0)) => Some(0),
        (Some(a), Some(b)) => Some(a.saturating_mul(b)),
        _ => None,
    }
}

// --- лексические формы ---------------------------------------------------

/// Значение BSL из лексической формы по типу модели.
///
/// # Errors
///
/// [`RtError::Xdto`], если тип — объектный, если лексическая форма не
/// разбирается в его отображении либо если ни один член объединения её не
/// принял.
pub fn value_from_lexical(
    model: &XdtoModel,
    type_index: usize,
    lexical: &str,
) -> RtResult<BslValue> {
    // Глубины хватает на любую честную схему: цепочка «производный тип ->
    // базовый» не длиннее числа типов, а список и объединение добавляют
    // по шагу на уровень вложенности. Ограничение здесь не оптимизация, а
    // страховка: `<xs:restriction base="t:A"/>` внутри самого `A` даёт
    // КОЛЬЦО, и без счётчика разбор ушёл бы в переполнение стека.
    value_from_lexical_at(model, type_index, lexical, model.types.len() + 8)
}

fn value_from_lexical_at(
    model: &XdtoModel,
    type_index: usize,
    lexical: &str,
    depth: usize,
) -> RtResult<BslValue> {
    let data = model.type_at(type_index)?;
    let Some(depth) = depth.checked_sub(1) else {
        return Err(RtError::Xdto(format!(
            "цепочка типов вокруг «{}» замкнута сама на себя",
            data.name
        )));
    };
    let Some(shape) = data.shape.as_ref() else {
        return Err(RtError::Xdto(format!(
            "тип объекта «{}» не строится из лексической формы",
            data.name
        )));
    };
    match shape {
        ValueShape::Builtin(bsl) => builtin_from_lexical(*bsl, lexical),
        ValueShape::Atomic => {
            let base = data.base.ok_or_else(|| {
                RtError::Xdto(format!("у типа значения «{}» нет базового типа", data.name))
            })?;
            value_from_lexical_at(model, base, lexical, depth)
        }
        // Список: лексическая форма делится пробельными символами.
        // Платформа отдаёт `ФиксированныйМассив` (измерено), а здесь это
        // обычный `Массив` — своего неизменяемого вида в этой реализации
        // нет, и `vstr.rs` читает фиксированный массив тем же обычным
        // (терять данные хуже, чем терять неизменяемость).
        ValueShape::List(item) => {
            let item = item.ok_or_else(|| {
                RtError::Xdto(format!(
                    "у списочного типа «{}» нет типа элемента",
                    data.name
                ))
            })?;
            let mut items = Vec::new();
            for part in lexical.split_whitespace() {
                items.push(value_from_lexical_at(model, item, part, depth)?);
            }
            Ok(BslValue::new_array(items))
        }
        // Объединение: первый член, который принял форму (измерено на
        // `union memberTypes="xs:int xs:string"`).
        ValueShape::Union(members) => {
            for member in members {
                if let Ok(v) = value_from_lexical_at(model, *member, lexical, depth) {
                    return Ok(v);
                }
            }
            Err(RtError::Xdto(format!(
                "лексическую форму «{lexical}» не принял ни один член объединения «{}»",
                data.name
            )))
        }
    }
}

fn bad_lexical(lexical: &str, what: &str) -> RtError {
    RtError::Xdto(format!("«{lexical}» — не лексическая форма {what}"))
}

/// Значение встроенного типа из лексической формы. Правила измерены
/// поимённо: у `xs:boolean` принимаются и слова, и цифры; у чисел —
/// ведущий плюс, хвостовые нули и показатель степени (`1.5E3` -> 1500);
/// пробелы по краям отбрасываются у всех.
fn builtin_from_lexical(bsl: BuiltinBsl, lexical: &str) -> RtResult<BslValue> {
    match bsl {
        // Строка идёт как есть, БЕЗ обрезки: `xs:string` с одними
        // пробелами — это пробелы (фасет `whiteSpace` их не трогает, он
        // только описан).
        BuiltinBsl::Str => Ok(BslValue::Str(BslString::from_str(lexical))),
        BuiltinBsl::Number => match bsl_number::BslNumber::parse_canonical(lexical.trim()) {
            Ok(n) => Ok(BslValue::Number(n)),
            Err(_) => Err(bad_lexical(lexical, "числа")),
        },
        BuiltinBsl::Double => parse_exponential(lexical.trim()),
        BuiltinBsl::Boolean => match lexical.trim() {
            "true" | "1" => Ok(BslValue::Boolean(true)),
            "false" | "0" => Ok(BslValue::Boolean(false)),
            _ => Err(bad_lexical(lexical, "«xs:boolean»")),
        },
        BuiltinBsl::Date => parse_xsd_date(lexical.trim()),
        BuiltinBsl::DateTime => parse_xsd_date_time(lexical.trim()),
        BuiltinBsl::Time => parse_xsd_time(lexical.trim()),
        BuiltinBsl::Base64 => {
            let bytes =
                decode_base64(lexical).ok_or_else(|| bad_lexical(lexical, "«base64Binary»"))?;
            Ok(BslValue::Object(Rc::new(BslObject::BinaryData(Rc::from(
                bytes.into_boxed_slice(),
            )))))
        }
        BuiltinBsl::Hex => {
            let bytes =
                decode_hex(lexical.trim()).ok_or_else(|| bad_lexical(lexical, "«hexBinary»"))?;
            Ok(BslValue::Object(Rc::new(BslObject::BinaryData(Rc::from(
                bytes.into_boxed_slice(),
            )))))
        }
        // Префикс платформа не разрешает вовсе: `Создать` от `xs:string`
        // — ошибка, а `просто` даёт имя с ПУСТЫМ URI (измерено).
        BuiltinBsl::QName => {
            let text = lexical.trim();
            if text.contains(':') || text.is_empty() {
                return Err(bad_lexical(lexical, "«QName» без префикса"));
            }
            Ok(crate::xsd::new_expanded_name("", text))
        }
    }
}

/// Лексическая форма `xs:double`/`xs:float`: то же десятичное число, но с
/// необязательным показателем степени. Показатель поддерживают ровно эти
/// два типа — `Создать` от `xs:decimal` с «1.5E3» и от `xs:int` с «1E2»
/// платформа отвергает (измерено), поэтому у остальных числовых типов
/// разбор обычный.
///
/// `INF`, `-INF` и `NaN` платформа принимает (измерено на `INF`), а здесь
/// они отвергаются: `Число` в 1С — десятичное с конечной точностью, и
/// бесконечности в нём нет.
fn parse_exponential(text: &str) -> RtResult<BslValue> {
    let (mantissa, exponent) = match text.split_once(['E', 'e']) {
        Some((m, e)) => {
            let e: i64 = e
                .strip_prefix('+')
                .unwrap_or(e)
                .parse()
                .map_err(|_| bad_lexical(text, "числа с показателем степени"))?;
            (m, e)
        }
        None => (text, 0),
    };
    let mantissa =
        bsl_number::BslNumber::parse_canonical(mantissa).map_err(|_| bad_lexical(text, "числа"))?;
    if exponent == 0 {
        return Ok(BslValue::Number(mantissa));
    }
    // Десятичный сдвиг — это умножение или деление на степень десяти;
    // умножение точное, а деление идёт через ту же операцию, что и
    // обычное `/`, то есть с округлением до 27 знаков.
    let magnitude = u32::try_from(exponent.unsigned_abs())
        .map_err(|_| bad_lexical(text, "числа с показателем степени"))?;
    let ten = bsl_number::BslNumber::from_i64(10);
    let mut power = bsl_number::BslNumber::from_i64(1);
    for _ in 0..magnitude {
        power = power
            .mul(&ten)
            .map_err(|_| bad_lexical(text, "числа с показателем степени"))?;
    }
    let scaled = if exponent > 0 {
        mantissa.mul(&power)
    } else {
        mantissa.div(&power)
    };
    scaled
        .map(BslValue::Number)
        .map_err(|_| bad_lexical(text, "числа с показателем степени"))
}

/// `xs:date`: `ГГГГ-ММ-ДД` с необязательным поясом. Пояс не отбрасывается,
/// а пересчитывается в местное время, поэтому `2026-08-12+02:00` на машине
/// с поясом +03:00 дало 12.08.2026 1:00:00 (измерено).
fn parse_xsd_date(text: &str) -> RtResult<BslValue> {
    let (body, tail) = split_zone(text, 10);
    let mut parts = body.split('-');
    let year: i64 = parse_part(parts.next(), text, "даты")?;
    let month: u32 = parse_part(parts.next(), text, "даты")?;
    let day: u32 = parse_part(parts.next(), text, "даты")?;
    if parts.next().is_some() {
        return Err(bad_lexical(text, "даты"));
    }
    let wall = crate::BslDate::from_civil(year, month, day, 0, 0, 0)
        .ok_or_else(|| bad_lexical(text, "даты"))?;
    Ok(BslValue::Date(apply_zone(wall, tail)?))
}

/// `xs:dateTime`: `ГГГГ-ММ-ДДTЧЧ:ММ:СС` с необязательным поясом.
fn parse_xsd_date_time(text: &str) -> RtResult<BslValue> {
    let t = text
        .find('T')
        .ok_or_else(|| bad_lexical(text, "«dateTime»"))?;
    let (body, tail) = split_zone(text, t + 1);
    let (date_part, time_part) = body.split_at(t);
    let mut dp = date_part.split('-');
    let year: i64 = parse_part(dp.next(), text, "«dateTime»")?;
    let month: u32 = parse_part(dp.next(), text, "«dateTime»")?;
    let day: u32 = parse_part(dp.next(), text, "«dateTime»")?;
    let (hour, minute, second) = parse_clock(&time_part[1..], text)?;
    let wall = crate::BslDate::from_civil(year, month, day, hour, minute, second)
        .ok_or_else(|| bad_lexical(text, "«dateTime»"))?;
    Ok(BslValue::Date(apply_zone(wall, tail)?))
}

/// `xs:time`: `ЧЧ:ММ:СС`. Дата у результата — 01.01.0001 (измерено).
fn parse_xsd_time(text: &str) -> RtResult<BslValue> {
    let (body, tail) = split_zone(text, 0);
    let (hour, minute, second) = parse_clock(body, text)?;
    let wall = crate::BslDate::from_civil(1, 1, 1, hour, minute, second)
        .ok_or_else(|| bad_lexical(text, "времени"))?;
    Ok(BslValue::Date(apply_zone(wall, tail)?))
}

fn parse_part<T: std::str::FromStr>(part: Option<&str>, text: &str, what: &str) -> RtResult<T> {
    part.and_then(|p| p.parse().ok())
        .ok_or_else(|| bad_lexical(text, what))
}

fn parse_clock(text: &str, whole: &str) -> RtResult<(u32, u32, u32)> {
    let mut parts = text.split(':');
    let hour: u32 = parse_part(parts.next(), whole, "времени")?;
    let minute: u32 = parse_part(parts.next(), whole, "времени")?;
    // Доли секунды платформа принимает, но `Дата` их не хранит.
    let seconds = parts.next().unwrap_or("0");
    let second: u32 = parse_part(seconds.split('.').next(), whole, "времени")?;
    if parts.next().is_some() {
        return Err(bad_lexical(whole, "времени"));
    }
    Ok((hour, minute, second))
}

/// Хвост часового пояса, если он есть: `Z` либо `±ЧЧ:ММ`. Знак ищется
/// начиная с `from`, чтобы дефисы самой даты не попали в пояс.
///
/// `from` — БАЙТОВОЕ смещение, посчитанное по ожидаемой длине формы
/// (`parse_xsd_date` передаёт 10 — длину `ГГГГ-ММ-ДД`), а лексическая форма
/// приходит из схемы и может быть какой угодно: у `2026-08-1я` смещение 10
/// попадает ВНУТРЬ многобайтового символа. Поэтому срез берётся через
/// `get`: не граница символа — значит пояса тут нет, форма возвращается
/// целиком, и ошибку выдаёт вызывающий разбор (`bad_lexical`), а не паника
/// на пользовательских данных.
fn split_zone(text: &str, from: usize) -> (&str, Option<i32>) {
    if let Some(body) = text.strip_suffix('Z') {
        return (body, Some(0));
    }
    if let Some(tail) = text.get(from..) {
        if let Some(rel) = tail.find(['+', '-']) {
            let at = from + rel;
            let sign = if text.as_bytes()[at] == b'-' { -1 } else { 1 };
            if let Some((h, m)) = text[at + 1..].split_once(':') {
                if let (Ok(h), Ok(m)) = (h.parse::<i32>(), m.parse::<i32>()) {
                    return (&text[..at], Some(sign * (h * 3600 + m * 60)));
                }
            }
        }
    }
    (text, None)
}

/// Пояс пересчитывается в МЕСТНОЕ время машины, как это делает платформа
/// (измерено: `2026-08-12T18:41:17Z` дало 21:41:17 на машине с +03:00, а
/// `…+02:00` — 19:41:17). Без пояса запись остаётся как есть.
fn apply_zone(wall: crate::BslDate, zone: Option<i32>) -> RtResult<crate::BslDate> {
    match zone {
        None => Ok(wall),
        Some(offset) => crate::json::local_date_from_utc_seconds(
            crate::json::pseudo_unix_seconds(wall) - i64::from(offset),
            "лексическая форма XDTO",
        ),
    }
}

/// Разбор `hexBinary`: пары шестнадцатеричных цифр, регистр не важен.
fn decode_hex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() / 2);
    for pair in bytes.chunks(2) {
        let hi = (pair[0] as char).to_digit(16)?;
        let lo = (pair[1] as char).to_digit(16)?;
        out.push((hi * 16 + lo) as u8);
    }
    Some(out)
}

/// Разбор `base64Binary`. Пробельные символы внутри записи игнорируются —
/// так требует XML Schema и так ведёт себя платформа с многострочным
/// содержимым элемента.
fn decode_base64(text: &str) -> Option<Vec<u8>> {
    let mut quad = [0u8; 4];
    let mut filled = 0usize;
    let mut padding = 0usize;
    let mut out = Vec::new();
    for ch in text.chars() {
        if ch.is_whitespace() {
            continue;
        }
        let value = match ch {
            'A'..='Z' => ch as u8 - b'A',
            'a'..='z' => ch as u8 - b'a' + 26,
            '0'..='9' => ch as u8 - b'0' + 52,
            '+' => 62,
            '/' => 63,
            '=' => {
                padding += 1;
                0
            }
            _ => return None,
        };
        // Значащий символ после заполнителя — испорченная запись.
        if padding > 0 && ch != '=' {
            return None;
        }
        quad[filled] = value;
        filled += 1;
        if filled == 4 {
            let triple = (u32::from(quad[0]) << 18)
                | (u32::from(quad[1]) << 12)
                | (u32::from(quad[2]) << 6)
                | u32::from(quad[3]);
            out.push((triple >> 16) as u8);
            if padding < 2 {
                out.push((triple >> 8) as u8);
            }
            if padding < 1 {
                out.push(triple as u8);
            }
            filled = 0;
        }
    }
    if filled != 0 || padding > 2 {
        return None;
    }
    Some(out)
}

// --- значения BSL --------------------------------------------------------

fn str_value(s: &str) -> BslValue {
    BslValue::Str(BslString::from_str(s))
}

fn number_value(n: i64) -> BslValue {
    BslValue::Number(bsl_number::BslNumber::from_i64(n))
}

/// `ТипЗначенияXDTO`/`ТипОбъектаXDTO` по номеру в модели.
pub fn type_value(model: &Rc<XdtoModel>, index: usize) -> BslValue {
    BslValue::Object(Rc::new(BslObject::XdtoType(model.clone(), index)))
}

fn property_value(model: &Rc<XdtoModel>, index: usize) -> BslValue {
    BslValue::Object(Rc::new(BslObject::XdtoProperty(model.clone(), index)))
}

/// `ЗначениеXDTO` из готовой пары «значение, лексическая форма».
fn data_value(model: &Rc<XdtoModel>, data: &Rc<XdtoValueData>) -> BslValue {
    BslValue::Object(Rc::new(BslObject::XdtoValue(model.clone(), data.clone())))
}

/// Границы наружу: `unbounded` — это `-1` (измерено).
fn bound_value(bound: Option<u32>) -> BslValue {
    match bound {
        Some(n) => number_value(i64::from(n)),
        None => number_value(-1),
    }
}

/// Как печатает `Строка()` от типа: `{URI}Имя`, а у безымянного
/// (анонимного) типа — пустая строка (измерено).
fn type_display(data: &XdtoTypeData) -> String {
    if data.name.is_empty() {
        return String::new();
    }
    XName {
        uri: data.ns.clone(),
        local: data.name.clone(),
    }
    .display_text()
}

/// Строковое представление значения модели типов.
pub fn display_text(obj: &BslObject) -> Option<String> {
    Some(match obj {
        BslObject::XdtoType(model, i) => match model.types.get(*i) {
            Some(data) => type_display(data),
            None => String::new(),
        },
        // Свойство печатается ИМЕНЕМ (измерено: `Строка(Свв)` -> `name`).
        BslObject::XdtoProperty(model, i) => match model.properties.get(*i) {
            Some(data) => data.name.clone(),
            None => String::new(),
        },
        // Фабрика, экземпляр, его список и последовательность печатаются
        // именем своего типа — измерено все четыре: `Строка(Фаб)` ->
        // `ФабрикаXDTO`, `Строка(Объект)` -> `ОбъектXDTO`,
        // `Строка(О.code)` -> `СписокXDTO` (и пустого, и непустого),
        // `Строка(О.Последовательность())` -> `ПоследовательностьXDTO`.
        BslObject::XdtoProperties(..)
        | BslObject::XdtoFacets(..)
        | BslObject::XdtoFacet(..)
        | BslObject::XdtoValue(..)
        | BslObject::XdtoFactory(_)
        | BslObject::XdtoObject(..)
        | BslObject::XdtoList(..)
        | BslObject::XdtoSequence(..) => type_name_of(obj)?.to_string(),
        _ => return None,
    })
}

/// Имя типа значения — то, чем зовут тип в коде.
pub fn type_name_of(obj: &BslObject) -> Option<&'static str> {
    Some(match obj {
        BslObject::XdtoType(model, i) => match model.types.get(*i) {
            Some(data) if data.is_value() => "ТипЗначенияXDTO",
            Some(_) => "ТипОбъектаXDTO",
            None => return None,
        },
        BslObject::XdtoProperty(..) => "СвойствоXDTO",
        BslObject::XdtoProperties(..) => "КоллекцияСвойствXDTO",
        BslObject::XdtoFacets(..) => "КоллекцияФасетовXDTO",
        BslObject::XdtoFacet(..) => "ФасетXDTO",
        BslObject::XdtoValue(..) => "ЗначениеXDTO",
        BslObject::XdtoFactory(_) => "ФабрикаXDTO",
        BslObject::XdtoObject(..) => "ОбъектXDTO",
        BslObject::XdtoList(..) => "СписокXDTO",
        BslObject::XdtoSequence(..) => "ПоследовательностьXDTO",
        _ => return None,
    })
}

/// `ТипЗнч()` значения модели типов.
pub fn type_id_of(obj: &BslObject) -> Option<TypeId> {
    Some(match obj {
        BslObject::XdtoType(model, i) => match model.types.get(*i) {
            Some(data) if data.is_value() => TypeId::XdtoValueType,
            Some(_) => TypeId::XdtoObjectType,
            None => return None,
        },
        BslObject::XdtoProperty(..) => TypeId::XdtoProperty,
        BslObject::XdtoProperties(..) => TypeId::XdtoPropertyCollection,
        BslObject::XdtoFacets(..) => TypeId::XdtoFacetCollection,
        BslObject::XdtoFacet(..) => TypeId::XdtoFacet,
        BslObject::XdtoValue(..) => TypeId::XdtoDataValue,
        BslObject::XdtoFactory(_) => TypeId::XdtoFactory,
        BslObject::XdtoObject(..) => TypeId::XdtoDataObject,
        BslObject::XdtoList(..) => TypeId::XdtoList,
        BslObject::XdtoSequence(..) => TypeId::XdtoSequence,
        _ => return None,
    })
}

// --- фабрика -------------------------------------------------------------

/// `ФабрикаXDTO` над готовой моделью типов.
///
/// Фабрика — это СНИМОК: `Новый ФабрикаXDTO(Наб)` строит модель на месте, и
/// схема, добавленная в тот же набор позже, ей уже не видна (измерено:
/// `Ф = Новый ФабрикаXDTO(Н); Н.Добавить(Схема); Ф.Тип(...)` ->
/// `Неопределено` — промах поиска, а не ошибка). Отсюда и `Rc<XdtoModel>`
/// вместо ссылки на набор.
pub fn factory_value(model: Rc<XdtoModel>) -> BslValue {
    BslValue::Object(Rc::new(BslObject::XdtoFactory(model)))
}

/// `ОбъектXDTO` — свежий экземпляр типа объекта: хранилище пусто, владельца
/// нет.
fn object_value(model: &Rc<XdtoModel>, index: usize) -> BslValue {
    instance_value(&Rc::new(XdtoObjectData {
        model: model.clone(),
        type_index: index,
        owner: RefCell::new(Weak::new()),
        entries: RefCell::new(Vec::new()),
    }))
}

/// Значение вокруг готового хранилища — им же отдаётся `Владелец()`.
fn instance_value(data: &Rc<XdtoObjectData>) -> BslValue {
    BslValue::Object(Rc::new(BslObject::XdtoObject(data.clone())))
}

/// `СоздатьФабрикуXDTO(Путь)` — фабрика по файлу XSD.
///
/// Источник у этой функции ровно один — путь к файлу: схему, набор схем,
/// текст схемы, число и вызов без аргументов платформа отвергает
/// (измерено все пять). Схема разбирается тем же путём, что и
/// `ПостроительСхемXML.СоздатьСхемуXML`, — второго разборщика в проекте
/// нет.
///
/// # Errors
///
/// [`RtError::Xdto`], если аргумент не строка или файла нет;
/// [`RtError::Xsd`] и [`RtError::Xml`], если содержимое файла — не схема.
pub fn factory_of_file(args: &[BslValue]) -> RtResult<BslValue> {
    let [BslValue::Str(path)] = args else {
        return Err(RtError::Xdto(
            "СоздатьФабрикуXDTO берёт один аргумент — путь к файлу XSD".to_string(),
        ));
    };
    let path = path.to_string();
    let text = std::fs::read_to_string(&path)
        .map_err(|e| RtError::Xdto(format!("файл схемы «{path}» не прочитан: {e}")))?;
    // Сигнатуру UTF-8 разборщик видит как символ перед `<` — снимаем её
    // так же, как `ЧтениеXML.ОткрытьФайл`.
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    let schema = crate::xsd::schema_of_text(text)?;
    Ok(factory_value(model_of_schemas(&[schema])?))
}

/// `Новый ФабрикаXDTO([НаборСхемXML])` — фабрика по набору схем.
///
/// Аргумент необязателен: без него получается фабрика с одними встроенными
/// типами XML Schema (измерено). Пустой набор даёт ровно её же. Всё
/// остальное — путь, схема, текст, массив — платформа отвергает
/// (измерено), и здесь так же.
///
/// # Errors
///
/// [`RtError::Xdto`], если аргумент не `НаборСхемXML`; ошибки построения
/// модели — из [`model_of_schemas`].
pub fn factory_of_schema_set(arg: &BslValue) -> RtResult<BslValue> {
    let schemas: Vec<Rc<XsSchemaData>> = match arg {
        BslValue::Undefined => Vec::new(),
        BslValue::Object(o) => match &**o {
            BslObject::XsSchemaSet(set) => set.borrow().clone(),
            _ => return Err(bad_factory_source()),
        },
        _ => return Err(bad_factory_source()),
    };
    Ok(factory_value(model_of_schemas(&schemas)?))
}

fn bad_factory_source() -> RtError {
    RtError::Xdto(
        "Новый ФабрикаXDTO берёт либо ничего, либо НаборСхемXML; \
         фабрику по файлу XSD строит СоздатьФабрикуXDTO"
            .to_string(),
    )
}

/// Фабрика ли это — нужно диспетчеру методов: имя `Создать` делят фабрика
/// и менеджер файловых потоков.
pub fn is_factory(v: &BslValue) -> bool {
    matches!(v, BslValue::Object(o) if matches!(&**o, BslObject::XdtoFactory(_)))
}

/// Экземпляр `ОбъектXDTO` ли это.
pub fn is_object(v: &BslValue) -> bool {
    matches!(v, BslValue::Object(o) if matches!(&**o, BslObject::XdtoObject(..)))
}

/// `ЗначениеXDTO` ли это — у него свой `Тип()`, как у экземпляра.
pub fn is_value(v: &BslValue) -> bool {
    matches!(v, BslValue::Object(o) if matches!(&**o, BslObject::XdtoValue(..)))
}

/// `СписокXDTO` ли это — нужно диспетчеру методов: имена `Добавить`,
/// `Получить`, `Установить` делят между собой все коллекции рантайма.
pub fn is_list(v: &BslValue) -> bool {
    matches!(v, BslValue::Object(o) if matches!(&**o, BslObject::XdtoList(..)))
}

/// `ПоследовательностьXDTO` ли это.
pub fn is_sequence(v: &BslValue) -> bool {
    matches!(v, BslValue::Object(o) if matches!(&**o, BslObject::XdtoSequence(..)))
}

fn not_applicable(obj: &BslValue, method: &'static str) -> RtError {
    RtError::MethodNotApplicable {
        method,
        receiver: obj.type_name(),
    }
}

/// Модель фабрики-получателя.
fn factory_model<'a>(obj: &'a BslValue, method: &'static str) -> RtResult<&'a Rc<XdtoModel>> {
    match obj {
        BslValue::Object(o) => match &**o {
            BslObject::XdtoFactory(model) => Ok(model),
            _ => Err(not_applicable(obj, method)),
        },
        _ => Err(not_applicable(obj, method)),
    }
}

/// `ФабрикаXDTO.Тип(URI, Имя)` и `ФабрикаXDTO.Тип(РасширенноеИмяXML)`.
///
/// Неизвестное имя — `Неопределено`, а не ошибка (измерено, как и то, что
/// объявление глобального ЭЛЕМЕНТА типом не является). Одна строка вместо
/// пары, три аргумента, числа вместо имён — ошибка (измерено все четыре
/// пробы).
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не фабрика либо
/// аргументы не той формы.
pub fn factory_type(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let model = factory_model(obj, "Тип")?;
    let found = match args {
        [BslValue::Str(uri), BslValue::Str(name)] => {
            model.find(&uri.to_string(), &name.to_string())
        }
        [BslValue::Object(o)] => match &**o {
            BslObject::XmlExpandedName(name) => model.find(&name.uri, &name.local),
            _ => return Err(not_applicable(obj, "Тип")),
        },
        _ => return Err(not_applicable(obj, "Тип")),
    };
    Ok(match found {
        Some(index) => type_value(model, index),
        None => BslValue::Undefined,
    })
}

/// `ФабрикаXDTO.Создать(Тип[, Лексика])`.
///
/// Смысл вызова решает вид типа, и оба измерены. У типа ЗНАЧЕНИЯ вызов без
/// лексической формы отдаёт `Неопределено`, а с формой — `ЗначениеXDTO`,
/// разобранное тем же путём, что и `ЗначениеПоУмолчанию` свойства. У типа
/// ОБЪЕКТА лексической формы быть не должно (`Создать(ТипОбъекта, "аб")` —
/// ошибка), а результат — `ОбъектXDTO`; абстрактный тип платформа
/// инстанцировать отказывается.
///
/// ФАСЕТЫ ЗДЕСЬ НЕ ПРОВЕРЯЮТСЯ. Платформа по ним лексическую форму
/// проверяет (измерено: `Создать` от `Small` с «1000» и от `Code` с «аб» —
/// ошибка), и это сознательно отложено до задачи проверки значений — см.
/// «Фасеты только хранятся» в шапке модуля.
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не фабрика, первый
/// аргумент не тип XDTO либо аргументов не то количество;
/// [`RtError::Xdto`], если лексическая форма не разбирается в этом типе или
/// тип объекта абстрактный.
pub fn factory_create(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    factory_model(obj, "Создать")?;
    let [first, rest @ ..] = args else {
        return Err(not_applicable(obj, "Создать"));
    };
    // Третий аргумент платформа принимает (измерено: `Создать(Тип, "42",
    // 1)` отдаёт `ЗначениеXDTO`), а четвёртый уже нет. Что он значит, не
    // измерено, поэтому здесь он принимается и ни на что не влияет:
    // додумывать ему смысл хуже, чем отвергать программу, которая на
    // платформе работает.
    if rest.len() > 2 {
        return Err(not_applicable(obj, "Создать"));
    }
    let BslValue::Object(o) = first else {
        return Err(not_applicable(obj, "Создать"));
    };
    let BslObject::XdtoType(model, index) = &**o else {
        return Err(not_applicable(obj, "Создать"));
    };
    // Модель берётся у САМОГО типа, а не у фабрики-получателя: тип и так
    // несёт свою модель, и чужой тип строил бы значение по своей. Что
    // платформа делает с типом из другой фабрики, не измерено.
    let data = model.type_at(*index)?;
    if !data.is_value() {
        if !rest.is_empty() {
            return Err(not_applicable(obj, "Создать"));
        }
        if data.is_abstract {
            return Err(RtError::Xdto(format!(
                "абстрактный тип «{}» экземпляров не имеет",
                type_display(data)
            )));
        }
        return Ok(object_value(model, *index));
    }
    let Some(lexical) = rest.first() else {
        // Тип значения без лексической формы — `Неопределено` (измерено).
        return Ok(BslValue::Undefined);
    };
    let BslValue::Str(text) = lexical else {
        return Err(RtError::Xdto(
            "лексическая форма значения XDTO — это строка".to_string(),
        ));
    };
    let text = text.to_string();
    Ok(data_value(
        model,
        &Rc::new(XdtoValueData {
            value: value_from_lexical(model, *index, &text)?,
            lexical: text,
            type_index: *index,
        }),
    ))
}

/// `ОбъектXDTO.Тип()` — свой тип XDTO. Именно МЕТОД: обращение к `Тип` как
/// к свойству платформа отвергает, а `Тип()` отдаёт тот же тип, что и
/// `Фабрика.Тип(URI, Имя)` (измерено обе стороны, включая равенство).
/// Аргументов у него нет — `Тип(1)` платформа не берёт.
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не `ОбъектXDTO` либо
/// вызов с аргументами.
pub fn object_type(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    if !args.is_empty() {
        return Err(not_applicable(obj, "Тип"));
    }
    // `ЗначениеXDTO` отвечает на `Тип()` так же — типом, по которому
    // разобрана его лексическая форма (измерено: `Создать(xs:int, "42")
    // .Тип()` -> `{...}int`).
    if let BslValue::Object(o) = obj {
        if let BslObject::XdtoValue(model, data) = &**o {
            model.type_at(data.type_index)?;
            return Ok(type_value(model, data.type_index));
        }
    }
    let data = instance_of(obj, "Тип")?;
    // Номер проверяется до построения значения: испорченная модель обязана
    // отвечать ошибкой, а не типом, которого нет.
    data.type_data()?;
    Ok(type_value(&data.model, data.type_index))
}

/// Член `ВидФасетаXDTO` по виду фасета лексической модели XSD.
fn facet_kind_value(kind: FacetKind) -> EnumValue {
    match kind {
        FacetKind::Length => EnumValue::XdtoFacetLength,
        FacetKind::MinLength => EnumValue::XdtoFacetMinLength,
        FacetKind::MaxLength => EnumValue::XdtoFacetMaxLength,
        FacetKind::Pattern => EnumValue::XdtoFacetPattern,
        FacetKind::Enumeration => EnumValue::XdtoFacetEnumeration,
        FacetKind::WhiteSpace => EnumValue::XdtoFacetWhiteSpace,
        FacetKind::TotalDigits => EnumValue::XdtoFacetTotalDigits,
        FacetKind::FractionDigits => EnumValue::XdtoFacetFractionDigits,
        FacetKind::MinInclusive => EnumValue::XdtoFacetMinInclusive,
        FacetKind::MaxInclusive => EnumValue::XdtoFacetMaxInclusive,
        FacetKind::MinExclusive => EnumValue::XdtoFacetMinExclusive,
        FacetKind::MaxExclusive => EnumValue::XdtoFacetMaxExclusive,
    }
}

/// Свойство значения модели типов.
///
/// # Errors
///
/// [`RtError::UnknownColumn`], если такого члена у этого вида значения
/// нет; [`RtError::Xdto`], если модель ссылается на несуществующий узел.
pub fn get_property(obj: &BslValue, name: &str) -> RtResult<BslValue> {
    let unknown = || RtError::UnknownColumn(name.to_string());
    // Сравнение — через `fold`, а не `eq_ignore_ascii_case`: имена членов
    // здесь РУССКИЕ, а ASCII-свёртка кириллицу не трогает. Имя приходит в
    // том написании, в каком его первым увидел интерн полей, и `Значение`
    // в скрипте, где раньше встретилось `значение`, доходит сюда строчным.
    let is =
        |ru: &str, en: &str| crate::fold::folded_eq(name, ru) || crate::fold::folded_eq(name, en);
    let BslValue::Object(o) = obj else {
        return Err(unknown());
    };
    match &**o {
        BslObject::XdtoType(model, i) => {
            let data = model.type_at(*i)?;
            if is("Имя", "Name") {
                return Ok(str_value(&data.name));
            }
            if is("URIПространстваИмен", "NamespaceURI") {
                return Ok(str_value(&data.ns));
            }
            if is("БазовыйТип", "BaseType") {
                return Ok(match data.base {
                    Some(b) => type_value(model, b),
                    None => BslValue::Undefined,
                });
            }
            if data.is_value() {
                if is("Фасеты", "Facets") {
                    // У типа БЕЗ фасетов это `Неопределено`, а не пустая
                    // коллекция (измерено на `xs:date`).
                    return Ok(if data.facets.is_empty() {
                        BslValue::Undefined
                    } else {
                        BslValue::Object(Rc::new(BslObject::XdtoFacets(model.clone(), *i)))
                    });
                }
                return Err(unknown());
            }
            if is("Свойства", "Properties") {
                return Ok(BslValue::Object(Rc::new(BslObject::XdtoProperties(
                    model.clone(),
                    *i,
                ))));
            }
            if is("Открытый", "Open") {
                return Ok(BslValue::Boolean(data.open));
            }
            if is("Абстрактный", "Abstract") {
                return Ok(BslValue::Boolean(data.is_abstract));
            }
            if is("Упорядоченный", "Ordered") {
                return Ok(BslValue::Boolean(data.ordered));
            }
            if is("Последовательный", "Sequenced") {
                return Ok(BslValue::Boolean(data.sequenced()));
            }
            if is("Смешанный", "Mixed") {
                return Ok(BslValue::Boolean(data.mixed));
            }
            Err(unknown())
        }
        BslObject::XdtoProperty(model, i) => {
            let data = model.property_at(*i)?;
            if is("Имя", "Name") {
                return Ok(str_value(&data.name));
            }
            if is("URIПространстваИмен", "NamespaceURI") {
                return Ok(str_value(&data.ns));
            }
            if is("Тип", "Type") {
                return Ok(type_value(model, data.type_index));
            }
            if is("НижняяГраница", "LowerBound") {
                return Ok(bound_value(data.lower));
            }
            if is("ВерхняяГраница", "UpperBound") {
                return Ok(bound_value(data.upper));
            }
            if is("Форма", "Form") {
                return Ok(BslValue::Enum(data.form));
            }
            if is("ЗначениеПоУмолчанию", "DefaultValue") {
                return Ok(match &data.default {
                    Some(v) => data_value(model, v),
                    None => BslValue::Undefined,
                });
            }
            Err(unknown())
        }
        BslObject::XdtoFacet(model, type_index, facet_index) => {
            let data = model.type_at(*type_index)?;
            let (kind, lexical) = data
                .facets
                .get(*facet_index)
                .ok_or_else(|| broken("фасет"))?;
            // Английское имя `Вид` — `Type`, а не `Kind`: `Kind` платформа
            // отвергает (измерено обе пробы).
            if is("Вид", "Type") {
                return Ok(BslValue::Enum(facet_kind_value(*kind)));
            }
            // `Значение` фасета — ВСЕГДА строка, даже у числовых
            // (измерено).
            if is("Значение", "Value") {
                return Ok(str_value(lexical));
            }
            Err(unknown())
        }
        BslObject::XdtoValue(_, data) => {
            if is("Значение", "Value") {
                return Ok(data.value.clone());
            }
            if is("ЛексическоеЗначение", "LexicalValue") {
                return Ok(str_value(&data.lexical));
            }
            Err(unknown())
        }
        // Своих читаемых членов у фабрики нет: `Тип` и `Создать` — методы,
        // а на постороннее имя платформа отвечает ошибкой (измерено
        // `Фаб.НетТакогоЧлена`). `Пакеты` этой реализацией не поддержаны.
        BslObject::XdtoFactory(_) => Err(unknown()),
        // У экземпляра члены — это СВОЙСТВА ЕГО ТИПА: `Тип`, `Владелец`,
        // `Свойства` читаются методами, а не точкой (измерено — все три
        // как члены отвергнуты).
        BslObject::XdtoObject(data) => object_get_property(data, name),
        // А вот у списка и последовательности владелец, наоборот, ЧЛЕН:
        // `Список.Владелец` даёт объект, `Список.Владелец()` — ошибка
        // (измерено обе пробы, и то же самое у последовательности).
        BslObject::XdtoList(data, _) | BslObject::XdtoSequence(data) => {
            if is("Владелец", "Owner") {
                return Ok(instance_value(data));
            }
            Err(unknown())
        }
        _ => Err(unknown()),
    }
}

/// Длина коллекции свойств или фасетов.
///
/// # Errors
///
/// [`RtError::Xdto`], если модель ссылается на несуществующий тип.
pub fn collection_len(obj: &BslObject) -> Option<RtResult<usize>> {
    match obj {
        BslObject::XdtoProperties(model, i) => {
            Some(model.type_at(*i).map(|data| data.properties.len()))
        }
        BslObject::XdtoFacets(model, i) => Some(model.type_at(*i).map(|data| data.facets.len())),
        // Длина есть и у экземплярных коллекций: `Количество()` измерено и
        // у списка, и у последовательности. Разница между ними в другом —
        // список ещё и ИНДЕКСИРУЕТСЯ, а последовательность нет, поэтому
        // `Для Каждого` по ней платформа отвергает (измерено).
        BslObject::XdtoList(data, prop) => Some(Ok(list_len(data, *prop))),
        BslObject::XdtoSequence(data) => Some(sequence_len(data)),
        _ => None,
    }
}

/// Элемент коллекции по номеру.
///
/// # Errors
///
/// [`RtError::IndexOutOfBounds`], если номер за границей.
pub fn collection_get(obj: &BslObject, i: usize) -> RtResult<BslValue> {
    match obj {
        BslObject::XdtoProperties(model, t) => {
            let data = model.type_at(*t)?;
            match data.properties.get(i) {
                Some(p) => Ok(property_value(model, *p)),
                None => Err(RtError::IndexOutOfBounds {
                    index: i as i64,
                    len: data.properties.len(),
                }),
            }
        }
        BslObject::XdtoFacets(model, t) => {
            let data = model.type_at(*t)?;
            if i < data.facets.len() {
                Ok(BslValue::Object(Rc::new(BslObject::XdtoFacet(
                    model.clone(),
                    *t,
                    i,
                ))))
            } else {
                Err(RtError::IndexOutOfBounds {
                    index: i as i64,
                    len: data.facets.len(),
                })
            }
        }
        // `Список[i]` и `Для Каждого` по списку — измерены оба.
        BslObject::XdtoList(data, prop) => list_item(data, *prop, i),
        _ => Err(RtError::NotIndexable),
    }
}

/// `Получить` у коллекции свойств (имя или номер) и у коллекции фасетов
/// (только номер — поиск по имени платформа отвергает, измерено).
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не коллекция модели
/// типов или аргумент не тот; [`RtError::IndexOutOfBounds`] на номере за
/// границей.
pub fn collection_lookup(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let not_applicable = || RtError::MethodNotApplicable {
        method: "Получить",
        receiver: obj.type_name(),
    };
    let BslValue::Object(o) = obj else {
        return Err(not_applicable());
    };
    let [arg] = args else {
        return Err(not_applicable());
    };
    match (&**o, arg) {
        (BslObject::XdtoProperties(model, t), BslValue::Str(s)) => {
            let data = model.type_at(*t)?;
            let name = s.to_string();
            // Неизвестное имя — `Неопределено`, а не ошибка (измерено).
            Ok(data
                .properties
                .iter()
                .find(|p| {
                    model
                        .properties
                        .get(**p)
                        .is_some_and(|prop| prop.name == name)
                })
                .map_or(BslValue::Undefined, |p| property_value(model, *p)))
        }
        (BslObject::XdtoProperties(..) | BslObject::XdtoFacets(..), BslValue::Number(n)) => {
            let index = n.to_i64_exact().ok_or_else(not_applicable)?;
            let len = match collection_len(o) {
                Some(len) => len?,
                None => return Err(not_applicable()),
            };
            let index =
                usize::try_from(index).map_err(|_| RtError::IndexOutOfBounds { index, len })?;
            collection_get(o, index)
        }
        _ => Err(not_applicable()),
    }
}

// --- экземпляр -----------------------------------------------------------

/// Хранилище экземпляра `ОбъектXDTO`.
///
/// Слоты здесь параллельны не списку свойств типа, а ПОРЯДКУ ЗАПОЛНЕНИЯ:
/// хранится список записей «свойство — значение». Так устроено потому, что
/// именно этот порядок платформа показывает через `Последовательность()`
/// (измерено: `О.cb.Добавить("аб"); О.ca = "вг"; О.cb.Добавить("де")` даёт
/// `[cb=аб][ca=вг][cb=де]`), а повторная запись одиночного свойства своё
/// место в нём СОХРАНЯЕТ (измерено: `О.ca = "вг"; О.cb.Добавить("аб");
/// О.ca = "де"` -> `[ca=де][cb=аб]`, а после `Сбросить` то же свойство
/// уходит в конец). Массив слотов по числу свойств этот порядок потерял бы,
/// а он наблюдаем.
#[derive(Debug)]
pub struct XdtoObjectData {
    model: Rc<XdtoModel>,
    type_index: usize,
    /// `Владелец()` — объект, в свойство которого этот экземпляр записан
    /// (измерено: `О.anon = А; А.Владелец() = О` — «Да», у отдельно
    /// созданного — `Неопределено`). Ссылка СЛАБАЯ: владелец держит этот
    /// экземпляр в своём хранилище, и сильная обратная ссылка замкнула бы
    /// `Rc` в кольцо, которого счётчик ссылок не разбирает.
    owner: RefCell<Weak<XdtoObjectData>>,
    entries: RefCell<Vec<XdtoEntry>>,
}

/// Одно заполнение свойства. Множественное свойство — это несколько
/// записей с одним и тем же `prop`.
#[derive(Debug)]
struct XdtoEntry {
    /// Номер свойства в [`XdtoModel::properties`].
    prop: usize,
    value: BslValue,
}

impl XdtoObjectData {
    fn type_data(&self) -> RtResult<&XdtoTypeData> {
        self.model.type_at(self.type_index)
    }

    /// Номер свойства по имени: поиск идёт по СПЛЮЩЕННОМУ списку типа и
    /// регистра не различает (измерено: `О.NAME` читает `name`).
    fn property_by_name(&self, name: &str) -> RtResult<Option<usize>> {
        for p in &self.type_data()?.properties {
            if crate::fold::folded_eq(&self.model.property_at(*p)?.name, name) {
                return Ok(Some(*p));
            }
        }
        Ok(None)
    }

    /// Номера записей хранилища, относящихся к свойству, в порядке
    /// заполнения.
    fn occurrences(&self, prop: usize) -> Vec<usize> {
        self.entries
            .borrow()
            .iter()
            .enumerate()
            .filter(|(_, e)| e.prop == prop)
            .map(|(i, _)| i)
            .collect()
    }

    /// Чтение ОДИНОЧНОГО свойства: последнее заполнение, а если его нет —
    /// значение по умолчанию из `default`/`fixed` (измерено: у свежего
    /// объекта `def` -> 7, `fx` -> 9, `color` -> «red», а свойство без
    /// того и другого -> `Неопределено`).
    fn single_value(&self, prop: usize) -> RtResult<BslValue> {
        if let Some(e) = self.entries.borrow().iter().rev().find(|e| e.prop == prop) {
            return Ok(e.value.clone());
        }
        Ok(match &self.model.property_at(prop)?.default {
            Some(v) => v.value.clone(),
            None => BslValue::Undefined,
        })
    }
}

/// Множественное ли свойство: верхняя граница, отличная от единицы.
/// Измерено на границах `0..-1` (`code`) и `1..5` (`many5`) — оба дают
/// `СписокXDTO`, а `1..1` и `0..1` — само значение.
fn is_multiple(prop: &XdtoPropertyData) -> bool {
    prop.upper != Some(1)
}

/// Хранилище получателя-экземпляра.
fn instance_of<'a>(obj: &'a BslValue, method: &'static str) -> RtResult<&'a Rc<XdtoObjectData>> {
    match obj {
        BslValue::Object(o) => match &**o {
            BslObject::XdtoObject(data) => Ok(data),
            _ => Err(not_applicable(obj, method)),
        },
        _ => Err(not_applicable(obj, method)),
    }
}

/// Хранилище и свойство получателя-списка.
fn list_of<'a>(
    obj: &'a BslValue,
    method: &'static str,
) -> RtResult<(&'a Rc<XdtoObjectData>, usize)> {
    match obj {
        BslValue::Object(o) => match &**o {
            BslObject::XdtoList(data, prop) => Ok((data, *prop)),
            _ => Err(not_applicable(obj, method)),
        },
        _ => Err(not_applicable(obj, method)),
    }
}

/// Свойство, названное аргументом: платформа принимает и имя строкой, и
/// само `СвойствоXDTO` (измерено на `Получить`, `Установлено` и
/// `Сбросить`). Постороннее имя — ошибка, а не `Неопределено` (измерено).
fn property_arg(data: &XdtoObjectData, arg: &BslValue, method: &'static str) -> RtResult<usize> {
    let unknown = |name: String| {
        RtError::Xdto(format!(
            "у типа «{}» нет свойства «{name}»",
            data.type_data()
                .map(type_display)
                .unwrap_or_else(|_| String::new())
        ))
    };
    match arg {
        BslValue::Str(s) => {
            let name = s.to_string();
            data.property_by_name(&name)?.ok_or_else(|| unknown(name))
        }
        BslValue::Object(o) => match &**o {
            BslObject::XdtoProperty(model, index) => {
                // Чужое свойство — из другой модели или из другого типа —
                // не годится: хранилище адресуется номерами СВОЕЙ модели.
                if !Rc::ptr_eq(model, &data.model) || !data.type_data()?.properties.contains(index)
                {
                    return Err(unknown(
                        model
                            .property_at(*index)
                            .map(|p| p.name.clone())
                            .unwrap_or_default(),
                    ));
                }
                Ok(*index)
            }
            _ => Err(RtError::MethodNotApplicable {
                method,
                receiver: arg.type_name(),
            }),
        },
        _ => Err(RtError::MethodNotApplicable {
            method,
            receiver: arg.type_name(),
        }),
    }
}

/// `anyType` ли это — единственный тип, который принимает в свойство любое
/// значение как есть (измерено на `notype`: строка остаётся строкой, число
/// числом, `ОбъектXDTO` объектом).
fn is_any_type(data: &XdtoTypeData) -> bool {
    data.shape.is_none() && data.base.is_none()
}

/// Наследник ли `candidate` типу `target` — цепочкой базовых типов.
fn derives_from(model: &XdtoModel, candidate: usize, target: usize) -> bool {
    let mut cur = candidate;
    // Длина модели — верхняя граница цепочки и заодно страховка от кольца
    // в испорченной схеме.
    for _ in 0..=model.types.len() {
        if cur == target {
            return true;
        }
        match model.types.get(cur).and_then(|t| t.base) {
            Some(base) => cur = base,
            None => return false,
        }
    }
    false
}

/// Приведение значения к типу свойства — то, что платформа делает при
/// записи.
///
/// Правило одно на все измеренные случаи: значение переводится в
/// ЛЕКСИЧЕСКУЮ ФОРМУ типа-приёмника и разбирается обратно. Отсюда и
/// `О.name = 5` -> строка «5», и `О.name = Дата(2026,8,13)` ->
/// «2026-08-13T00:00:00», и `О.name = Истина` -> «true», и отказы: «true»
/// не разбирается как `xs:int` (`О.id = Истина` — ошибка), «1.5» тоже
/// (`О.id = 1.5` — ошибка), а `О.id = "5"` даёт число 5. Объединение
/// выбирает член по той же лексической форме (измерено: `5` и «5» дают
/// число, «аб» — строку).
fn coerce_to_property(
    owner: &Rc<XdtoObjectData>,
    prop: usize,
    value: BslValue,
) -> RtResult<BslValue> {
    let model = &owner.model;
    let target = model.property_at(prop)?.type_index;
    let data = model.type_at(target)?;
    if data.shape.is_none() {
        // `anyType` принимает что угодно (измерено), остальные типы
        // объектов — только экземпляр своего типа или его наследника
        // (измерено: `ExtType` в свойство типа `RootType` проходит,
        // `EmptyType` — ошибка).
        if is_any_type(data) {
            return Ok(value);
        }
        let BslValue::Object(o) = &value else {
            return Err(bad_property_value(model, target, &value));
        };
        let BslObject::XdtoObject(inst) = &**o else {
            return Err(bad_property_value(model, target, &value));
        };
        if !Rc::ptr_eq(&inst.model, model) || !derives_from(model, inst.type_index, target) {
            return Err(bad_property_value(model, target, &value));
        }
        *inst.owner.borrow_mut() = Rc::downgrade(owner);
        return Ok(value);
    }
    // `ЗначениеXDTO` в свойство простого типа платформа принимает и берёт
    // из него ЗНАЧЕНИЕ, а не лексическую форму: `О.id = Создать(xs:string,
    // "5")` дало число 5, хотя типы источника и приёмника разные
    // (измерено).
    let value = match &value {
        BslValue::Object(o) => match &**o {
            BslObject::XdtoValue(_, v) => v.value.clone(),
            _ => value,
        },
        _ => value,
    };
    coerce_to_value_type(model, target, value)
}

/// Значение простого типа из значения BSL — лексический круг из
/// [`coerce_to_property`].
fn coerce_to_value_type(
    model: &Rc<XdtoModel>,
    target: usize,
    value: BslValue,
) -> RtResult<BslValue> {
    let builtin = model.builtin_of(target);
    // Двоичные данные и расширенное имя проходят без круга: обратной
    // лексической формы у них здесь нет (см. «Двоичные лексические формы» и
    // «QName с префиксом» в шапке модуля), а значение уже нужного вида.
    if let BslValue::Object(o) = &value {
        match (builtin, &**o) {
            (Some(BuiltinBsl::Base64 | BuiltinBsl::Hex), BslObject::BinaryData(_))
            | (Some(BuiltinBsl::QName), BslObject::XmlExpandedName(_)) => return Ok(value),
            _ => {}
        }
    }
    let lexical = lexical_of_value(&value, builtin)
        .ok_or_else(|| bad_property_value(model, target, &value))?;
    value_from_lexical(model, target, &lexical)
}

/// Лексическая форма значения BSL для типа-приёмника. `None` — значение,
/// у которого лексической формы нет вовсе: `Неопределено`, `Null` и любой
/// объект (измерено: запись `Неопределено`, `Null` и `ОбъектXDTO` в
/// свойство простого типа — ошибка).
fn lexical_of_value(value: &BslValue, target: Option<BuiltinBsl>) -> Option<String> {
    Some(match value {
        BslValue::Str(s) => s.to_string(),
        BslValue::Number(n) => n.to_canonical(),
        BslValue::Boolean(b) => (if *b { "true" } else { "false" }).to_string(),
        BslValue::Date(d) => {
            let c = d.to_civil();
            match target {
                // У приёмника-даты и приёмника-времени формы свои, у всех
                // остальных (в том числе у `xs:string`) — полная запись
                // `dateTime` (измерено: `О.name = Дата(2026,8,13)` дало
                // «2026-08-13T00:00:00»).
                Some(BuiltinBsl::Date) => format!("{:04}-{:02}-{:02}", c.year, c.month, c.day),
                Some(BuiltinBsl::Time) => {
                    format!("{:02}:{:02}:{:02}", c.hour, c.minute, c.second)
                }
                _ => format!(
                    "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
                    c.year, c.month, c.day, c.hour, c.minute, c.second
                ),
            }
        }
        _ => return None,
    })
}

fn bad_property_value(model: &Rc<XdtoModel>, target: usize, value: &BslValue) -> RtError {
    let name = model
        .type_at(target)
        .map(type_display)
        .unwrap_or_else(|_| String::new());
    RtError::Xdto(format!(
        "значение типа «{}» не годится свойству типа «{name}»",
        value.type_name()
    ))
}

/// Запись ОДИНОЧНОГО свойства: место в порядке заполнения сохраняется, а
/// незаполненное свойство встаёт в конец (измерено обе стороны).
fn set_single(data: &Rc<XdtoObjectData>, prop: usize, value: BslValue) -> RtResult<()> {
    let coerced = coerce_to_property(data, prop, value)?;
    let mut entries = data.entries.borrow_mut();
    match entries.iter_mut().rev().find(|e| e.prop == prop) {
        Some(e) => e.value = coerced,
        None => entries.push(XdtoEntry {
            prop,
            value: coerced,
        }),
    }
    Ok(())
}

/// Чтение свойства экземпляра.
///
/// # Errors
///
/// [`RtError::UnknownColumn`], если у типа нет такого свойства (измерено:
/// постороннее имя — ошибка, а не `Неопределено`).
fn object_get_property(data: &Rc<XdtoObjectData>, name: &str) -> RtResult<BslValue> {
    let Some(prop) = data.property_by_name(name)? else {
        return Err(RtError::UnknownColumn(name.to_string()));
    };
    if is_multiple(data.model.property_at(prop)?) {
        return Ok(BslValue::Object(Rc::new(BslObject::XdtoList(
            data.clone(),
            prop,
        ))));
    }
    data.single_value(prop)
}

/// Запись свойства экземпляра.
///
/// # Errors
///
/// [`RtError::UnknownColumn`], если такого свойства нет; [`RtError::Xdto`],
/// если свойство множественное (оно наполняется через `СписокXDTO` —
/// измерено, что присваивание в него ошибка) либо значение не приводится к
/// типу свойства.
pub fn set_property(obj: &BslValue, name: &str, value: BslValue) -> RtResult<()> {
    let data = instance_of(obj, "Установить")?;
    let Some(prop) = data.property_by_name(name)? else {
        return Err(RtError::UnknownColumn(name.to_string()));
    };
    if is_multiple(data.model.property_at(prop)?) {
        return Err(RtError::Xdto(format!(
            "множественное свойство «{name}» наполняется через «СписокXDTO», \
             а не присваиванием"
        )));
    }
    set_single(data, prop, value)
}

/// `ОбъектXDTO.Получить(Имя|Свойство)` — только ОДИНОЧНОЕ свойство:
/// множественное платформа через `Получить` не отдаёт (измерено), для него
/// есть `ПолучитьСписок`.
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`] на чужом получателе или аргументе,
/// [`RtError::Xdto`], если свойства нет или оно множественное.
pub fn object_get(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let data = instance_of(obj, "Получить")?;
    let [arg] = args else {
        return Err(not_applicable(obj, "Получить"));
    };
    let prop = property_arg(data, arg, "Получить")?;
    if is_multiple(data.model.property_at(prop)?) {
        return Err(RtError::Xdto(format!(
            "множественное свойство «{}» читается методом «ПолучитьСписок»",
            data.model.property_at(prop)?.name
        )));
    }
    data.single_value(prop)
}

/// `ОбъектXDTO.Установить(Имя|Свойство, Значение)` — тоже только
/// одиночное свойство (измерено: `Установить("code", ...)` — ошибка).
///
/// # Errors
///
/// Те же, что у [`set_property`].
pub fn object_set(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let data = instance_of(obj, "Установить")?;
    let [arg, value] = args else {
        return Err(not_applicable(obj, "Установить"));
    };
    let prop = property_arg(data, arg, "Установить")?;
    if is_multiple(data.model.property_at(prop)?) {
        return Err(RtError::Xdto(format!(
            "множественное свойство «{}» наполняется через «СписокXDTO»",
            data.model.property_at(prop)?.name
        )));
    }
    set_single(data, prop, value.clone())?;
    Ok(BslValue::Undefined)
}

/// `ОбъектXDTO.ПолучитьСписок(Имя|Свойство)` — тот же список, что отдаёт
/// чтение множественного свойства (измерено: `О.ПолучитьСписок("code")
/// .Добавить(...)` видно через `О.code`). У одиночного свойства списка нет
/// (измерено — ошибка).
///
/// # Errors
///
/// [`RtError::Xdto`], если свойства нет или оно одиночное.
pub fn object_get_list(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let data = instance_of(obj, "ПолучитьСписок")?;
    let [arg] = args else {
        return Err(not_applicable(obj, "ПолучитьСписок"));
    };
    let prop = property_arg(data, arg, "ПолучитьСписок")?;
    if !is_multiple(data.model.property_at(prop)?) {
        return Err(RtError::Xdto(format!(
            "свойство «{}» одиночное, списка у него нет",
            data.model.property_at(prop)?.name
        )));
    }
    Ok(BslValue::Object(Rc::new(BslObject::XdtoList(
        data.clone(),
        prop,
    ))))
}

/// `ОбъектXDTO.Установлено(Имя|Свойство)`.
///
/// Заполненность — это ЗАПИСЬ в хранилище, а не наличие значения при
/// чтении: у свежего объекта `Установлено("def")` — «Нет», хотя `О.def`
/// отдаёт 7 из `default` (измерено).
///
/// # Errors
///
/// [`RtError::Xdto`], если у типа нет такого свойства.
pub fn object_is_set(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let data = instance_of(obj, "Установлено")?;
    let [arg] = args else {
        return Err(not_applicable(obj, "Установлено"));
    };
    let prop = property_arg(data, arg, "Установлено")?;
    Ok(BslValue::Boolean(!data.occurrences(prop).is_empty()))
}

/// `ОбъектXDTO.Сбросить(Имя|Свойство)` — забыть заполнение: после сброса
/// свойство с `default` снова отдаёт значение по умолчанию, а
/// множественное становится пустым (измерено).
///
/// # Errors
///
/// [`RtError::Xdto`], если у типа нет такого свойства.
pub fn object_unset(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let data = instance_of(obj, "Сбросить")?;
    let [arg] = args else {
        return Err(not_applicable(obj, "Сбросить"));
    };
    let prop = property_arg(data, arg, "Сбросить")?;
    data.entries.borrow_mut().retain(|e| e.prop != prop);
    Ok(BslValue::Undefined)
}

/// `ОбъектXDTO.Свойства()` — свойства СВОЕГО ТИПА, та же коллекция, что и
/// `Тип().Свойства` (измерено: длины совпадают — 14 и 14).
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не экземпляр либо
/// вызов с аргументами.
pub fn object_properties(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let data = instance_of(obj, "Свойства")?;
    if !args.is_empty() {
        return Err(not_applicable(obj, "Свойства"));
    }
    data.type_data()?;
    Ok(BslValue::Object(Rc::new(BslObject::XdtoProperties(
        data.model.clone(),
        data.type_index,
    ))))
}

/// `ОбъектXDTO.Владелец()` — объект, в свойство которого этот записан;
/// у отдельно созданного — `Неопределено` (измерено обе стороны).
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не экземпляр либо
/// вызов с аргументами. У `СписокXDTO` и `ПоследовательностьXDTO`
/// владелец — ЧЛЕН, а не метод (измерено: `Список.Владелец()` — ошибка),
/// поэтому они сюда не попадают.
pub fn object_owner(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let data = instance_of(obj, "Владелец")?;
    if !args.is_empty() {
        return Err(not_applicable(obj, "Владелец"));
    }
    Ok(match data.owner.borrow().upgrade() {
        Some(owner) => instance_value(&owner),
        None => BslValue::Undefined,
    })
}

/// `ОбъектXDTO.Проверить()` — проверка ГРАНИЦ ВХОЖДЕНИЯ, своих и
/// вложенных.
///
/// Измерено четырьмя пробами: пустой тип проходит; `RootType` без
/// обязательных свойств отвергается; заполненный, но с пустым вложенным
/// объектом — отвергается, а с заполненным — проходит; шесть значений в
/// свойстве `1..5` — отвергается. Отсюда и правило: у каждого свойства
/// число заполнений обязано лежать между `НижняяГраница` и
/// `ВерхняяГраница`, а вложенные объекты — и в свойстве, и в списке —
/// проверяются рекурсивно.
///
/// ФАСЕТЫ здесь не проверяются, как и в `Создать`: значение, нарушающее
/// фасет, платформа не пускает уже в запись, а этой реализации проверять
/// образцы нечем (см. «Фасеты только хранятся» в шапке модуля).
///
/// # Errors
///
/// [`RtError::Xdto`], если какая-нибудь граница нарушена.
pub fn object_validate(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let data = instance_of(obj, "Проверить")?;
    if !args.is_empty() {
        return Err(not_applicable(obj, "Проверить"));
    }
    validate_instance(data, data.model.types.len() + 8)?;
    Ok(BslValue::Undefined)
}

fn validate_instance(data: &Rc<XdtoObjectData>, depth: usize) -> RtResult<()> {
    let Some(depth) = depth.checked_sub(1) else {
        return Err(RtError::Xdto(
            "объекты XDTO вложены друг в друга кольцом".to_string(),
        ));
    };
    for p in &data.type_data()?.properties {
        let prop = data.model.property_at(*p)?;
        let places = data.occurrences(*p);
        let count = u32::try_from(places.len()).unwrap_or(u32::MAX);
        if count < prop.lower.unwrap_or(0) {
            return Err(RtError::Xdto(format!(
                "свойство «{}» обязано входить не менее {} раз, а заполнено {count}",
                prop.name,
                prop.lower.unwrap_or(0)
            )));
        }
        if let Some(upper) = prop.upper {
            if count > upper {
                return Err(RtError::Xdto(format!(
                    "свойство «{}» входит не более {upper} раз, а заполнено {count}",
                    prop.name
                )));
            }
        }
        for place in places {
            let nested = match data.entries.borrow().get(place).map(|e| e.value.clone()) {
                Some(BslValue::Object(o)) => match &*o {
                    BslObject::XdtoObject(inst) => Some(inst.clone()),
                    _ => None,
                },
                _ => None,
            };
            if let Some(nested) = nested {
                validate_instance(&nested, depth)?;
            }
        }
    }
    Ok(())
}

/// `ОбъектXDTO.Последовательность()` — порядок заполнения свойств-элементов.
///
/// Есть она только у ПОСЛЕДОВАТЕЛЬНОГО типа: у `xs:choice` и `xs:all`
/// возвращается объект, а у типа-последовательности (`Упорядоченный` —
/// «Да») — `Неопределено`, а не ошибка (измерено на `RootType` и
/// `SimpContent`).
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не экземпляр либо
/// вызов с аргументами.
pub fn object_sequence(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let data = instance_of(obj, "Последовательность")?;
    if !args.is_empty() {
        return Err(not_applicable(obj, "Последовательность"));
    }
    if !data.type_data()?.sequenced() {
        return Ok(BslValue::Undefined);
    }
    Ok(BslValue::Object(Rc::new(BslObject::XdtoSequence(
        data.clone(),
    ))))
}

// --- `СписокXDTO` --------------------------------------------------------

/// Длина списка — число заполнений его свойства.
fn list_len(data: &Rc<XdtoObjectData>, prop: usize) -> usize {
    data.occurrences(prop).len()
}

fn out_of_bounds(i: usize, len: usize) -> RtError {
    RtError::IndexOutOfBounds {
        index: i as i64,
        len,
    }
}

/// `Список[i]`, `Список.Получить(i)` — само значение, а не `ЗначениеXDTO`
/// (измерено: `ТипЗнч(О.code[0])` — «Строка»).
///
/// # Errors
///
/// [`RtError::IndexOutOfBounds`], если номера в списке нет (измерено:
/// платформа отвечает ошибкой, а не `Неопределено`).
pub fn list_get(obj: &BslValue, i: usize) -> RtResult<BslValue> {
    let (data, prop) = list_of(obj, "Получить")?;
    list_item(data, prop, i)
}

fn list_item(data: &Rc<XdtoObjectData>, prop: usize, i: usize) -> RtResult<BslValue> {
    let places = data.occurrences(prop);
    let place = *places
        .get(i)
        .ok_or_else(|| out_of_bounds(i, places.len()))?;
    let entries = data.entries.borrow();
    entries
        .get(place)
        .map(|e| e.value.clone())
        .ok_or_else(|| broken("значение свойства"))
}

/// `Список.Добавить(Значение)` — значение приводится к типу свойства теми
/// же правилами, что и при записи (измерено: `many5.Добавить(5)` даёт
/// строку «5», `Добавить(Дата)` — «2026-08-13T00:00:00», `Добавить(Истина)`
/// — «true», а `Добавить(Неопределено)` — ошибка).
///
/// # Errors
///
/// [`RtError::Xdto`], если значение не приводится к типу свойства.
pub fn list_add(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let (data, prop) = list_of(obj, "Добавить")?;
    let [value] = args else {
        return Err(not_applicable(obj, "Добавить"));
    };
    let coerced = coerce_to_property(data, prop, value.clone())?;
    data.entries.borrow_mut().push(XdtoEntry {
        prop,
        value: coerced,
    });
    Ok(BslValue::Undefined)
}

/// `Список.Установить(i, Значение)`.
///
/// # Errors
///
/// [`RtError::IndexOutOfBounds`] за границей списка (измерено),
/// [`RtError::Xdto`], если значение не приводится к типу свойства.
pub fn list_set(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let (data, prop) = list_of(obj, "Установить")?;
    let [index, value] = args else {
        return Err(not_applicable(obj, "Установить"));
    };
    let places = data.occurrences(prop);
    let i = index_arg(index, places.len())?;
    let place = *places
        .get(i)
        .ok_or_else(|| out_of_bounds(i, places.len()))?;
    let coerced = coerce_to_property(data, prop, value.clone())?;
    let mut entries = data.entries.borrow_mut();
    let slot = entries
        .get_mut(place)
        .ok_or_else(|| broken("значение свойства"))?;
    slot.value = coerced;
    Ok(BslValue::Undefined)
}

/// `Список.Вставить(i, Значение)` — новое заполнение встаёт на место
/// i-го, то есть и в последовательности оно оказывается перед ним
/// (измерено: после `cb.Добавить("аб"); ca = "вг"; cb.Вставить(0, "де")`
/// последовательность — `[cb=де][cb=аб][ca=вг]`).
///
/// Позиция обязана быть ЗАНЯТОЙ: и в пустой список, и на место сразу за
/// последним элементом платформа вставлять отказывается (измерено оба),
/// так что дописать в конец можно только `Добавить`.
///
/// # Errors
///
/// [`RtError::IndexOutOfBounds`] за границей списка, [`RtError::Xdto`],
/// если значение не приводится к типу свойства.
pub fn list_insert(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let (data, prop) = list_of(obj, "Вставить")?;
    let [index, value] = args else {
        return Err(not_applicable(obj, "Вставить"));
    };
    let places = data.occurrences(prop);
    let i = index_arg(index, places.len())?;
    let at = *places
        .get(i)
        .ok_or_else(|| out_of_bounds(i, places.len()))?;
    let coerced = coerce_to_property(data, prop, value.clone())?;
    data.entries.borrow_mut().insert(
        at,
        XdtoEntry {
            prop,
            value: coerced,
        },
    );
    Ok(BslValue::Undefined)
}

/// `Список.Удалить(i)`.
///
/// # Errors
///
/// [`RtError::IndexOutOfBounds`] за границей списка (измерено).
pub fn list_delete(obj: &BslValue, index: &BslValue) -> RtResult<()> {
    let (data, prop) = list_of(obj, "Удалить")?;
    let places = data.occurrences(prop);
    let i = index_arg(index, places.len())?;
    let place = *places
        .get(i)
        .ok_or_else(|| out_of_bounds(i, places.len()))?;
    data.entries.borrow_mut().remove(place);
    Ok(())
}

/// `Список.Очистить()` — забыть все заполнения этого свойства.
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не список.
pub fn list_clear(obj: &BslValue) -> RtResult<()> {
    let (data, prop) = list_of(obj, "Очистить")?;
    data.entries.borrow_mut().retain(|e| e.prop != prop);
    Ok(())
}

/// Номер элемента из аргумента-числа.
fn index_arg(index: &BslValue, len: usize) -> RtResult<usize> {
    let BslValue::Number(n) = index else {
        return Err(RtError::BadIndex);
    };
    let i = n.to_i64_exact().ok_or(RtError::BadIndex)?;
    usize::try_from(i).map_err(|_| RtError::IndexOutOfBounds { index: i, len })
}

// --- `ПоследовательностьXDTO` --------------------------------------------

/// Хранилище получателя-последовательности.
fn sequence_of<'a>(obj: &'a BslValue, method: &'static str) -> RtResult<&'a Rc<XdtoObjectData>> {
    match obj {
        BslValue::Object(o) => match &**o {
            BslObject::XdtoSequence(data) => Ok(data),
            _ => Err(not_applicable(obj, method)),
        },
        _ => Err(not_applicable(obj, method)),
    }
}

/// Элемент ли это последовательности: в неё попадают только свойства формы
/// «Элемент» — запись АТРИБУТА её не удлиняет (измерено на `cat`).
fn in_sequence(model: &XdtoModel, prop: usize) -> RtResult<bool> {
    Ok(model.property_at(prop)?.form == EnumValue::XmlFormElement)
}

/// Номера записей хранилища, попадающих в последовательность.
fn sequence_places(data: &Rc<XdtoObjectData>) -> RtResult<Vec<usize>> {
    let mut places = Vec::new();
    for (i, e) in data.entries.borrow().iter().enumerate() {
        if in_sequence(&data.model, e.prop)? {
            places.push(i);
        }
    }
    Ok(places)
}

fn sequence_len(data: &Rc<XdtoObjectData>) -> RtResult<usize> {
    Ok(sequence_places(data)?.len())
}

/// `Последовательность.ПолучитьЗначение(i)`.
///
/// # Errors
///
/// [`RtError::IndexOutOfBounds`] за границей (измерено).
pub fn sequence_value(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let data = sequence_of(obj, "ПолучитьЗначение")?;
    let [index] = args else {
        return Err(not_applicable(obj, "ПолучитьЗначение"));
    };
    let places = sequence_places(data)?;
    let i = index_arg(index, places.len())?;
    let place = *places
        .get(i)
        .ok_or_else(|| out_of_bounds(i, places.len()))?;
    let entries = data.entries.borrow();
    entries
        .get(place)
        .map(|e| e.value.clone())
        .ok_or_else(|| broken("значение свойства"))
}

/// `Последовательность.ПолучитьСвойство(i)` — `СвойствоXDTO`, которым это
/// место заполнено (измерено: печатается именем свойства).
///
/// # Errors
///
/// [`RtError::IndexOutOfBounds`] за границей.
pub fn sequence_property(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let data = sequence_of(obj, "ПолучитьСвойство")?;
    let [index] = args else {
        return Err(not_applicable(obj, "ПолучитьСвойство"));
    };
    let places = sequence_places(data)?;
    let i = index_arg(index, places.len())?;
    let place = *places
        .get(i)
        .ok_or_else(|| out_of_bounds(i, places.len()))?;
    let prop = data
        .entries
        .borrow()
        .get(place)
        .map(|e| e.prop)
        .ok_or_else(|| broken("значение свойства"))?;
    Ok(property_value(&data.model, prop))
}

/// `Последовательность.Добавить(Свойство, Значение)` — заполнение в конец.
///
/// Свойство берётся только `СвойствоXDTO`, имя строкой платформа не
/// принимает, и атрибут тоже (измерено обе пробы). Заполнение видно и
/// через само свойство: после `Добавить(ca, "аб")` чтение `О.ca` даёт
/// «аб», а второе `Добавить` того же свойства удлиняет
/// последовательность до двух, оставляя в свойстве последнее значение
/// (измерено).
///
/// # Errors
///
/// [`RtError::Xdto`], если свойство чужое, атрибутное либо значение не
/// приводится к его типу.
pub fn sequence_add(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let data = sequence_of(obj, "Добавить")?;
    let [arg, value] = args else {
        return Err(not_applicable(obj, "Добавить"));
    };
    let BslValue::Object(_) = arg else {
        return Err(RtError::MethodNotApplicable {
            method: "Добавить",
            receiver: arg.type_name(),
        });
    };
    let prop = property_arg(data, arg, "Добавить")?;
    if !in_sequence(&data.model, prop)? {
        return Err(RtError::Xdto(format!(
            "свойство «{}» — не элемент, в последовательность оно не входит",
            data.model.property_at(prop)?.name
        )));
    }
    let coerced = coerce_to_property(data, prop, value.clone())?;
    data.entries.borrow_mut().push(XdtoEntry {
        prop,
        value: coerced,
    });
    Ok(BslValue::Undefined)
}

/// `Последовательность.Очистить()` — забыть заполнения свойств-ЭЛЕМЕНТОВ;
/// атрибуты уцелевают (измерено: после очистки `cat` на месте, `ca` пуст).
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не последовательность.
pub fn sequence_clear(obj: &BslValue) -> RtResult<()> {
    let data = sequence_of(obj, "Очистить")?;
    let places = sequence_places(data)?;
    let mut entries = data.entries.borrow_mut();
    for place in places.into_iter().rev() {
        entries.remove(place);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Модель типов из текста XSD — тем же путём, что и в бою: дерево
    /// строит `dom`, схему — `xsd`, а типы — этот модуль.
    fn model(text: &str) -> Rc<XdtoModel> {
        let schema = crate::xsd::schema_of_text(text).expect("схема обязана разбираться");
        model_of_schema(&schema).expect("модель обязана строиться")
    }

    /// Схема из `measure-xdto.bsl`, сокращённая до того, что проверяют
    /// тесты ниже. Имена и порядок объявлений — те же, поэтому измеренные
    /// строки платформы читаются рядом с ожиданиями.
    const SAMPLE: &str = concat!(
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:t="urn:test" "#,
        r#"targetNamespace="urn:test" elementFormDefault="qualified" "#,
        r#"attributeFormDefault="unqualified">"#,
        r#"<xs:simpleType name="Code"><xs:restriction base="xs:string">"#,
        r#"<xs:minLength value="2"/><xs:maxLength value="5"/>"#,
        r#"<xs:pattern value="[A-Z]+"/></xs:restriction></xs:simpleType>"#,
        r#"<xs:simpleType name="Codes"><xs:list itemType="t:Code"/></xs:simpleType>"#,
        r#"<xs:simpleType name="Either2"><xs:union memberTypes="xs:int xs:string"/></xs:simpleType>"#,
        r#"<xs:complexType name="RootType"><xs:sequence>"#,
        r#"<xs:element name="name" type="xs:string"/>"#,
        r#"<xs:element name="code" type="t:Code" minOccurs="0" maxOccurs="unbounded"/>"#,
        r#"<xs:element name="def" type="xs:int" default="7" minOccurs="0"/>"#,
        r#"<xs:element name="many5" type="xs:string" maxOccurs="5"/>"#,
        r#"<xs:element name="notype" minOccurs="0"/>"#,
        r#"<xs:element name="uq" type="xs:string" form="unqualified"/>"#,
        r#"<xs:element name="anon"><xs:complexType><xs:sequence>"#,
        r#"<xs:element name="inner" type="xs:decimal"/>"#,
        r#"</xs:sequence></xs:complexType></xs:element>"#,
        r#"</xs:sequence>"#,
        r#"<xs:attribute name="id" type="xs:int" use="required"/>"#,
        r#"<xs:attribute name="opt" type="xs:string"/>"#,
        r#"<xs:attribute name="q" type="xs:string" form="qualified"/>"#,
        r#"<xs:attribute name="fx" type="xs:int" fixed="9"/>"#,
        r#"</xs:complexType>"#,
        r#"<xs:complexType name="ExtType"><xs:complexContent>"#,
        r#"<xs:extension base="t:RootType">"#,
        r#"<xs:sequence><xs:element name="extra" type="xs:boolean"/></xs:sequence>"#,
        r#"<xs:attribute name="ea" type="xs:string"/>"#,
        r#"</xs:extension></xs:complexContent></xs:complexType>"#,
        r#"<xs:complexType name="ChoiceType">"#,
        r#"<xs:choice minOccurs="0"><xs:element name="ca" type="xs:string"/>"#,
        r#"<xs:element name="cb" type="xs:string" maxOccurs="3"/></xs:choice>"#,
        r#"<xs:attribute name="cat" type="xs:string"/></xs:complexType>"#,
        r#"<xs:complexType name="AllType"><xs:all>"#,
        r#"<xs:element name="ap" type="xs:string"/>"#,
        r#"<xs:element name="aq" type="xs:string" minOccurs="0"/></xs:all></xs:complexType>"#,
        r#"<xs:complexType name="NestType"><xs:sequence>"#,
        r#"<xs:element name="nx" type="xs:string"/>"#,
        r#"<xs:sequence maxOccurs="unbounded"><xs:element name="ny" type="xs:string"/>"#,
        r#"<xs:element name="nz" type="xs:string" minOccurs="0"/></xs:sequence>"#,
        r#"<xs:element name="nw" type="xs:string"/>"#,
        r#"</xs:sequence></xs:complexType>"#,
        r#"<xs:complexType name="SimpContent"><xs:simpleContent>"#,
        r#"<xs:extension base="xs:string"><xs:attribute name="su" type="xs:string"/>"#,
        r#"</xs:extension></xs:simpleContent></xs:complexType>"#,
        r#"<xs:complexType name="AbstrType" abstract="true" mixed="true"><xs:sequence>"#,
        r#"<xs:element name="x" type="xs:string"/></xs:sequence></xs:complexType>"#,
        r#"<xs:complexType name="EmptyType"/>"#,
        r#"</xs:schema>"#,
    );

    fn type_of(model: &Rc<XdtoModel>, uri: &str, name: &str) -> BslValue {
        let index = model
            .find(uri, name)
            .unwrap_or_else(|| panic!("в модели нет типа {{{uri}}}{name}"));
        type_value(model, index)
    }

    fn prop(obj: &BslValue, name: &str) -> BslValue {
        get_property(obj, name).unwrap_or_else(|e| panic!("член «{name}»: {e}"))
    }

    fn text_of(v: &BslValue) -> String {
        match v {
            BslValue::Str(s) => s.to_string(),
            other => panic!("ожидалась строка, получено {other:?}"),
        }
    }

    fn number_of(v: &BslValue) -> i64 {
        match v {
            BslValue::Number(n) => n.to_i64_exact().expect("целое"),
            other => panic!("ожидалось число, получено {other:?}"),
        }
    }

    /// Число целиком, включая дробную часть, — как каноническая строка.
    fn decimal_of(v: &BslValue) -> String {
        match v {
            BslValue::Number(n) => n.to_canonical(),
            other => panic!("ожидалось число, получено {other:?}"),
        }
    }

    /// Имена свойств типа объекта по порядку — то, что печатает проба
    /// `об Свойства порядок`.
    fn property_names(model: &Rc<XdtoModel>, uri: &str, name: &str) -> Vec<String> {
        let t = type_of(model, uri, name);
        let props = prop(&t, "Свойства");
        let len = match &props {
            BslValue::Object(o) => collection_len(o).expect("коллекция").expect("длина"),
            other => panic!("ожидалась коллекция, получено {other:?}"),
        };
        (0..len)
            .map(|i| match &props {
                BslValue::Object(o) => text_of(&prop(
                    &collection_get(o, i).expect("элемент коллекции"),
                    "Имя",
                )),
                other => panic!("ожидалась коллекция, получено {other:?}"),
            })
            .collect()
    }

    /// Порядок свойств: унаследованные, потом свои атрибуты, потом свои
    /// элементы. Обе строки — из `measure-xdto.platform.txt`.
    #[test]
    fn properties_are_flattened_attributes_first_then_elements() {
        let m = model(SAMPLE);
        assert_eq!(
            property_names(&m, "urn:test", "RootType"),
            vec!["id", "opt", "q", "fx", "name", "code", "def", "many5", "notype", "uq", "anon"]
        );
        // Наследник: сначала весь базовый тип, потом СВОЙ атрибут, потом
        // свой элемент.
        let ext = property_names(&m, "urn:test", "ExtType");
        assert_eq!(ext.len(), 13);
        assert_eq!(
            &ext[ext.len() - 2..],
            &["ea".to_string(), "extra".to_string()]
        );
        assert_eq!(&ext[..4], &["id", "opt", "q", "fx"].map(str::to_string));
        assert!(property_names(&m, "urn:test", "EmptyType").is_empty());
    }

    /// Границы вхождения перемножаются по вложенным группам, а
    /// `unbounded` показывается как -1.
    #[test]
    fn occurrence_bounds_multiply_through_model_groups() {
        let m = model(SAMPLE);
        let bounds = |type_name: &str, prop_name: &str| {
            let t = type_of(&m, "urn:test", type_name);
            let props = prop(&t, "Свойства");
            let p = collection_lookup(&props, &[str_value(prop_name)]).expect("поиск свойства");
            (
                number_of(&prop(&p, "НижняяГраница")),
                number_of(&prop(&p, "ВерхняяГраница")),
            )
        };
        assert_eq!(bounds("RootType", "name"), (1, 1));
        assert_eq!(bounds("RootType", "code"), (0, -1), "maxOccurs=unbounded");
        assert_eq!(bounds("RootType", "many5"), (1, 5));
        assert_eq!(bounds("RootType", "id"), (1, 1), "use=required");
        assert_eq!(bounds("RootType", "opt"), (0, 1), "атрибут без use");
        // `<xs:choice minOccurs="0">` обнуляет нижние границы вложенного.
        assert_eq!(bounds("ChoiceType", "ca"), (0, 1));
        assert_eq!(bounds("ChoiceType", "cb"), (0, 3));
        assert_eq!(bounds("ChoiceType", "cat"), (0, 1));
        // Вложенная последовательность с `maxOccurs="unbounded"`.
        assert_eq!(bounds("NestType", "nx"), (1, 1));
        assert_eq!(bounds("NestType", "ny"), (1, -1));
        assert_eq!(bounds("NestType", "nz"), (0, -1));
        assert_eq!(bounds("NestType", "nw"), (1, 1));
        assert_eq!(bounds("AllType", "ap"), (1, 1));
        assert_eq!(bounds("AllType", "aq"), (0, 1));
    }

    /// Форма и пространство имён свойства — по правилу форм схемы.
    #[test]
    fn property_form_and_namespace_follow_the_schema_forms() {
        let m = model(SAMPLE);
        let root = type_of(&m, "urn:test", "RootType");
        let props = prop(&root, "Свойства");
        let by_name = |n: &str| collection_lookup(&props, &[str_value(n)]).expect("поиск");
        assert_eq!(
            prop(&by_name("name"), "Форма"),
            BslValue::Enum(EnumValue::XmlFormElement)
        );
        assert_eq!(
            prop(&by_name("id"), "Форма"),
            BslValue::Enum(EnumValue::XmlFormAttribute)
        );
        assert_eq!(
            text_of(&prop(&by_name("name"), "URIПространстваИмен")),
            "urn:test"
        );
        assert_eq!(
            text_of(&prop(&by_name("uq"), "URIПространстваИмен")),
            "",
            "form=unqualified"
        );
        assert_eq!(text_of(&prop(&by_name("id"), "URIПространстваИмен")), "");
        assert_eq!(
            text_of(&prop(&by_name("q"), "URIПространстваИмен")),
            "urn:test",
            "form=qualified"
        );
        // Неизвестное имя — `Неопределено`, а не ошибка.
        assert_eq!(
            collection_lookup(&props, &[str_value("нетТакого")]).expect("поиск"),
            BslValue::Undefined
        );
    }

    /// Тип свойства: объявленный, анонимный и — при отсутствии обоих —
    /// `anyType`.
    #[test]
    fn property_type_falls_back_to_any_type() {
        let m = model(SAMPLE);
        let root = type_of(&m, "urn:test", "RootType");
        let props = prop(&root, "Свойства");
        let by_name = |n: &str| collection_lookup(&props, &[str_value(n)]).expect("поиск");
        let name_type = prop(&by_name("name"), "Тип");
        assert_eq!(text_of(&prop(&name_type, "Имя")), "string");
        assert_eq!(text_of(&prop(&name_type, "URIПространстваИмен")), XSD_NS);
        let no_type = prop(&by_name("notype"), "Тип");
        assert_eq!(text_of(&prop(&no_type, "Имя")), "anyType");
        // Анонимный тип: имя пусто, а пространство имён — целевое.
        let anon = prop(&by_name("anon"), "Тип");
        assert_eq!(text_of(&prop(&anon, "Имя")), "");
        assert_eq!(text_of(&prop(&anon, "URIПространстваИмен")), "urn:test");
        let anon_props = prop(&anon, "Свойства");
        assert_eq!(
            match &anon_props {
                BslValue::Object(o) => collection_len(o).expect("коллекция").expect("длина"),
                other => panic!("ожидалась коллекция, получено {other:?}"),
            },
            1
        );
    }

    /// Флаги типа объекта: упорядоченность по виду группы, а
    /// «последовательный» — производный от неё и от смешанности.
    #[test]
    fn object_type_flags_follow_the_content_model() {
        let m = model(SAMPLE);
        let flags = |name: &str| {
            let t = type_of(&m, "urn:test", name);
            (
                prop(&t, "Упорядоченный"),
                prop(&t, "Последовательный"),
                prop(&t, "Смешанный"),
                prop(&t, "Открытый"),
            )
        };
        let yes = BslValue::Boolean(true);
        let no = BslValue::Boolean(false);
        assert_eq!(
            flags("RootType"),
            (yes.clone(), no.clone(), no.clone(), no.clone())
        );
        assert_eq!(
            flags("ChoiceType"),
            (no.clone(), yes.clone(), no.clone(), no.clone())
        );
        assert_eq!(
            flags("AllType"),
            (no.clone(), yes.clone(), no.clone(), no.clone())
        );
        assert_eq!(
            flags("AbstrType"),
            (yes.clone(), yes.clone(), yes.clone(), no.clone())
        );
        assert_eq!(
            flags("EmptyType"),
            (yes.clone(), no.clone(), no.clone(), no.clone())
        );
        assert_eq!(
            prop(&type_of(&m, "urn:test", "AbstrType"), "Абстрактный"),
            yes
        );
        assert_eq!(
            prop(&type_of(&m, "urn:test", "RootType"), "Абстрактный"),
            no
        );
        // Открыт ровно `anyType`.
        assert_eq!(prop(&type_of(&m, XSD_NS, "anyType"), "Открытый"), yes);
    }

    /// Базовый тип: у объекта без базового — `anyType`, у значения —
    /// `anySimpleType`, а у простого содержимого базовым остаётся
    /// `anyType`, простой же тип виден свойством `__content`.
    #[test]
    fn base_type_defaults_to_any_type_and_any_simple_type() {
        let m = model(SAMPLE);
        let base_of = |uri: &str, name: &str| {
            let t = type_of(&m, uri, name);
            text_of(&prop(&prop(&t, "БазовыйТип"), "Имя"))
        };
        assert_eq!(base_of("urn:test", "RootType"), "anyType");
        assert_eq!(base_of("urn:test", "ExtType"), "RootType");
        assert_eq!(base_of("urn:test", "Code"), "string");
        assert_eq!(base_of("urn:test", "Codes"), "anySimpleType");
        assert_eq!(base_of("urn:test", "SimpContent"), "anyType");
        assert_eq!(base_of(XSD_NS, "int"), "long");
        assert_eq!(base_of(XSD_NS, "string"), "anySimpleType");
        assert_eq!(
            prop(&type_of(&m, XSD_NS, "anyType"), "БазовыйТип"),
            BslValue::Undefined
        );
        // Простое содержимое: атрибут, затем текстовое свойство.
        assert_eq!(
            property_names(&m, "urn:test", "SimpContent"),
            vec!["su", "__content"]
        );
        let simp = type_of(&m, "urn:test", "SimpContent");
        let content =
            collection_lookup(&prop(&simp, "Свойства"), &[str_value("__content")]).expect("поиск");
        assert_eq!(
            prop(&content, "Форма"),
            BslValue::Enum(EnumValue::XmlFormText)
        );
        assert_eq!(text_of(&prop(&prop(&content, "Тип"), "Имя")), "string");
        assert_eq!(text_of(&prop(&content, "URIПространстваИмен")), "");
        // У СМЕШАННОГО типа текстового свойства нет.
        assert_eq!(property_names(&m, "urn:test", "AbstrType"), vec!["x"]);
    }

    /// Фасеты: вид и значение-строка, а у типа без фасетов —
    /// `Неопределено`.
    #[test]
    fn facets_report_their_kind_and_lexical_value() {
        let m = model(SAMPLE);
        let facets_of = |uri: &str, name: &str| {
            let t = type_of(&m, uri, name);
            let facets = prop(&t, "Фасеты");
            match &facets {
                BslValue::Undefined => Vec::new(),
                BslValue::Object(o) => {
                    let len = collection_len(o).expect("коллекция").expect("длина");
                    (0..len)
                        .map(|i| {
                            let f = collection_get(o, i).expect("фасет");
                            (prop(&f, "Вид"), text_of(&prop(&f, "Значение")))
                        })
                        .collect::<Vec<_>>()
                }
                other => panic!("ожидалась коллекция или Неопределено, получено {other:?}"),
            }
        };
        assert_eq!(
            facets_of("urn:test", "Code"),
            vec![
                (
                    BslValue::Enum(EnumValue::XdtoFacetMinLength),
                    "2".to_string()
                ),
                (
                    BslValue::Enum(EnumValue::XdtoFacetMaxLength),
                    "5".to_string()
                ),
                (
                    BslValue::Enum(EnumValue::XdtoFacetPattern),
                    "[A-Z]+".to_string()
                ),
            ]
        );
        // Встроенные типы несут измеренные фасеты.
        assert_eq!(
            facets_of(XSD_NS, "string"),
            vec![(
                BslValue::Enum(EnumValue::XdtoFacetWhiteSpace),
                "preserve".to_string()
            )]
        );
        assert_eq!(
            facets_of(XSD_NS, "int"),
            vec![
                (
                    BslValue::Enum(EnumValue::XdtoFacetMinInclusive),
                    "-2147483648".to_string()
                ),
                (
                    BslValue::Enum(EnumValue::XdtoFacetMaxInclusive),
                    "2147483647".to_string()
                ),
            ]
        );
        // `xs:date` фасетов не несёт, и `Фасеты` у него `Неопределено`.
        assert_eq!(
            prop(&type_of(&m, XSD_NS, "date"), "Фасеты"),
            BslValue::Undefined
        );
    }

    /// Значение по умолчанию — `ЗначениеXDTO` из `default` и из `fixed`.
    #[test]
    fn default_value_comes_from_both_default_and_fixed() {
        let m = model(SAMPLE);
        let root = type_of(&m, "urn:test", "RootType");
        let props = prop(&root, "Свойства");
        let by_name = |n: &str| collection_lookup(&props, &[str_value(n)]).expect("поиск");
        let def = prop(&by_name("def"), "ЗначениеПоУмолчанию");
        assert_eq!(number_of(&prop(&def, "Значение")), 7);
        assert_eq!(text_of(&prop(&def, "ЛексическоеЗначение")), "7");
        let fx = prop(&by_name("fx"), "ЗначениеПоУмолчанию");
        assert_eq!(number_of(&prop(&fx, "Значение")), 9);
        assert_eq!(text_of(&prop(&fx, "ЛексическоеЗначение")), "9");
        assert_eq!(
            prop(&by_name("name"), "ЗначениеПоУмолчанию"),
            BslValue::Undefined
        );
    }

    /// Таблица встроенных типов: каждая строка — из
    /// `measure-xdto.platform.txt` (`вст <имя>`).
    #[test]
    fn builtin_types_map_to_the_measured_bsl_types() {
        let m = model(SAMPLE);
        let value = |name: &str, lexical: &str| {
            let index = m.find(XSD_NS, name).expect("встроенный тип");
            value_from_lexical(&m, index, lexical).expect("лексическая форма")
        };
        let type_of_value = |name: &str, lexical: &str| value(name, lexical).type_of().unwrap();
        for name in [
            "string",
            "normalizedString",
            "token",
            "duration",
            "gYear",
            "anyURI",
            "NCName",
            "Name",
            "NMTOKEN",
            "language",
            "ID",
            "anySimpleType",
        ] {
            assert_eq!(
                type_of_value(name, "аб"),
                BslValue::Type(TypeId::String),
                "{name}"
            );
        }
        for name in [
            "decimal",
            "int",
            "integer",
            "long",
            "short",
            "byte",
            "unsignedInt",
            "unsignedLong",
            "unsignedShort",
            "unsignedByte",
            "nonNegativeInteger",
            "positiveInteger",
            "negativeInteger",
            "nonPositiveInteger",
            "double",
            "float",
        ] {
            assert_eq!(
                type_of_value(name, "-42"),
                BslValue::Type(TypeId::Number),
                "{name}"
            );
        }
        assert_eq!(
            type_of_value("boolean", "true"),
            BslValue::Type(TypeId::Boolean)
        );
        assert_eq!(
            type_of_value("date", "2026-08-12"),
            BslValue::Type(TypeId::Date)
        );
        assert_eq!(
            type_of_value("dateTime", "2026-08-12T18:41:17"),
            BslValue::Type(TypeId::Date)
        );
        assert_eq!(
            type_of_value("time", "18:41:17"),
            BslValue::Type(TypeId::Date)
        );
        assert_eq!(
            type_of_value("base64Binary", "0LDQsQ=="),
            BslValue::Type(TypeId::BinaryData)
        );
        assert_eq!(
            type_of_value("hexBinary", "D0B0D0B1"),
            BslValue::Type(TypeId::BinaryData)
        );
        assert_eq!(
            type_of_value("QName", "просто"),
            BslValue::Type(TypeId::XmlExpandedName)
        );
        // `anyType` — тип ОБЪЕКТА: значения из лексической формы он не
        // строит (измерено: `Создать` от него отвергает лексику).
        let any = m.find(XSD_NS, "anyType").expect("anyType");
        assert!(value_from_lexical(&m, any, "текст").is_err());
    }

    /// Разбор лексических форм — те же записи, что снимались на
    /// платформе.
    #[test]
    fn lexical_forms_follow_the_measured_conversions() {
        let m = model(SAMPLE);
        let value = |name: &str, lexical: &str| {
            let index = m.find(XSD_NS, name).expect("встроенный тип");
            value_from_lexical(&m, index, lexical).expect("лексическая форма")
        };
        assert_eq!(text_of(&value("string", "аб в")), "аб в");
        assert_eq!(decimal_of(&value("decimal", "-12.75")), "-12.75");
        assert_eq!(
            value("decimal", "+12.750"),
            value("decimal", "12.75"),
            "ведущий плюс и хвостовые нули"
        );
        assert_eq!(number_of(&value("double", "1.5E3")), 1500);
        assert_eq!(number_of(&value("int", " 42 ")), 42, "пробелы по краям");
        assert_eq!(value("boolean", "true"), BslValue::Boolean(true));
        assert_eq!(value("boolean", "1"), BslValue::Boolean(true));
        assert_eq!(value("boolean", "false"), BslValue::Boolean(false));
        assert_eq!(value("boolean", "0"), BslValue::Boolean(false));
        // Двоичные записи дают одни и те же байты — «аб» в UTF-8.
        let expected: &[u8] = &[0xD0, 0xB0, 0xD0, 0xB1];
        for (name, lexical) in [("base64Binary", "0LDQsQ=="), ("hexBinary", "D0B0D0B1")] {
            match value(name, lexical) {
                BslValue::Object(o) => match &*o {
                    BslObject::BinaryData(bytes) => assert_eq!(&**bytes, expected, "{name}"),
                    other => panic!("ожидались двоичные данные, получено {other:?}"),
                },
                other => panic!("ожидались двоичные данные, получено {other:?}"),
            }
        }
        // QName без префикса — расширенное имя с пустым URI, с префиксом
        // — ошибка (измерено обе стороны).
        let qname = m.find(XSD_NS, "QName").expect("QName");
        let name = value_from_lexical(&m, qname, "просто").expect("QName");
        // Расширенное имя — значение модели СХЕМЫ, и члены у него читает
        // она же.
        let expanded = |member: &str| {
            text_of(&crate::xsd::get_property(&name, member).expect("член расширенного имени"))
        };
        assert_eq!(expanded("ЛокальноеИмя"), "просто");
        assert_eq!(expanded("URIПространстваИмен"), "");
        assert!(value_from_lexical(&m, qname, "xs:string").is_err());
        // Непринимаемые записи — ошибка, а не подстановка.
        let int = m.find(XSD_NS, "int").expect("int");
        assert!(value_from_lexical(&m, int, "ерунда").is_err());
        let date = m.find(XSD_NS, "date").expect("date");
        assert!(value_from_lexical(&m, date, "ерунда").is_err());
        // Форма ДЛИННЕЕ `ГГГГ-ММ-ДД`, у которой многобайтовый символ
        // накрывает смещение 10: разбор ищет пояс именно с него, и срез по
        // сырому байтовому индексу здесь ронял процесс. Ожидается ошибка.
        assert!(value_from_lexical(&m, date, "2026-08-1я").is_err());
        assert!(value_from_lexical(&m, date, "2026-08-1я+03:00").is_err());
        let boolean = m.find(XSD_NS, "boolean").expect("boolean");
        assert!(value_from_lexical(&m, boolean, "да").is_err());
    }

    /// Свой тип наследует отображение базового, список даёт
    /// фиксированный массив, а объединение выбирает первый подошедший
    /// член.
    #[test]
    fn derived_list_and_union_types_map_as_measured() {
        let m = model(SAMPLE);
        let value = |uri: &str, name: &str, lexical: &str| {
            let index = m.find(uri, name).expect("тип");
            value_from_lexical(&m, index, lexical).expect("лексическая форма")
        };
        assert_eq!(text_of(&value("urn:test", "Code", "AB")), "AB");
        // `union memberTypes="xs:int xs:string"`: «5» — число, «аб» —
        // строка.
        assert_eq!(number_of(&value("urn:test", "Either2", "5")), 5);
        assert_eq!(text_of(&value("urn:test", "Either2", "аб")), "аб");
        // Платформа отдаёт здесь `ФиксированныйМассив`, здесь это обычный
        // массив: неизменяемого вида в этой реализации нет.
        match value("urn:test", "Codes", "AB CD") {
            BslValue::Object(o) => match &*o {
                BslObject::Array(items) => {
                    let items = items.borrow();
                    assert_eq!(items.len(), 2);
                    assert_eq!(text_of(&items[0]), "AB");
                    assert_eq!(text_of(&items[1]), "CD");
                }
                other => panic!("ожидался массив, получено {other:?}"),
            },
            other => panic!("ожидался массив, получено {other:?}"),
        }
    }

    /// Имена и представления значений модели — то, что печатают
    /// `Строка()` и `ТипЗнч()`.
    #[test]
    fn type_and_property_values_print_as_measured() {
        let m = model(SAMPLE);
        let root = type_of(&m, "urn:test", "RootType");
        assert_eq!(root.to_string(), "{urn:test}RootType");
        assert_eq!(
            root.type_of().unwrap(),
            BslValue::Type(TypeId::XdtoObjectType)
        );
        assert_eq!(TypeId::XdtoObjectType.name(), "Тип объекта XDTO");
        let code = type_of(&m, "urn:test", "Code");
        assert_eq!(code.to_string(), "{urn:test}Code");
        assert_eq!(
            code.type_of().unwrap(),
            BslValue::Type(TypeId::XdtoValueType)
        );
        let props = prop(&root, "Свойства");
        assert_eq!(
            props.type_of().unwrap(),
            BslValue::Type(TypeId::XdtoPropertyCollection)
        );
        assert_eq!(props.to_string(), "КоллекцияСвойствXDTO");
        let name = collection_lookup(&props, &[str_value("name")]).expect("поиск");
        assert_eq!(name.to_string(), "name", "свойство печатается именем");
        assert_eq!(
            name.type_of().unwrap(),
            BslValue::Type(TypeId::XdtoProperty)
        );
        // Анонимный тип печатается ПУСТОЙ строкой, хотя URI у него есть.
        let anon = prop(
            &collection_lookup(&props, &[str_value("anon")]).expect("поиск"),
            "Тип",
        );
        assert_eq!(anon.to_string(), "");
        assert_eq!(text_of(&prop(&anon, "URIПространстваИмен")), "urn:test");
        let facets = prop(&code, "Фасеты");
        assert_eq!(
            facets.type_of().unwrap(),
            BslValue::Type(TypeId::XdtoFacetCollection)
        );
        assert_eq!(facets.to_string(), "КоллекцияФасетовXDTO");
        match &facets {
            BslValue::Object(o) => {
                let f = collection_get(o, 0).expect("фасет");
                assert_eq!(f.type_of().unwrap(), BslValue::Type(TypeId::XdtoFacet));
                assert_eq!(f.to_string(), "ФасетXDTO");
            }
            other => panic!("ожидалась коллекция, получено {other:?}"),
        }
        let def = prop(
            &collection_lookup(&props, &[str_value("def")]).expect("поиск"),
            "ЗначениеПоУмолчанию",
        );
        assert_eq!(
            def.type_of().unwrap(),
            BslValue::Type(TypeId::XdtoDataValue)
        );
        assert_eq!(def.to_string(), "ЗначениеXDTO");
    }

    /// Разбор перечня фасетов из снятой строки: `Вид=[значение]` через
    /// пробел. Значение само может содержать скобки (`Образец` у
    /// `xs:integer` — это `[\-+]?[0-9]+`), поэтому конец значения ищется
    /// как последняя `]` перед следующим `=[`, а не как первая же.
    fn measured_facets(text: &str) -> Vec<(String, String)> {
        let mut out = Vec::new();
        let mut rest = text.trim();
        while let Some(eq) = rest.find("=[") {
            let key = rest[..eq].trim().to_string();
            let after = &rest[eq + 2..];
            let end = match after.find("=[") {
                Some(next) => after[..next].rfind(']'),
                None => after.rfind(']'),
            }
            .expect("у значения фасета есть закрывающая скобка");
            out.push((key, after[..end].to_string()));
            rest = &after[end + 1..];
        }
        out
    }

    /// Таблица встроенных типов сверяется СО СНЯТЫМ ФАЙЛОМ, а не с
    /// ожиданиями этого теста: строки `фв <имя>` дают базовый тип и
    /// фасеты, строки `вст <имя>` — тип BSL, в который платформа
    /// отобразила значение. Пока файл лежит рядом, ни одна строка
    /// [`BUILTIN_TYPES`] не может разъехаться с платформой незаметно.
    #[test]
    fn every_builtin_row_is_backed_by_a_measured_line() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/conformance/measure/measure-xdto.platform.txt");
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("не читается {}: {e}", path.display()));
        let m = model(SAMPLE);

        // `фв <имя>` -> базовый тип и фасеты.
        let mut checked_bases = 0;
        let mut checked_values = 0;
        for line in text.lines() {
            let Some((label, value)) = line.split_once('\t') else {
                continue;
            };
            if let Some(name) = label.strip_prefix("фв ") {
                let index = m
                    .find(XSD_NS, name)
                    .unwrap_or_else(|| panic!("в таблице нет встроенного типа {name}"));
                let t = type_value(&m, index);
                // `база=[{URI}имя] Вид=[значение] Вид=[значение] …`
                let (base_part, facet_part) = value.split_once(']').expect("база в строке");
                let base = base_part
                    .strip_prefix("база=[")
                    .expect("строка начинается с базы");
                assert_eq!(
                    prop(&t, "БазовыйТип").to_string(),
                    base,
                    "базовый тип {name}"
                );
                let measured = measured_facets(facet_part);
                let ours: Vec<(String, String)> = m.types[index]
                    .facets
                    .iter()
                    .map(|(kind, lexical)| {
                        (
                            facet_kind_value(*kind).display_text().to_string(),
                            lexical.clone(),
                        )
                    })
                    .collect();
                if facet_part.contains("фасеты=<нет>") {
                    assert!(ours.is_empty(), "у {name} фасетов быть не должно");
                } else {
                    assert_eq!(ours, measured, "фасеты {name}");
                }
                checked_bases += 1;
                continue;
            }
            // `вст <имя>` -> `… знач=<Тип> [значение] …`; строки с
            // уточнением в метке («вст boolean цифрой») пропускаются:
            // тип там тот же, а сверяется таблица, а не разбор.
            let Some(name) = label.strip_prefix("вст ") else {
                continue;
            };
            let Some(index) = m.find(XSD_NS, name) else {
                continue;
            };
            match value.split_once("знач=") {
                Some((_, rest)) if rest.starts_with("<не создаётся>") => {
                    assert!(
                        m.builtin_of(index).is_none(),
                        "{name} не должен строить значение"
                    );
                }
                Some((_, rest)) => {
                    let measured = rest.split_once(" [").expect("значение в скобках").0;
                    let ours = m
                        .builtin_of(index)
                        .unwrap_or_else(|| panic!("{name} обязан отображаться в тип BSL"));
                    assert_eq!(ours.type_id().name(), measured, "тип BSL для {name}");
                }
                None => continue,
            }
            checked_values += 1;
        }
        // Обе выборки непусты и покрывают таблицу: иначе тест зелен
        // просто оттого, что ничего не нашёл.
        assert_eq!(
            checked_bases,
            BUILTIN_TYPES.len() - 1,
            "строк «фв» должно быть по одной на каждый тип, кроме anyType"
        );
        assert!(checked_values >= 30, "строк «вст» найдено {checked_values}");
    }

    /// Тождество типов: два обращения к одному имени дают РАВНЫЕ значения
    /// (измерено), а разные типы не равны.
    #[test]
    fn types_are_references_into_one_model() {
        let m = model(SAMPLE);
        assert_eq!(
            type_of(&m, "urn:test", "RootType"),
            type_of(&m, "urn:test", "RootType")
        );
        assert_ne!(
            type_of(&m, "urn:test", "RootType"),
            type_of(&m, "urn:test", "ExtType")
        );
        assert!(m.find("urn:нет", "RootType").is_none(), "чужой URI");
        assert!(m.find("urn:test", "нетТакого").is_none());
    }

    /// Ошибочные пути отвечают `RtError`, а не паникой.
    #[test]
    fn broken_schemas_report_errors_instead_of_panicking() {
        // Ссылка на несуществующий тип.
        let schema = crate::xsd::schema_of_text(concat!(
            r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:t">"#,
            r#"<xs:complexType name="T"><xs:sequence>"#,
            r#"<xs:element name="a" type="xs:нетТакого"/>"#,
            r#"</xs:sequence></xs:complexType></xs:schema>"#,
        ))
        .expect("схема разбирается");
        let error = model_of_schema(&schema).expect_err("тип не разрешается");
        assert!(
            error.to_string().contains("нетТакого"),
            "в тексте ошибки нет имени типа: {error}"
        );

        // Кольцо в цепочке базовых типов простого типа: разбор
        // лексической формы обязан отвечать ошибкой, а не переполнением
        // стека.
        let schema = crate::xsd::schema_of_text(concat!(
            r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:t="urn:t" "#,
            r#"targetNamespace="urn:t">"#,
            r#"<xs:simpleType name="A"><xs:restriction base="t:A"/></xs:simpleType>"#,
            r#"</xs:schema>"#,
        ))
        .expect("схема разбирается");
        let cyclic =
            model_of_schema(&schema).expect("модель строится: цикл здесь только у значений");
        let a = cyclic.find("urn:t", "A").expect("тип A");
        assert!(value_from_lexical(&cyclic, a, "что-нибудь").is_err());
        assert!(cyclic.builtin_of(a).is_none(), "кольцо не даёт отображения");

        // Цикл наследования типов ОБЪЕКТА ловится при построении модели.
        let schema = crate::xsd::schema_of_text(concat!(
            r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:t="urn:t" "#,
            r#"targetNamespace="urn:t">"#,
            r#"<xs:complexType name="A"><xs:complexContent>"#,
            r#"<xs:extension base="t:B"/></xs:complexContent></xs:complexType>"#,
            r#"<xs:complexType name="B"><xs:complexContent>"#,
            r#"<xs:extension base="t:A"/></xs:complexContent></xs:complexType>"#,
            r#"</xs:schema>"#,
        ))
        .expect("схема разбирается");
        let error = model_of_schema(&schema).expect_err("цикл наследования");
        assert!(
            error.to_string().contains("цикл"),
            "в тексте ошибки нет слова про цикл: {error}"
        );

        // Значение по умолчанию, не разбирающееся в своём типе.
        let schema = crate::xsd::schema_of_text(concat!(
            r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:t">"#,
            r#"<xs:complexType name="T"><xs:sequence>"#,
            r#"<xs:element name="a" type="xs:int" default="ерунда"/>"#,
            r#"</xs:sequence></xs:complexType></xs:schema>"#,
        ))
        .expect("схема разбирается");
        assert!(model_of_schema(&schema).is_err(), "мусор в default");

        // Тот же путь, но лексическая форма испорчена так, что разбор
        // `xs:date` берёт срез по НЕ границе символа: «2026-08-1я» длиннее
        // десяти байт, и десятый байт лежит внутри «я». Схема доходит сюда
        // сама (`collect_elements` -> `has_constraint` -> `value_of`), так
        // что ответом обязана быть ошибка, а не паника процесса.
        let schema = crate::xsd::schema_of_text(concat!(
            r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:t">"#,
            r#"<xs:complexType name="T"><xs:sequence>"#,
            r#"<xs:element name="a" type="xs:date" default="2026-08-1я"/>"#,
            r#"</xs:sequence></xs:complexType></xs:schema>"#,
        ))
        .expect("схема разбирается");
        assert!(
            model_of_schema(&schema).is_err(),
            "испорченная дата в default"
        );

        // Неизвестный член — `RtError`, а не паника.
        let m = model(SAMPLE);
        let root = type_of(&m, "urn:test", "RootType");
        assert!(get_property(&root, "НетТакогоЧлена").is_err());
        // Члены типа ЗНАЧЕНИЯ на типе объекта не отвечают, и наоборот.
        assert!(get_property(&root, "Фасеты").is_err());
        assert!(get_property(&type_of(&m, "urn:test", "Code"), "Свойства").is_err());
    }

    // --- фабрика -----------------------------------------------------------

    /// Фабрика над моделью одной схемы — то, что получается из
    /// `СоздатьФабрикуXDTO`, только без файла на диске.
    fn factory(text: &str) -> BslValue {
        factory_value(model(text))
    }

    fn factory_of_texts(texts: &[&str]) -> BslValue {
        let schemas: Vec<Rc<XsSchemaData>> = texts
            .iter()
            .map(|t| crate::xsd::schema_of_text(t).expect("схема обязана разбираться"))
            .collect();
        factory_value(model_of_schemas(&schemas).expect("модель обязана строиться"))
    }

    /// Набор схем даёт ОДНУ модель: типы всех схем видны через одну
    /// фабрику, а ссылка по имени разрешается через границу схемы.
    #[test]
    fn a_factory_over_a_schema_set_resolves_names_across_schemas() {
        let a = concat!(
            r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:a">"#,
            r#"<xs:simpleType name="Code"><xs:restriction base="xs:string">"#,
            r#"<xs:minLength value="2"/></xs:restriction></xs:simpleType></xs:schema>"#,
        );
        let b = concat!(
            r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:a="urn:a" "#,
            r#"targetNamespace="urn:b"><xs:complexType name="Row"><xs:sequence>"#,
            r#"<xs:element name="code" type="a:Code"/>"#,
            r#"</xs:sequence></xs:complexType></xs:schema>"#,
        );
        let f = factory_of_texts(&[a, b]);
        let row = factory_type(&f, &[str_value("urn:b"), str_value("Row")]).expect("тип");
        assert_eq!(row.to_string(), "{urn:b}Row");
        let code = factory_type(&f, &[str_value("urn:a"), str_value("Code")]).expect("тип");
        assert_eq!(code.to_string(), "{urn:a}Code");
        // Свойство схемы B ссылается на тип схемы A — и ссылка связана.
        let by_name =
            collection_lookup(&prop(&row, "Свойства"), &[str_value("code")]).expect("поиск");
        assert_eq!(prop(&by_name, "Тип"), code);
        // Порядок схем в наборе на разрешение не влияет.
        let reversed = factory_of_texts(&[b, a]);
        assert_eq!(
            factory_type(&reversed, &[str_value("urn:a"), str_value("Code")])
                .expect("тип")
                .to_string(),
            "{urn:a}Code"
        );
        // Встроенные типы объявлены ОДИН раз на всю модель, а не по разу
        // на схему: иначе `find` возвращал бы первый из двух одинаковых.
        let string = factory_type(&f, &[str_value(XSD_NS), str_value("string")]).expect("тип");
        assert_eq!(string.to_string(), format!("{{{XSD_NS}}}string"));
        // Пустой набор — это фабрика с одними встроенными типами.
        let empty = factory_of_texts(&[]);
        assert_eq!(
            factory_type(&empty, &[str_value(XSD_NS), str_value("string")]).expect("тип"),
            factory_type(&empty, &[str_value(XSD_NS), str_value("string")]).expect("тип")
        );
        assert_eq!(
            factory_type(&empty, &[str_value("urn:a"), str_value("Code")]).expect("тип"),
            BslValue::Undefined
        );
    }

    /// `Тип` берёт пару (URI, имя) или расширенное имя, неизвестное имя
    /// даёт `Неопределено`, а два обращения за одним именем — равные
    /// значения (всё измерено).
    #[test]
    fn factory_type_takes_a_pair_or_an_expanded_name() {
        let f = factory(SAMPLE);
        let pair = factory_type(&f, &[str_value("urn:test"), str_value("RootType")]).expect("тип");
        assert_eq!(pair.to_string(), "{urn:test}RootType");
        let expanded = crate::xsd::new_expanded_name("urn:test", "RootType");
        assert_eq!(factory_type(&f, &[expanded]).expect("тип"), pair);
        // Два обращения за одним именем равны — тип это ссылка в модель.
        assert_eq!(
            factory_type(&f, &[str_value("urn:test"), str_value("RootType")]).expect("тип"),
            pair
        );
        // Неизвестное имя и чужой URI — `Неопределено`, а не ошибка.
        for args in [
            [str_value("urn:test"), str_value("НетТакого")],
            [str_value("urn:нет"), str_value("RootType")],
            // Пустой URI (измерено на `Тип("", "RootType")`).
            [str_value(""), str_value("RootType")],
        ] {
            assert_eq!(factory_type(&f, &args).expect("поиск"), BslValue::Undefined);
        }
        // Одна строка, три аргумента, числа вместо имён и вызов без
        // аргументов — ошибка (измерено все четыре).
        assert!(factory_type(&f, &[str_value("RootType")]).is_err());
        assert!(factory_type(
            &f,
            &[
                str_value("urn:test"),
                str_value("RootType"),
                number_value(1)
            ]
        )
        .is_err());
        assert!(factory_type(&f, &[number_value(5), number_value(5)]).is_err());
        assert!(factory_type(&f, &[]).is_err());
        // Получатель обязан быть фабрикой.
        assert!(factory_type(&pair, &[str_value("urn:test"), str_value("RootType")]).is_err());
    }

    /// `Создать` от типа ЗНАЧЕНИЯ: без лексики — `Неопределено`, с
    /// лексикой — `ЗначениеXDTO` с обоими членами.
    #[test]
    fn factory_create_builds_a_value_from_its_lexical_form() {
        let f = factory(SAMPLE);
        let code = factory_type(&f, &[str_value("urn:test"), str_value("Code")]).expect("тип");
        let value = factory_create(&f, &[code.clone(), str_value("AB")]).expect("значение");
        assert_eq!(
            value.type_of().unwrap(),
            BslValue::Type(TypeId::XdtoDataValue)
        );
        assert_eq!(text_of(&prop(&value, "Значение")), "AB");
        assert_eq!(text_of(&prop(&value, "ЛексическоеЗначение")), "AB");
        // Лексическая форма разбирается по ТИПУ: свой тип наследует
        // отображение базового, а встроенный числовой даёт число.
        let int = factory_type(&f, &[str_value(XSD_NS), str_value("int")]).expect("тип");
        let number = factory_create(&f, &[int.clone(), str_value("-42")]).expect("значение");
        assert_eq!(number_of(&prop(&number, "Значение")), -42);
        // Без лексической формы — `Неопределено` (измерено).
        assert_eq!(
            factory_create(&f, std::slice::from_ref(&int)).expect("вызов"),
            BslValue::Undefined
        );
        // Третий аргумент платформа принимает, четвёртый — уже нет.
        assert!(factory_create(&f, &[int.clone(), str_value("1"), number_value(1)]).is_ok());
        assert!(factory_create(
            &f,
            &[
                int.clone(),
                str_value("1"),
                number_value(1),
                number_value(1)
            ]
        )
        .is_err());
        // Не разбирающаяся форма — ошибка, а не подстановка.
        assert!(factory_create(&f, &[int, str_value("ерунда")]).is_err());
        // Первый аргумент — обязательно тип XDTO, а лексическая форма —
        // обязательно строка (нестроковую см. в шапке модуля).
        assert!(factory_create(&f, &[str_value("string"), str_value("аб")]).is_err());
        assert!(factory_create(&f, &[code, number_value(42)]).is_err());
        assert!(factory_create(&f, &[]).is_err());
    }

    /// `Создать` от типа ОБЪЕКТА даёт экземпляр: он печатается своим
    /// именем и отдаёт свой тип методом `Тип()`.
    #[test]
    fn factory_create_builds_an_object_that_knows_its_type() {
        let f = factory(SAMPLE);
        let root = factory_type(&f, &[str_value("urn:test"), str_value("RootType")]).expect("тип");
        let object = factory_create(&f, std::slice::from_ref(&root)).expect("экземпляр");
        assert_eq!(object.to_string(), "ОбъектXDTO");
        assert_eq!(
            object.type_of().unwrap(),
            BslValue::Type(TypeId::XdtoDataObject)
        );
        assert_eq!(TypeId::XdtoDataObject.name(), "Объект XDTO");
        assert_eq!(object_type(&object, &[]).expect("тип"), root);
        // Аргументов у `Тип()` нет, а два экземпляра одного типа не равны
        // (измерено обе стороны).
        assert!(object_type(&object, &[number_value(1)]).is_err());
        assert_ne!(object, factory_create(&f, &[root]).expect("экземпляр"));
        // Лексической формы тип объекта не берёт, абстрактный тип
        // экземпляров не имеет (измерено).
        let abstr =
            factory_type(&f, &[str_value("urn:test"), str_value("AbstrType")]).expect("тип");
        assert!(factory_create(&f, &[abstr]).is_err());
        let empty =
            factory_type(&f, &[str_value("urn:test"), str_value("EmptyType")]).expect("тип");
        assert!(factory_create(&f, &[empty.clone(), str_value("аб")]).is_err());
        assert!(factory_create(&f, &[empty]).is_ok());
        // Незаполненное свойство — `Неопределено`, а постороннее имя —
        // ошибка (измерено обе стороны).
        assert_eq!(
            get_property(&object, "name").expect("свойство читается"),
            BslValue::Undefined
        );
        assert!(get_property(&object, "нетТакого").is_err());
        // `Тип` у экземпляра — метод, а не член: обращение как к свойству
        // отвечает ошибкой (измерено).
        assert!(get_property(&object, "Тип").is_err());
    }

    /// Фабрика по набору схем строится только из набора: путь, схема и
    /// прочее — ошибка (измерено), а `Неопределено` значит «без схем».
    #[test]
    fn a_factory_is_built_from_a_schema_set_or_from_nothing() {
        let empty = factory_of_schema_set(&BslValue::Undefined).expect("фабрика без схем");
        assert_eq!(empty.to_string(), "ФабрикаXDTO");
        assert_eq!(
            empty.type_of().unwrap(),
            BslValue::Type(TypeId::XdtoFactory)
        );
        assert_eq!(TypeId::XdtoFactory.name(), "Фабрика XDTO");
        assert!(is_factory(&empty));
        let set = crate::xsd::new_schema_set();
        assert!(factory_of_schema_set(&set).is_ok());
        // Путь к файлу, схема и число сюда не годятся.
        for wrong in [
            str_value("/tmp/схема.xsd"),
            crate::xsd::new_schema(),
            number_value(1),
        ] {
            assert!(factory_of_schema_set(&wrong).is_err(), "{wrong:?}");
        }
        // Две фабрики от одного и того же набора не равны (измерено на
        // двух фабриках от одного файла).
        assert_ne!(
            factory_of_schema_set(&set).expect("фабрика"),
            factory_of_schema_set(&set).expect("фабрика")
        );
        // `ЗначениеЗаполнено` от фабрики — ошибка (измерено).
        assert!(empty.is_filled().is_err());
        // Постороннего члена у фабрики нет.
        assert!(get_property(&empty, "Пакеты").is_err());
    }

    /// `СоздатьФабрикуXDTO` читает файл: несуществующий путь, нестроковый
    /// аргумент и содержимое, которое схемой не является, — ошибки.
    #[test]
    fn a_factory_from_a_file_reports_a_missing_or_broken_source() {
        let dir = std::env::temp_dir();
        let path = dir.join("open-bsl-xdto-factory-test.xsd");
        std::fs::write(
            &path,
            concat!(
                r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:f">"#,
                r#"<xs:simpleType name="Code"><xs:restriction base="xs:string"/>"#,
                r#"</xs:simpleType></xs:schema>"#,
            ),
        )
        .expect("временный файл пишется");
        let f = factory_of_file(&[str_value(&path.to_string_lossy())]).expect("фабрика");
        assert_eq!(
            factory_type(&f, &[str_value("urn:f"), str_value("Code")])
                .expect("тип")
                .to_string(),
            "{urn:f}Code"
        );
        let missing = dir.join("open-bsl-xdto-factory-нет-такого.xsd");
        let error = factory_of_file(&[str_value(&missing.to_string_lossy())])
            .expect_err("файла нет — ошибка");
        assert!(
            error
                .to_string()
                .contains("open-bsl-xdto-factory-нет-такого"),
            "в тексте ошибки нет пути: {error}"
        );
        // Не схема и не разметка вовсе.
        let broken = dir.join("open-bsl-xdto-factory-test-broken.xsd");
        std::fs::write(&broken, "<чепуха/>").expect("временный файл пишется");
        assert!(factory_of_file(&[str_value(&broken.to_string_lossy())]).is_err());
        // Ни без аргумента, ни с двумя, ни с нестроковым (измерено).
        assert!(factory_of_file(&[]).is_err());
        assert!(factory_of_file(&[number_value(1)]).is_err());
        assert!(factory_of_file(&[
            str_value(&path.to_string_lossy()),
            str_value(&path.to_string_lossy())
        ])
        .is_err());
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(&broken);
    }

    // --- экземпляр ---------------------------------------------------------

    /// Экземпляр `RootType` из фабрики над [`SAMPLE`].
    fn instance(f: &BslValue, name: &str) -> BslValue {
        let t = factory_type(f, &[str_value("urn:test"), str_value(name)]).expect("тип");
        factory_create(f, &[t]).expect("экземпляр")
    }

    /// Чтение свежего экземпляра: пусто там, где ничего не объявлено, и
    /// сразу значение там, где есть `default`/`fixed`.
    #[test]
    fn a_fresh_instance_answers_with_defaults_and_empty_lists() {
        let f = factory(SAMPLE);
        let o = instance(&f, "RootType");
        assert_eq!(prop(&o, "name"), BslValue::Undefined);
        assert_eq!(prop(&o, "opt"), BslValue::Undefined);
        // `default="7"` и `fixed="9"` подставляются при чтении (измерено).
        assert_eq!(prop(&o, "def"), number_value(7));
        assert_eq!(prop(&o, "fx"), number_value(9));
        // Поиск имени регистронезависим (измерено: `О.NAME`).
        assert_eq!(prop(&o, "NAME"), BslValue::Undefined);
        // Множественное свойство — всегда список, даже пустой.
        let list = prop(&o, "code");
        assert_eq!(list.to_string(), "СписокXDTO");
        assert_eq!(list.type_of().unwrap(), BslValue::Type(TypeId::XdtoList));
        assert_eq!(TypeId::XdtoList.name(), "Список XDTO");
        assert_eq!(list.collection_len().expect("длина"), 0);
        // Постороннее имя — ошибка, а не `Неопределено` (измерено).
        assert!(get_property(&o, "нетТакого").is_err());
        // Унаследованное свойство читается у наследника так же.
        assert_eq!(prop(&instance(&f, "ExtType"), "def"), number_value(7));
    }

    /// Запись идёт через лексическую форму типа-приёмника — отсюда и
    /// приведения, и отказы (измерено поимённо).
    #[test]
    fn writing_a_property_goes_through_the_lexical_form() {
        let f = factory(SAMPLE);
        let o = instance(&f, "RootType");
        set_property(&o, "name", str_value("аб")).expect("строка в строку");
        assert_eq!(text_of(&prop(&o, "name")), "аб");
        // Регистр имени не важен и на записи (измерено).
        set_property(&o, "NAME", number_value(5)).expect("число в строку");
        assert_eq!(text_of(&prop(&o, "name")), "5");
        set_property(&o, "name", BslValue::Boolean(true)).expect("булево в строку");
        assert_eq!(text_of(&prop(&o, "name")), "true");
        let day = BslValue::Date(crate::BslDate::from_civil(2026, 8, 13, 0, 0, 0).expect("дата"));
        set_property(&o, "name", day.clone()).expect("дата в строку");
        assert_eq!(text_of(&prop(&o, "name")), "2026-08-13T00:00:00");
        // В `xs:int` строка цифрами проходит, а «true» — нет: это не его
        // лексическая форма (измерено обе стороны).
        set_property(&o, "id", str_value("5")).expect("строка в число");
        assert_eq!(prop(&o, "id"), number_value(5));
        assert!(set_property(&o, "id", BslValue::Boolean(true)).is_err());
        // `Неопределено` и `Null` не пишутся вовсе — сброс делает
        // `Сбросить` (измерено).
        assert!(set_property(&o, "name", BslValue::Undefined).is_err());
        assert!(set_property(&o, "name", BslValue::Null).is_err());
        // `ЗначениеXDTO` принимается, и берётся из него ЗНАЧЕНИЕ: тип
        // источника может быть другим (измерено).
        let int_type = factory_type(
            &f,
            &[
                str_value("http://www.w3.org/2001/XMLSchema"),
                str_value("string"),
            ],
        )
        .expect("тип");
        let value = factory_create(&f, &[int_type, str_value("5")]).expect("значение");
        set_property(&o, "id", value).expect("значение XDTO в число");
        assert_eq!(prop(&o, "id"), number_value(5));
        // Множественное свойство присваиванием не пишется (измерено).
        assert!(set_property(&o, "code", str_value("AB")).is_err());
        assert!(set_property(&o, "нетТакого", str_value("аб")).is_err());
        // Свойство типа `anyType` принимает что угодно как есть (измерено).
        set_property(&o, "notype", number_value(5)).expect("число в anyType");
        assert_eq!(prop(&o, "notype"), number_value(5));
    }

    /// Список — окно в хранилище владельца, а не снимок.
    #[test]
    fn a_list_is_a_window_into_its_owner() {
        let f = factory(SAMPLE);
        let o = instance(&f, "RootType");
        let list = prop(&o, "code");
        list_add(&list, &[str_value("AB")]).expect("добавление");
        // Видно через второе чтение свойства, и два чтения РАВНЫ
        // (измерено обе стороны).
        assert_eq!(prop(&o, "code").collection_len().expect("длина"), 1);
        assert_eq!(prop(&o, "code"), prop(&o, "code"));
        assert_eq!(prop(&list, "Владелец"), o);
        // В списке лежит само значение, а не `ЗначениеXDTO` (измерено).
        assert_eq!(text_of(&list_get(&list, 0).expect("элемент")), "AB");
        assert!(list_get(&list, 1).is_err());
        // Приведение то же, что при записи свойства.
        list_add(&prop(&o, "many5"), &[number_value(5)]).expect("число в строку");
        assert_eq!(
            text_of(&list_get(&prop(&o, "many5"), 0).expect("элемент")),
            "5"
        );
        assert!(list_add(&list, &[BslValue::Undefined]).is_err());
        // `Вставить` встаёт на место указанного элемента, `Удалить` и
        // `Очистить` работают по позиции (измерено).
        list_insert(&list, &[number_value(0), str_value("CD")]).expect("вставка");
        assert_eq!(text_of(&list_get(&list, 0).expect("элемент")), "CD");
        assert_eq!(list.collection_len().expect("длина"), 2);
        list_set(&list, &[number_value(0), str_value("EF")]).expect("установка");
        assert_eq!(text_of(&list_get(&list, 0).expect("элемент")), "EF");
        assert!(list_set(&list, &[number_value(9), str_value("EF")]).is_err());
        // `Вставить` требует ЗАНЯТОЙ позиции: ни за концом, ни в пустой
        // список платформа не вставляет (измерено оба).
        assert!(list_insert(&list, &[number_value(2), str_value("EF")]).is_err());
        list_delete(&list, &number_value(0)).expect("удаление");
        assert_eq!(list.collection_len().expect("длина"), 1);
        list_clear(&list).expect("очистка");
        assert_eq!(list.collection_len().expect("длина"), 0);
        assert!(list_delete(&list, &number_value(0)).is_err());
        assert!(list_insert(&list, &[number_value(0), str_value("EF")]).is_err());
    }

    /// Члены экземпляра: заполненность — это ЗАПИСЬ, а не наличие
    /// значения при чтении.
    #[test]
    fn instance_members_tell_filling_from_defaults() {
        let f = factory(SAMPLE);
        let o = instance(&f, "RootType");
        // У свойства с `default` чтение даёт 7, а `Установлено` — «Нет»
        // (измерено).
        assert_eq!(
            object_is_set(&o, &[str_value("def")]).expect("установлено"),
            BslValue::Boolean(false)
        );
        object_set(&o, &[str_value("name"), str_value("аб")]).expect("установка");
        assert_eq!(
            object_is_set(&o, &[str_value("name")]).expect("установлено"),
            BslValue::Boolean(true)
        );
        assert_eq!(
            text_of(&object_get(&o, &[str_value("name")]).expect("чтение")),
            "аб"
        );
        // Свойство можно назвать и объектом `СвойствоXDTO` (измерено).
        let properties = prop(&object_type(&o, &[]).expect("тип"), "Свойства");
        let name = collection_lookup(&properties, &[str_value("name")]).expect("свойство");
        assert_eq!(
            text_of(&object_get(&o, std::slice::from_ref(&name)).expect("чтение")),
            "аб"
        );
        object_unset(&o, &[name]).expect("сброс");
        assert_eq!(prop(&o, "name"), BslValue::Undefined);
        // Множественное свойство `Получить` не отдаёт, а `ПолучитьСписок`
        // отдаёт — и это тот же список (измерено).
        assert!(object_get(&o, &[str_value("code")]).is_err());
        let list = object_get_list(&o, &[str_value("code")]).expect("список");
        list_add(&list, &[str_value("AB")]).expect("добавление");
        assert_eq!(prop(&o, "code").collection_len().expect("длина"), 1);
        assert!(object_get_list(&o, &[str_value("name")]).is_err());
        // Постороннее имя — ошибка у всех четырёх (измерено).
        assert!(object_get(&o, &[str_value("нетТакого")]).is_err());
        assert!(object_is_set(&o, &[str_value("нетТакого")]).is_err());
        assert!(object_unset(&o, &[str_value("нетТакого")]).is_err());
        assert!(object_get_list(&o, &[str_value("нетТакого")]).is_err());
        // `Свойства()` — коллекция свойств СВОЕГО типа, `Владелец()` у
        // отдельно созданного объекта — `Неопределено` (измерено).
        assert_eq!(
            object_properties(&o, &[])
                .expect("свойства")
                .collection_len()
                .expect("длина"),
            properties.collection_len().expect("длина")
        );
        assert_eq!(
            object_owner(&o, &[]).expect("владелец"),
            BslValue::Undefined
        );
    }

    /// Вложенный объект: тот же экземпляр, свой владелец и рекурсивная
    /// проверка границ.
    #[test]
    fn a_nested_object_keeps_its_owner_and_is_validated() {
        let f = factory(SAMPLE);
        let o = instance(&f, "RootType");
        let anon = prop(
            &collection_lookup(
                &prop(&object_type(&o, &[]).expect("тип"), "Свойства"),
                &[str_value("anon")],
            )
            .expect("свойство"),
            "Тип",
        );
        let nested = factory_create(&f, &[anon]).expect("экземпляр анонимного типа");
        set_property(&o, "anon", nested.clone()).expect("объект в объектное свойство");
        // Записан ТОТ ЖЕ объект, и владелец у него — приёмник (измерено).
        assert_eq!(prop(&o, "anon"), nested);
        assert_eq!(object_owner(&nested, &[]).expect("владелец"), o);
        // Посторонний тип в это свойство не пишется (измерено; наследник
        // объявленного — пишется, но в этой схеме объектного свойства с
        // ИМЕНОВАННЫМ типом нет, и проверено это на платформе).
        assert!(set_property(&o, "anon", instance(&f, "EmptyType")).is_err());
        // `Проверить` смотрит и внутрь: вложенный объект пуст, а `inner`
        // у него обязателен (измерено).
        assert!(object_validate(&o, &[]).is_err());
        object_validate(&instance(&f, "EmptyType"), &[]).expect("пустой тип проходит");
        set_property(&nested, "inner", number_value(1)).expect("запись во вложенный");
        set_property(&o, "name", str_value("аб")).expect("запись");
        set_property(&o, "uq", str_value("вг")).expect("запись");
        set_property(&o, "id", number_value(1)).expect("запись");
        list_add(&prop(&o, "many5"), &[str_value("я")]).expect("добавление");
        object_validate(&o, &[]).expect("заполненный объект проходит");
        // Верхняя граница тоже проверяется: у `many5` она 5 (измерено).
        for _ in 0..5 {
            list_add(&prop(&o, "many5"), &[str_value("я")]).expect("добавление");
        }
        assert!(object_validate(&o, &[]).is_err());
    }

    /// Последовательность — порядок заполнения свойств-элементов; у
    /// упорядоченного типа её нет вовсе.
    #[test]
    fn the_sequence_follows_the_order_of_filling() {
        let f = factory(SAMPLE);
        // `xs:sequence` — упорядоченный тип, у него `Неопределено`
        // (измерено), а `xs:choice` и `xs:all` — последовательные.
        assert_eq!(
            object_sequence(&instance(&f, "RootType"), &[]).expect("последовательность"),
            BslValue::Undefined
        );
        let o = instance(&f, "ChoiceType");
        let seq = object_sequence(&o, &[]).expect("последовательность");
        assert_eq!(seq.to_string(), "ПоследовательностьXDTO");
        assert_eq!(seq.type_of().unwrap(), BslValue::Type(TypeId::XdtoSequence));
        assert_eq!(TypeId::XdtoSequence.name(), "Последовательность XDTO");
        assert_eq!(seq.collection_len().expect("длина"), 0);
        // Порядок: заполнение элементов, атрибут в него не попадает
        // (измерено).
        list_add(&prop(&o, "cb"), &[str_value("аб")]).expect("добавление");
        set_property(&o, "ca", str_value("вг")).expect("запись");
        set_property(&o, "cat", str_value("атрибут")).expect("запись атрибута");
        list_add(&prop(&o, "cb"), &[str_value("де")]).expect("добавление");
        assert_eq!(seq.collection_len().expect("длина"), 3);
        assert_eq!(
            text_of(&sequence_value(&seq, &[number_value(1)]).expect("значение")),
            "вг"
        );
        assert_eq!(
            sequence_property(&seq, &[number_value(1)])
                .expect("свойство")
                .to_string(),
            "ca"
        );
        assert!(sequence_value(&seq, &[number_value(3)]).is_err());
        // Повторная запись одиночного свойства своё место сохраняет
        // (измерено).
        set_property(&o, "ca", str_value("же")).expect("повторная запись");
        assert_eq!(seq.collection_len().expect("длина"), 3);
        assert_eq!(
            text_of(&sequence_value(&seq, &[number_value(1)]).expect("значение")),
            "же"
        );
        // `Владелец` у последовательности — ЧЛЕН (измерено), а два вызова
        // `Последовательность()` дают равные значения.
        assert_eq!(prop(&seq, "Владелец"), o);
        assert_eq!(object_sequence(&o, &[]).expect("вторая"), seq);
        // `Добавить` берёт именно `СвойствоXDTO` и именно элемент, а
        // заполнение видно через само свойство (измерено).
        let properties = prop(&object_type(&o, &[]).expect("тип"), "Свойства");
        let ca = collection_lookup(&properties, &[str_value("ca")]).expect("свойство");
        let cat = collection_lookup(&properties, &[str_value("cat")]).expect("свойство");
        sequence_add(&seq, &[ca, str_value("зи")]).expect("добавление");
        assert_eq!(seq.collection_len().expect("длина"), 4);
        assert_eq!(text_of(&prop(&o, "ca")), "зи");
        assert!(sequence_add(&seq, &[cat, str_value("к")]).is_err());
        assert!(sequence_add(&seq, &[str_value("ca"), str_value("к")]).is_err());
        // `Очистить` забывает элементы, атрибут уцелевает (измерено).
        sequence_clear(&seq).expect("очистка");
        assert_eq!(seq.collection_len().expect("длина"), 0);
        assert_eq!(prop(&o, "ca"), BslValue::Undefined);
        assert_eq!(text_of(&prop(&o, "cat")), "атрибут");
    }
}
