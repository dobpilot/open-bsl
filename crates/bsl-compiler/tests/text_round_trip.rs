//! Round-trip текстового формата на скомпилированном корпусе.
//!
//! Живёт здесь, а не в `bsl-bytecode`: печать и разбор проверяются на том,
//! что действительно выпускает кодоген, — а он тут. Сам `bsl-bytecode`
//! фронтенда не видит и проверяется на программах, собранных руками.

use std::cell::RefCell;

use bsl_bytecode::{Instr, LibraryRequirement, OPCODES, Program, parse_program, write_program};

/// Корпус, на котором проверяется round-trip. Он же — покрытие
/// опкодов: тест ниже требует, чтобы КАЖДЫЙ опкод из `OPCODES`
/// встретился хотя бы раз, иначе расхождение печати и разбора
/// обнаружилось бы уже у пользователя.
const CORPUS: &[&str] = &[
    // Арифметика, сравнения, логика, унарные.
    "х = 1 + 2 * 3 - 4 / 5;\nх = х + 1;\nу = -х;\nз = Не (х > 1 И х < 10 ИЛИ х = 5);\n\
     р = х <> 1; р = х >= 1; р = х <= 1; р = х = 1;\nк = у;\n",
    // Литералы всех видов, включая дату и строку с кавычками.
    "а = Истина; б = Ложь; в = Неопределено; г = Null;\n\
     д = '20240115103000'; е = \"строка с \"\"кавычками\"\" и ;\";\n",
    // Тернарный оператор: своего опкода у него нет (компилируется
    // переходами), но переходы эти в корпусе должны быть — вместе с
    // булевым результатом `И`/`ИЛИ`.
    "х = ?(Истина, 1, 2);\nу = ?(1 И 0, \"а\", ?(Ложь, \"б\", \"в\"));\n",
    // Остаток от деления — свой опкод, и в корпусе он обязан быть.
    "х = 7 % 2;\nу = -7 % 2 + 10 % 3 * 2;\n",
    // Ветвление и оба вида короткого замыкания.
    "а = 1; Если а = 1 Тогда х = 1; ИначеЕсли а = 2 Тогда х = 2; Иначе х = 3; КонецЕсли;\n",
    "а = 0; Пока а < 3 Цикл а = а + 1; КонецЦикла;\n",
    // Явный `Goto` нового синтаксиса понижается в тот же `Jump`.
    "Goto ~Конец;\nх = 1;\n~Конец:;\n",
    // Числовой цикл с телом и заведомо пустой (для NumericForNextI64).
    "Для ном = 1 По 10 Цикл с = ном; КонецЦикла;\nДля пуст = 1 По 10 Цикл КонецЦикла;\n",
    // Цикл по коллекции: CollectionLen, GetIndex.
    "м = Новый Массив; м.Добавить(1); Для Каждого э Из м Цикл ч = э; КонецЦикла;\n",
    // Коллекции: NewArray/NewStructure/NewTable/NewMap, Get/SetIndex, Get/SetProp.
    "м = Новый Массив(3);\nм[0] = 5;\nз = м[0];\n\
     с = Новый Структура(\"поле,ещё\", 1, 2);\nс.поле = с.ещё;\n\
     т = Новый ТаблицаЗначений;\nо = Новый ОписаниеТипов(\"Число\");\n\
     ср = Новый СравнениеЗначений;\nсо = Новый Соответствие;\n\
     со.Вставить(\"к\", 1);\nн = м.Количество();\n",
    // Пользовательские функции: Call, Return, параметры по ссылке и
    // по значению, значение по умолчанию и пропущенный аргумент.
    "Функция Ф(а, Знач б = 7, в = 9)\n  а = а + б + в;\n  Возврат а;\nКонецФункции\n\
     Процедура П()\n  Возврат;\nКонецПроцедуры\n\
     х = 1;\nу = Ф(х, , 3);\nП();\n",
    // Модульная переменная по ссылке из процедуры — ArgMode::ByRefModuleVar
    // (`bymodvar:` в тексте): внутри процедуры `М` — это `RExpr::ModuleVar`,
    // и параметр без `Знач` алиасит её module-слот, а не копирует.
    "Перем М;\nПроцедура Подменить(п)\n  п = 5;\nКонецПроцедуры\n\
     Процедура Вызвать()\n  Подменить(М);\nКонецПроцедуры\nМ = 1;\nВызвать();\n",
    // Исключения: обработчик и обе формы ВызватьИсключение.
    "Попытка\n  ВызватьИсключение \"беда\";\nИсключение\n  ВызватьИсключение;\nКонецПопытки;\n",
    // Встроенные функции и методы объектов.
    "х = Sqrt(2);\nс = Формат(1/3, \"ЧГ=0\");\n\
     т = Новый ТаблицаЗначений;\nт.Колонки.Добавить(\"ц\");\nт.Свернуть(\"ц\");\n",
    // Члены перечислений — константы: и член (`Перечисление ...` в
    // таблице констант), и голое имя перечисления обязаны пережить
    // печать и разбор.
    "п = ПорядокБайтов.BigEndian;\nт = ВариантЗаписиДатыJSON;\n",
    // Динамическое исполнение — обе формы.
    "х = 1;\nВыполнить(\"х = 2\");\nу = Вычислить(\"х + 1\");\n",
    // Переменные уровня модуля: чтение и запись из процедуры.
    "Перем Общая;\n\
     Процедура Пишет()\n  Общая = 1;\nКонецПроцедуры\n\
     Функция Читает()\n  Возврат Общая;\nКонецФункции\n\
     Общая = 0;\nПишет();\nх = Читает();\n",
    // Запись текста: NewTextWriter и оба горячих пути.
    "з = Новый ЗаписьТекста(\"/dev/null\");\nз.Записать(\"строка\");\nз.Закрыть();\n",
    // Двоичные данные: ядровой `CreateObject`, метод `Размер` и обе
    // глобальные функции — печать и разбор имён у них общие с
    // остальными CallBuiltin/CallMethod, но задеть их корпус обязан.
    "д = Новый ДвоичныеДанные(\"/dev/null\");\nн = д.Размер();\n\
     ч = РазделитьДвоичныеДанные(д, 4);\nц = СоединитьДвоичныеДанные(ч);\n",
    // УИД: обе формы конструктора — случайный и разбор строки.
    "у = Новый УникальныйИдентификатор;\n\
     ф = Новый УникальныйИдентификатор(\"abcdef12-3456-7890-abcd-ef1234567890\");\n\
     с = Строка(ф);\n",
    // Открытое имя метода, которого нет в таблице ядра: такой вызов
    // предназначен для объекта статически подключённого компонента.
    "объект = Новый Структура;\nобъект.МетодКомпонента();\n",
];

fn compile(src: &str) -> Program {
    let parsed = bsl_syntax::parse(src).unwrap_or_else(|e| panic!("{src}\nparse: {e:?}"));
    let resolved =
        bsl_sema::resolve_program(&parsed.items).unwrap_or_else(|e| panic!("{src}\nsema: {e:?}"));
    bsl_compiler::compile_program(&resolved).unwrap_or_else(|e| panic!("{src}\ncompile: {e:?}"))
}

fn call_component_program() -> Program {
    let mut program = compile("Возврат 1;");
    program
        .requirements
        .push(LibraryRequirement::new("bsl-test-host", "1.2.3"));
    program.chunks[0].instrs[0] = Instr::CallComponent {
        dst: 0,
        library: 1,
        function: 7,
        base: 0,
        count: 0,
    };
    program.chunks[0].instrs[1] = Instr::CreateObject {
        dst: 0,
        library: 1,
        constructor: 9,
        base: 0,
        count: 0,
    };
    let name: u16 = program.names.len().try_into().unwrap();
    program.names.push("ПолеКомпонента".to_string());
    program.chunks[0].instrs.extend([
        Instr::GetObjectProp {
            dst: 0,
            obj: 0,
            name,
        },
        Instr::SetObjectProp {
            obj: 0,
            name,
            src: 0,
        },
    ]);
    let instruction_count = program.chunks[0].instrs.len();
    program.chunks[0]
        .prop_cache
        .resize_with(instruction_count, || RefCell::new(None));
    program.chunks[0].bundle_len = bsl_bytecode::bundle::compute(
        &program.chunks[0],
        bsl_bytecode::bundle::module_overlap(0, program.module_vars.len()),
    );
    program
}

/// ГЛАВНЫЙ инвариант формата: печать -> разбор -> печать даёт ту же
/// строку. Побайтово, а не «эквивалентно»: любое расхождение здесь
/// значит, что часть программы потерялась при одном из переходов.
#[test]
fn round_trip_through_text_is_byte_identical() {
    for src in CORPUS {
        let program = compile(src);
        let first = write_program(&program, None).unwrap();
        let reparsed = parse_program(&first)
            .unwrap_or_else(|e| panic!("исходник:\n{src}\nбайт-код:\n{first}\nошибка: {e}"));
        let second = write_program(&reparsed, None).unwrap();
        assert_eq!(first, second, "round-trip разошёлся на:\n{src}");
    }
    let program = call_component_program();
    let first = write_program(&program, None).unwrap();
    let second = write_program(&parse_program(&first).unwrap(), None).unwrap();
    assert_eq!(first, second, "round-trip CallComponent разошёлся");
}

/// Разобранная программа совпадает с исходной по СУЩЕСТВУ, а не только
/// по печати: инструкции, константы, режимы аргументов, обработчики и
/// размеры кадров.
#[test]
fn reparsed_program_matches_the_original_structurally() {
    for src in CORPUS {
        let a = compile(src);
        let b = parse_program(&write_program(&a, None).unwrap()).unwrap();
        assert_eq!(a.names, b.names, "{src}");
        assert_eq!(a.top_level_locals, b.top_level_locals, "{src}");
        assert_eq!(a.shapes.len(), b.shapes.len(), "{src}");
        for (x, y) in a.shapes.iter().zip(&b.shapes) {
            assert_eq!(x.names, y.names, "поля формы разошлись: {src}");
        }
        assert_eq!(a.chunks.len(), b.chunks.len(), "{src}");
        for (x, y) in a.chunks.iter().zip(&b.chunks) {
            assert_eq!(x.instrs, y.instrs, "{src}");
            assert_eq!(x.consts, y.consts, "{src}");
            assert_eq!(x.call_arg_modes, y.call_arg_modes, "{src}");
            assert_eq!(x.exception_ranges, y.exception_ranges, "{src}");
            assert_eq!(
                (x.n_params, x.n_locals, x.n_regs),
                (y.n_params, y.n_locals, y.n_regs)
            );
            assert_eq!(x.local_names, y.local_names, "{src}");
            // Кэш инлайн-кэширования не сохраняется, но обязан быть
            // размером с код — иначе VM промахнётся по индексу.
            assert_eq!(y.prop_cache.len(), y.instrs.len(), "{src}");
            // Разметка бандлов тоже не сохраняется, но пересчёт при
            // разборе обязан дать в точности таблицу компилятора —
            // иначе скомпилированный и загруженный байт-код разойдутся
            // по диспетчеризации.
            assert_eq!(x.bundle_len, y.bundle_len, "{src}");
            // Признак обращения к объектам — тоже производный и тоже
            // пересчитывается разбором. От него зависит, возьмётся ли
            // за чанк нативный путь (см. `LinkedComponents` в
            // `bsl-vm`), и потеря его на разборе вернула бы внешний
            // листинг под JIT молча.
            assert_eq!(x.touches_objects, y.touches_objects, "{src}");
        }
    }
}

/// Конструкторы ядра с внешними возможностями проходят ту же границу, что
/// конструкторы подключаемых библиотек: кодоген не знает ни файловой
/// системы, ни источника случайности, ни отдельных опкодов для этих типов.
#[test]
fn effectful_core_constructors_use_the_component_abi() {
    let program = compile(
        "д = Новый ДвоичныеДанные(\"/dev/null\");\n\
         у = Новый УникальныйИдентификатор;\n\
         ф = Новый UUID(\"abcdef12-3456-7890-abcd-ef1234567890\");",
    );
    let constructors: Vec<(u8, u16, u8)> = program.chunks[0]
        .instrs
        .iter()
        .filter_map(|instruction| match instruction {
            Instr::CreateObject {
                library,
                constructor,
                count,
                ..
            } => Some((*library, *constructor, *count)),
            _ => None,
        })
        .collect();
    assert_eq!(constructors, [(0, 1, 1), (0, 2, 0), (0, 2, 1)]);
}

/// Метод компонентного получателя не может попасть в закрытый `CallMethod`.
/// `Закрыть` остаётся в базовом `BuiltinMethod`, остальные имена принадлежали
/// разным вынесенным пакетам; решение о форме опкода зависит от получателя,
/// а не от общего словаря имён.
#[test]
fn component_receiver_methods_compile_only_to_the_open_opcode() {
    fn construct(
        _context: &mut bsl_rt::CallContext<'_>,
        _arguments: &[bsl_rt::BslValue],
    ) -> bsl_rt::RtResult<bsl_rt::BslValue> {
        Ok(bsl_rt::BslValue::Undefined)
    }
    const CONSTRUCTORS: &[bsl_rt::ConstructorDescriptor] = &[bsl_rt::ConstructorDescriptor {
        code: bsl_rt::ConstructorCode::new(1),
        names: &["Внешний"],
        arity: bsl_rt::Arity::exact(0),
        call: construct,
    }];
    const LIBRARY: bsl_rt::LibraryDescriptor =
        bsl_rt::LibraryDescriptor::new("test-world", "0.0.0", bsl_rt::ObjectContextNeed::Reduced)
            .with_constructors(CONSTRUCTORS);

    let mut builder = bsl_rt::RuntimeBuilder::new();
    builder.register(bsl_rt::core_library()).register(LIBRARY);
    let registry = builder.build().unwrap();
    let src = "о = Новый Внешний;\n\
               о.Закрыть();\n\
               о.ЗаписатьНачалоОбъекта();\n\
               о.ПрочитатьБайт();\n\
               о.ПолучитьТекст();\n\
               о.ТекущаяПозиция();";
    let parsed = bsl_syntax::parse(src).unwrap();
    let resolved = bsl_sema::resolve_program_with_registry(&parsed.items, &registry).unwrap();
    let program = bsl_compiler::compile_program(&resolved).unwrap();

    let mut open = 0;
    for instruction in &program.chunks[0].instrs {
        match instruction {
            Instr::CallObjectMethod { .. } => open += 1,
            Instr::CallMethod { method, .. } => {
                panic!("компонентный метод ушёл в закрытый опкод: {method:?}")
            }
            _ => {}
        }
    }
    assert_eq!(open, 5);
}

#[test]
fn the_corpus_covers_every_opcode() {
    let mut seen: Vec<&str> = Vec::new();
    for src in CORPUS {
        let text = write_program(&compile(src), None).unwrap();
        for op in OPCODES {
            // Опкод — первое слово инструкции; ищем по границе, чтобы
            // `Eq` не «покрылся» строкой `NotEq`.
            if text
                .lines()
                .filter_map(|l| l.split_whitespace().nth(1))
                .any(|word| word == *op)
                && !seen.contains(op)
            {
                seen.push(op);
            }
        }
    }
    let text = write_program(&call_component_program(), None).unwrap();
    for op in OPCODES {
        if text
            .lines()
            .filter_map(|line| line.split_whitespace().nth(1))
            .any(|word| word == *op)
            && !seen.contains(op)
        {
            seen.push(op);
        }
    }
    let missing: Vec<&&str> = OPCODES.iter().filter(|op| !seen.contains(op)).collect();
    assert!(
        missing.is_empty(),
        "корпус не задевает опкоды: {missing:?}. Добавьте на них исходник — \
         иначе печать и разбор для них ничем не проверены."
    );
}
