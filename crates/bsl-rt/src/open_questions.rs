//! Реестр ОТКРЫТЫХ ВОПРОСОВ — решений, принятых рассуждением, а не сверкой
//! с реальной 1С.
//!
//! Зачем реестр, если метки и так стоят в коде: метка видна только тому, кто
//! читает конкретный файл. Реестр же даёт список, который целиком
//! отрабатывается за один сеанс у платформы — а
//! `tests/conformance/measure/measure-all.bsl` превращает этот список в один
//! прогоняемый скрипт.
//!
//! Три вещи держатся согласованными и проверяются тестом
//! `crates/bsl-rt/tests/open_questions_registry.rs`:
//!
//! 1. метка в коде — комментарий вида `НЕ ИЗМЕРЕНО(ID)`; ID обязателен,
//!    метка без скобок теперь ошибка теста;
//! 2. запись в [`OPEN_QUESTIONS`] — что за вопрос, что выбрано и что этим
//!    заблокировано;
//! 3. ровно одна строка в скрипте замеров с тем же ID.
//!
//! Каждый ID встречается в коде хотя бы раз (обычно дважды: у реализации и у
//! теста, который фиксирует ВЫБРАННОЕ поведение) и ровно один раз в скрипте.
//!
//! Область в ID (`NUM`, `SQRT`, `DATE`, `STR`, `FMT`, `TYPE`, `TABLE`,
//! `EXEC`) выбирается по вопросу, а не по крейту: `FMT.DATE.DEFAULT` — про
//! представление даты, хотя метки стоят и в `bsl-rt`, и в `bsl-format`.
//!
//! Правило, которое реестр охраняет: эталон НИКОГДА не берётся из вывода
//! этой же реализации. Замер приходит с платформы, сравнение делает
//! `bsl-cli --ingest-measurements`, а что чинить — решает человек.

/// Один открытый вопрос.
pub struct OpenQuestion {
    /// Стабильный ID вида `ОБЛАСТЬ.ВОПРОС`. Тот же текст стоит в метке
    /// `НЕ ИЗМЕРЕНО(ID)` и в скрипте замеров.
    pub id: &'static str,
    /// Сам вопрос: что именно неизвестно про платформу.
    pub what: &'static str,
    /// Что выбрано здесь, пока замера нет.
    pub chosen: &'static str,
    /// Что этим заблокировано: тест, фикстура или функция, эталон которой
    /// нельзя написать до замера.
    pub blocks: &'static str,
}

/// Известное значение платформы — НЕ вопрос, а якорь.
///
/// Якоря живут в том же скрипте замеров и нужны как канарейка на весь
/// сеанс: если платформа выдала на `NUM.DIV.THIRD` что-то отличное от
/// `expect`, значит сеанс снят не с той конфигурации (другая локаль, другая
/// версия, вывод пропущен через `Строка` не там), и остальным строкам
/// доверять нельзя ещё до разбора расхождений.
pub struct Anchor {
    pub id: &'static str,
    /// Точный текст, который платформа обязана напечатать.
    pub expect: &'static str,
    /// Откуда значение известно.
    pub note: &'static str,
}

pub const OPEN_QUESTIONS: &[OpenQuestion] = &[
    OpenQuestion {
        id: "NUM.ROUND.DEFAULT_MODE",
        what: "схема округления `Округл` без третьего аргумента",
        chosen: "half-up — та же схема, что у подтверждённого замером деления",
        blocks: "bsl_number::DEFAULT_ROUND_MODE; bsl-vm::tests::round_takes_an_explicit_mode",
    },
    OpenQuestion {
        id: "NUM.ROUND.MODE_CODES",
        what: "какими числами платформа кодирует режимы `Округл` и что такое \
               «Окр15как10» — half-even или половина ВНИЗ",
        chosen: "0 -> умолчание, 1 -> half-even; прочие коды — ошибка типа",
        blocks: "bsl_number::RoundMode; bsl-number/tests/oracle.rs::\
                 round_to_scale_half_even_differs_from_half_up_only_on_exact_ties",
    },
    OpenQuestion {
        id: "SQRT.SMALL_ARG",
        what: "как платформа округляет результат f64 при малом аргументе: \
               модель «15 значащих» воспроизводит Sqrt(2), но не Sqrt(0.02)",
        chosen: "15 значащих с округлением (`F64_SIG`), расхождение признано \
                 неразрешённым",
        blocks: "bsl-number/tests/oracle.rs::sqrt_small_argument_unresolved (#[ignore])",
    },
    OpenQuestion {
        id: "DATE.WEEKDAY_NUMBERING",
        what: "нумерация `ДеньНедели`: понедельник = 1 или воскресенье = 1",
        chosen: "понедельник = 1 .. воскресенье = 7 (так же считает `НачалоНедели`)",
        blocks: "bsl_rt::BslDate::weekday; фикстура dates",
    },
    OpenQuestion {
        id: "DATE.ADD_MONTH_CLAMP",
        what: "что делает `ДобавитьМесяц` с днём, которого в целевом месяце нет",
        chosen: "зажатие в последний день месяца (31.01 + 1 месяц = 29.02)",
        blocks: "bsl_rt::BslDate::add_months; фикстура dates",
    },
    OpenQuestion {
        id: "DATE.PATTERN_LETTERS",
        what: "полный набор букв шаблона `ДФ` сверх реализованных (`К` — квартал, \
               `в` — до/после полудня и т.п.)",
        chosen: "реализованы только буквы из брифа, остальные копируются как текст",
        blocks: "bsl_rt::date::format_pattern; фикстура dates",
    },
    OpenQuestion {
        id: "DATE.LONG_FORMAT_CODES",
        what: "набор кодов `ДЛФ` и вид каждого в русской локали",
        chosen: "Д/ДД/ДДД/В плюс ДВ; неизвестный код — формат по умолчанию, не ошибка",
        blocks: "bsl_rt::date::format_long; фикстура dates",
    },
    OpenQuestion {
        id: "FMT.DATE.DEFAULT",
        what: "представление даты по умолчанию: печатается ли время у полуночной \
               даты, есть ли ведущий ноль в часах, как выглядит пустая дата",
        chosen: "всегда `дд.ММ.гггг ЧЧ:мм:сс`, пустая дата — `01.01.0001 00:00:00`",
        blocks: "bsl_rt::date::DEFAULT_PATTERN; фикстура dates",
    },
    OpenQuestion {
        id: "FMT.NUM.TOTAL_DIGITS",
        what: "что считает `ЧЦ` — все разряды или только целые — и что делает \
               платформа, когда одна целая часть уже длиннее `ЧЦ`",
        chosen: "все разряды вместе; целая часть при переполнении печатается как есть",
        blocks: "bsl_format::NumberFormat::total_digits",
    },
    OpenQuestion {
        id: "FMT.NUM.ZERO_TEXT",
        what: "`ЧН` — применяется ли к значению, ставшему нулём после округления, \
               и что значит `ЧН=` без значения",
        chosen: "применяется после округления; `ЧН=` — пустая строка",
        blocks: "bsl_format::NumberFormat::zero_text",
    },
    OpenQuestion {
        id: "FMT.NUM.LEADING_ZEROS",
        what: "`ЧВН` — до какой ширины дополнять ведущими нулями",
        chosen: "до `ЧЦ` минус дробные разряды; без `ЧЦ` ключ не делает ничего",
        blocks: "bsl_format::NumberFormat::leading_zeros",
    },
    OpenQuestion {
        id: "FMT.NUM.SHIFT",
        what: "`ЧС` — в какую сторону сдвигает разряды и что значит отрицательное \
               значение",
        chosen: "`ЧС=n` делит на 10^n (точным умножением), отрицательное умножает; \
                 |n| > 30 — ошибка",
        blocks: "bsl_format::shift_digits",
    },
    OpenQuestion {
        id: "FMT.BOOLEAN.TRUE_TEXT",
        what: "`БИ` — текст для `Истина`, и что происходит, когда задан только он",
        chosen: "`БИ` переопределяет только истину; ложь остаётся локальной",
        blocks: "bsl_format::BooleanFormat",
    },
    OpenQuestion {
        id: "FMT.BOOLEAN.FALSE_TEXT",
        what: "`БЛ` — текст для `Ложь`, и что происходит, когда задан только он",
        chosen: "`БЛ` переопределяет только ложь; истина остаётся локальной",
        blocks: "bsl_format::BooleanFormat",
    },
    OpenQuestion {
        id: "FMT.LOCALE.KEY",
        what: "имя ключа локали (`Л`/`L`) и что значит пустое значение",
        chosen: "`Л=<код>`; без ключа — русская локаль",
        blocks: "bsl_format::parse_locale",
    },
    OpenQuestion {
        id: "FMT.LOCALE.COVERAGE",
        what: "какие коды локалей платформа понимает и что делает с незнакомым",
        chosen: "поддержаны только `ru`/`en` (с любым регионом), остальное — \
                 внятная ошибка вместо молчаливого отката к русской",
        blocks: "bsl_rt::Locale::parse",
    },
    OpenQuestion {
        id: "FMT.LOCALE.BOOLEAN",
        what: "как выглядит `Истина` в английской локали — `Yes`, `True` или `1`",
        chosen: "`Yes`/`No` — зеркало измеренного русского `Да`/`Нет`",
        blocks: "bsl_rt::Locale::boolean_text",
    },
    OpenQuestion {
        id: "STR.CHAR_CODE_SURROGATE",
        what: "что возвращает `КодСимвола` на первой половине суррогатной пары — \
               код-юнит (55357) или кодовую точку (128512)",
        chosen: "кодовую точку, ради обратимости `КодСимвола(Символ(к)) = к`",
        blocks: "bsl_rt::BslString::char_code_at; фикстура strings-and-types",
    },
    OpenQuestion {
        id: "TYPE.IS_FILLED.BOOLEAN",
        what: "считает ли `ЗначениеЗаполнено` значение `Ложь` незаполненным",
        chosen: "булево заполнено всегда, включая `Ложь`",
        blocks: "bsl_rt::BslValue::is_filled; фикстура strings-and-types",
    },
    OpenQuestion {
        id: "TYPE.IS_FILLED.BLANK_STRING",
        what: "пуста ли для `ЗначениеЗаполнено` строка из одних пробелов",
        chosen: "пуста (сравнение после `СокрЛП`)",
        blocks: "bsl_rt::BslValue::is_filled; фикстура strings-and-types",
    },
    OpenQuestion {
        id: "TYPE.IS_FILLED.EMPTY_COLLECTION",
        what: "заполнена ли пустая коллекция (`Массив`, `Структура`, `Соответствие`)",
        chosen: "не заполнена — по числу элементов",
        blocks: "bsl_rt::BslValue::is_filled; фикстура strings-and-types",
    },
    OpenQuestion {
        id: "TABLE.TOTAL.NON_NUMERIC",
        what: "что делает `Итог` с нечисловыми значениями колонки — игнорирует, \
               падает или считает их нулём",
        chosen: "игнорирует; колонка из одних нечисловых даёт 0",
        blocks: "bsl_rt::ValueTableData::total; фикстура table-wave2",
    },
    OpenQuestion {
        id: "TABLE.SORT.COLLATION",
        what: "как платформа упорядочивает строки: место `ё` относительно `е`, \
               роль регистра, взаимный порядок кириллицы, латиницы и цифр",
        chosen: "приближение «сначала ВРег, затем исходный вид» — заведомо неточное",
        blocks: "bsl_rt::table::collate; фикстура table-wave2",
    },
    OpenQuestion {
        id: "TABLE.SORT.TYPE_ORDER",
        what: "в каком порядке платформа ставит РАЗНОТИПНЫЕ значения одной колонки",
        chosen: "пустые, числа, даты, булево, строки, типы, объекты — произвольно, \
                 но устойчиво",
        blocks: "bsl_rt::table::type_rank; фикстура table-wave2",
    },
    OpenQuestion {
        id: "TABLE.COLLAPSE.OTHER_COLUMNS",
        what: "что `Свернуть` делает с колонками, не попавшими ни в группировку, \
               ни в суммирование, и в каком порядке остаются оставшиеся",
        chosen: "удаляются; порядок — сначала группировочные, потом суммируемые",
        blocks: "bsl_rt::ValueTableData::collapse; фикстура table-wave3",
    },
    OpenQuestion {
        id: "TABLE.COLLAPSE.ROW_ORDER",
        what: "порядок строк результата `Свернуть`: первое вхождение группы или \
               сортировка по ключам группировки",
        chosen: "порядок первого вхождения — исходная сортировка не затирается",
        blocks: "bsl_rt::ValueTableData::collapse; фикстура table-wave3",
    },
    OpenQuestion {
        id: "TABLE.COLLAPSE.NON_NUMERIC",
        what: "суммирование колонки с нечисловыми значениями в `Свернуть`",
        chosen: "нечисловые игнорируются — то же решение, что у `Итог`",
        blocks: "bsl_rt::ValueTableData::collapse; фикстура table-wave3",
    },
    OpenQuestion {
        id: "TABLE.LOAD_COLUMN.LENGTH_MISMATCH",
        what: "что делает `ЗагрузитьКолонку`, когда длина массива не равна числу \
               строк таблицы",
        chosen: "лишние значения игнорируются, недостающие оставляют ячейку прежней; \
                 число строк не меняется",
        blocks: "bsl_rt::ValueTableData::load_column; фикстура table-wave3",
    },
    OpenQuestion {
        id: "TABLE.MOVE.OUT_OF_RANGE",
        what: "`Сдвинуть` за границы таблицы — ошибка или зажатие в границы",
        chosen: "ошибка `IndexOutOfBounds`",
        blocks: "bsl_rt::BslValue::table_move; фикстура table-wave3",
    },
    OpenQuestion {
        id: "EXEC.NEW_VARIABLE_SCOPE",
        what: "переживает ли вызов переменная, впервые созданная ВНУТРИ `Выполнить`",
        chosen: "не переживает — расширить статически размеченный кадр нечем",
        blocks: "bsl-vm::run_dynamic_snippet; фикстура dynamic-execute",
    },
    OpenQuestion {
        id: "EXEC.USER_FUNCTION_CALL",
        what: "видит ли фрагмент `Выполнить` пользовательские процедуры и функции \
               окружающего модуля",
        chosen: "не видит — таблица сигнатур во фрагмент не передаётся",
        blocks: "bsl_sema::resolve_snippet_stmts; фикстура dynamic-execute",
    },
    OpenQuestion {
        id: "EXEC.PROC_DECLARATION",
        what: "может ли фрагмент `Выполнить` объявлять процедуры и функции",
        chosen: "нет — объявление во фрагменте это `DynamicError`",
        blocks: "bsl-vm::compile_dynamic_snippet; фикстура dynamic-execute",
    },
];

/// Уже измеренные значения — якоря сеанса. Менять их можно только новым
/// замером (см. README, раздел «Уже измеренные эталоны»).
pub const MEASURED_ANCHORS: &[Anchor] = &[
    Anchor {
        id: "NUM.DIV.THIRD",
        expect: "0.333333333333333333333333333",
        note: "27 знаков после точки — лимит деления на масштабе, не на значащих",
    },
    Anchor {
        id: "NUM.DIV.TWO_THIRDS",
        expect: "0.666666666666666666666666667",
        note: "округление вверх на последнем разряде",
    },
    Anchor {
        id: "NUM.DIV.TEN_THIRDS",
        expect: "3.333333333333333333333333333",
        note: "27 знаков после точки при 28 значащих",
    },
    Anchor {
        id: "NUM.DIV.EXACT_TIE",
        expect: "0.000000003725290298461914063",
        note: "1/2^28 — точная ничья; ...063 доказывает half-up",
    },
    Anchor {
        id: "SQRT.TWO",
        expect: "1.4142135623731",
        note: "15 значащих на возврате из f64 (`F64_SIG`)",
    },
    Anchor {
        id: "NUM.MUL.TRAILING_ZEROS",
        expect: "1.1",
        note: "1.10 * 1.00 — хвостовые нули срезаются",
    },
    Anchor {
        id: "NUM.POW.TEN_TO_30",
        expect: "1000000000000000000000000000000",
        note: "целый показатель считается точно, не через f64",
    },
    Anchor {
        id: "NUM.STRLEN_OF_THIRD",
        expect: "29",
        note: "СтрДлина(Строка(1/3)): 0 + запятая + 27 знаков",
    },
    Anchor {
        id: "FMT.NUMBER.DEFAULT",
        expect: "1\u{A0}000,5",
        note: "группировка NBSP и запятая — умолчание Строка()",
    },
    Anchor {
        id: "FMT.GROUP_SEP_CODE",
        expect: "160",
        note: "разделитель групп — NBSP (U+00A0), не пробел",
    },
    Anchor {
        id: "FMT.NUMBER.ROUND_TRIP",
        expect: "1000000",
        note: "Число(Строка(1000000)) — обратный разбор живой",
    },
    Anchor {
        id: "FMT.BOOLEAN.TRUE",
        expect: "Да",
        note: "Строка(Истина)",
    },
    Anchor {
        id: "FMT.ARRAY.DEFAULT",
        expect: "Массив",
        note: "Строка(Новый Массив) — имя типа, не содержимое",
    },
];

/// Поиск вопроса по ID — нужен `bsl-cli --ingest-measurements`, чтобы
/// показать рядом с расхождением, что именно было выбрано.
pub fn question(id: &str) -> Option<&'static OpenQuestion> {
    OPEN_QUESTIONS.iter().find(|q| q.id == id)
}

/// Поиск якоря по ID.
pub fn anchor(id: &str) -> Option<&'static Anchor> {
    MEASURED_ANCHORS.iter().find(|a| a.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_across_questions_and_anchors() {
        let mut ids: Vec<&str> = OPEN_QUESTIONS
            .iter()
            .map(|q| q.id)
            .chain(MEASURED_ANCHORS.iter().map(|a| a.id))
            .collect();
        ids.sort_unstable();
        let before = ids.len();
        ids.dedup();
        assert_eq!(before, ids.len(), "ID повторяется в реестре: {ids:?}");
    }

    #[test]
    fn ids_look_like_area_dot_question() {
        for id in OPEN_QUESTIONS
            .iter()
            .map(|q| q.id)
            .chain(MEASURED_ANCHORS.iter().map(|a| a.id))
        {
            assert!(id.contains('.'), "ID без области: {id}");
            assert!(
                id.chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '.' || c == '_'),
                "ID не в верхнем регистре ASCII: {id}"
            );
        }
    }
}
