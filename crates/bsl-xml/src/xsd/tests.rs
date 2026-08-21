//! Тесты модели схемы и её поверхности.

use super::*;

/// Схема из текста XSD — тем же путём, что и в бою: дерево строит
/// `dom`, а разбирает уже готовое дерево этот модуль. В отличие от
/// [`schema_of_text`], отдаёт результат `СоздатьСхемуXML` как есть,
/// включая `Неопределено` на корне, который схемой не является.
fn schema_of(text: &str) -> RtResult<BslValue> {
    let mut state = crate::xml::XmlReaderState::over(crate::core::XmlParser::new(text));
    let doc = crate::dom::build_tree(&mut state).expect("дерево обязано строиться");
    let value = crate::dom::node_value(&doc, &doc);
    create_schema(&new_builder(), &[value])
}

fn schema(text: &str) -> BslValue {
    schema_of(text).expect("схема обязана разбираться")
}

fn prop(obj: &BslValue, name: &str) -> BslValue {
    get_property(obj, name).unwrap_or_else(|e| panic!("свойство «{name}»: {e}"))
}

fn text_of(v: &BslValue) -> String {
    match v {
        BslValue::Str(s) => s.to_string(),
        other => panic!("ожидалась строка, получено {other:?}"),
    }
}

fn count(v: &BslValue) -> usize {
    v.collection_len().expect("коллекция обязана иметь длину")
}

fn item(v: &BslValue, i: usize) -> BslValue {
    let list = v
        .object_ref()
        .and_then(|object| object.downcast_ref::<SchemaListObject>())
        .expect("ожидалась коллекция компонент");
    list_get(&list.schema, &list.kind, i).expect("элемент обязан быть")
}

/// Схема с двумя глобальными объявлениями, простым и составным типом —
/// её хватает почти всем проверкам ниже.
const SAMPLE: &str = concat!(
    r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:t="urn:t" "#,
    r#"targetNamespace="urn:t" elementFormDefault="qualified" version="1.7">"#,
    r#"<xs:element name="корень" type="t:Тип"/>"#,
    r#"<xs:element name="Апельсин" type="xs:string" default="да"/>"#,
    r#"<xs:attribute name="глоб" type="xs:int"/>"#,
    r#"<xs:simpleType name="Код"><xs:restriction base="xs:string">"#,
    r#"<xs:minLength value="2"/><xs:maxLength value="5" fixed="true"/>"#,
    r#"<xs:pattern value="[А-Я]+"/></xs:restriction></xs:simpleType>"#,
    r#"<xs:complexType name="Тип"><xs:sequence>"#,
    r#"<xs:element name="имя" type="xs:string"/>"#,
    r#"<xs:element name="вложенный"><xs:complexType><xs:sequence>"#,
    r#"<xs:element name="внутри" type="xs:int"/>"#,
    r#"</xs:sequence></xs:complexType></xs:element>"#,
    r#"<xs:choice><xs:element name="а" type="xs:string"/></xs:choice>"#,
    r#"</xs:sequence>"#,
    r#"<xs:attribute name="ид" type="xs:int" use="required"/>"#,
    r#"</xs:complexType></xs:schema>"#,
);

#[test]
fn schema_reports_its_namespace_version_and_forms() {
    let s = schema(SAMPLE);
    assert_eq!(text_of(&prop(&s, "ПространствоИмен")), "urn:t");
    assert_eq!(text_of(&prop(&s, "Версия")), "1.7");
    assert_eq!(
        prop(&s, "ФормаЭлементовПоУмолчанию"),
        BslValue::Enum(EnumValue::XsFormQualified)
    );
    // Неуказанное — `Неопределено`, а не значение по умолчанию.
    assert_eq!(prop(&s, "ФормаАтрибутовПоУмолчанию"), BslValue::Undefined);
    assert_eq!(prop(&s, "БлокировкаПоУмолчанию"), BslValue::Undefined);
    assert_eq!(prop(&s, "Контейнер"), BslValue::Undefined);
    assert_eq!(prop(&s, "Схема"), s, "схема — сама себе схема");
    assert_eq!(
        prop(&s, "ТипКомпоненты"),
        BslValue::Enum(EnumValue::XsCompSchema)
    );
}

/// Именованные коллекции ОТСОРТИРОВАНЫ по имени, а `Компоненты` идут в
/// порядке документа — измерено на платформе, и это разный порядок.
#[test]
fn named_collections_are_sorted_while_components_follow_the_document() {
    let s = schema(SAMPLE);
    let elements = prop(&s, "ОбъявленияЭлементов");
    assert_eq!(count(&elements), 2);
    assert_eq!(text_of(&prop(&item(&elements, 0), "Имя")), "Апельсин");
    assert_eq!(text_of(&prop(&item(&elements, 1), "Имя")), "корень");

    let components = prop(&s, "Компоненты");
    assert_eq!(count(&components), 5);
    assert_eq!(text_of(&prop(&item(&components, 0), "Имя")), "корень");
    assert_eq!(text_of(&prop(&item(&components, 1), "Имя")), "Апельсин");
}

#[test]
fn named_lookup_takes_a_name_a_pair_and_an_expanded_name() {
    let s = schema(SAMPLE);
    let elements = prop(&s, "ОбъявленияЭлементов");
    let by_name = list_lookup(&elements, &[BslValue::Str(BslString::from_str("корень"))])
        .expect("поиск по имени");
    assert_eq!(text_of(&prop(&by_name, "Имя")), "корень");
    let by_pair = list_lookup(
        &elements,
        &[
            BslValue::Str(BslString::from_str("urn:t")),
            BslValue::Str(BslString::from_str("корень")),
        ],
    )
    .expect("поиск по паре");
    assert_eq!(by_pair, by_name, "компоненты сравниваются по тождеству");
    let by_expanded =
        list_lookup(&elements, &[new_expanded_name("urn:t", "корень")]).expect("поиск по имени");
    assert_eq!(by_expanded, by_name);
    // Чужой URI и неизвестное имя — `Неопределено`, а не ошибка.
    assert_eq!(
        list_lookup(
            &elements,
            &[
                BslValue::Str(BslString::from_str("urn:нет")),
                BslValue::Str(BslString::from_str("корень")),
            ],
        )
        .expect("чужой URI"),
        BslValue::Undefined
    );
    assert_eq!(
        list_lookup(&elements, &[BslValue::Str(BslString::from_str("нет"))]).expect("нет имени"),
        BslValue::Undefined
    );
}

#[test]
fn element_declaration_keeps_the_lexical_type_name_and_default() {
    let s = schema(SAMPLE);
    let elements = prop(&s, "ОбъявленияЭлементов");
    let root = list_lookup(&elements, &[BslValue::Str(BslString::from_str("корень"))]).unwrap();
    let type_name = prop(&root, "ИмяТипа");
    assert_eq!(text_of(&prop(&type_name, "ЛокальноеИмя")), "Тип");
    assert_eq!(text_of(&prop(&type_name, "URIПространстваИмен")), "urn:t");
    assert_eq!(text_of(&prop(&root, "URIПространстваИмен")), "urn:t");
    assert_eq!(
        prop(&root, "ЭтоГлобальноеОбъявление"),
        BslValue::Boolean(true)
    );
    assert_eq!(prop(&root, "АнонимноеОпределениеТипа"), BslValue::Undefined);
    // Без `default`/`fixed`: лексическое значение пусто, а
    // типизированное — `Неопределено` (измерено).
    assert_eq!(text_of(&prop(&root, "ЛексическоеЗначение")), "");
    assert_eq!(prop(&root, "Ограничение"), BslValue::Undefined);
    assert_eq!(prop(&root, "Значение"), BslValue::Undefined);

    let plain = list_lookup(&elements, &[BslValue::Str(BslString::from_str("Апельсин"))]).unwrap();
    assert_eq!(text_of(&prop(&plain, "ЛексическоеЗначение")), "да");
    assert_eq!(
        prop(&plain, "Ограничение"),
        BslValue::Enum(EnumValue::XsConstraintDefault)
    );
    assert_eq!(text_of(&prop(&plain, "Значение")), "да");
}

/// Границы вхождения частицы. Каждое ожидание здесь пришпилено к своей
/// строке `measure-xsd.platform.txt`: `час0 …` (`minOccurs="0"
/// maxOccurs="unbounded"` -> `Число [0]` и `Строка [unbounded]`),
/// `выбор0 …` (написан только `minOccurs`, второй конец пуст), `границы
/// числом` (2 и 5), `границы мусором` (`много` и пустая строка — обе
/// пусты), `границы наоборот` (`unbounded` в `minOccurs` -> `Число [4
/// 294 967 295]`, `-1` -> оно же) и `границы через край` (`4294967296`
/// -> 0, двадцать девяток -> 1 661 992 959, `007` -> 7, `" 5 "` -> 5,
/// `+3` -> 3).
#[test]
fn particle_occurs_bounds_repeat_what_is_written() {
    let s = schema(concat!(
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">"#,
        r#"<xs:complexType name="т"><xs:sequence>"#,
        r#"<xs:element name="а" type="xs:string" minOccurs="0" maxOccurs="unbounded"/>"#,
        r#"<xs:element name="б" type="xs:string" minOccurs="2" maxOccurs="5"/>"#,
        r#"<xs:element name="в" type="xs:string" minOccurs="много" maxOccurs=""/>"#,
        r#"<xs:element name="г" type="xs:string" minOccurs="unbounded"/>"#,
        r#"<xs:element name="д" type="xs:string" minOccurs="-1" maxOccurs="0"/>"#,
        r#"<xs:element name="е" type="xs:string" minOccurs="4294967296""#,
        r#" maxOccurs="99999999999999999999"/>"#,
        r#"<xs:element name="ж" type="xs:string" minOccurs="007" maxOccurs=" 5 "/>"#,
        r#"<xs:element name="з" type="xs:string" minOccurs="+3"/>"#,
        r#"<xs:choice minOccurs="0"><xs:element name="и" type="xs:string"/></xs:choice>"#,
        r#"</xs:sequence></xs:complexType></xs:schema>"#,
    ));
    let content = prop(&item(&prop(&s, "ОпределенияТипов"), 0), "Содержимое");
    // Сам `<xs:sequence>` написан без атрибутов — обе границы пусты.
    assert_eq!(prop(&content, "МинимальноВходит"), BslValue::Undefined);
    assert_eq!(prop(&content, "МаксимальноВходит"), BslValue::Undefined);

    let particles = prop(&prop(&content, "Часть"), "Фрагменты");
    let num = |n: i64| BslValue::Number(BslNumber::from_i64(n));
    let bounds = |i: usize| {
        let p = item(&particles, i);
        (prop(&p, "МинимальноВходит"), prop(&p, "МаксимальноВходит"))
    };
    // `unbounded` в `maxOccurs` — СТРОКА, а не число.
    assert_eq!(bounds(0), (num(0), str_value("unbounded")));
    assert_eq!(bounds(1), (num(2), num(5)));
    // Нечисло границы не задаёт: и слово, и пустая строка дают пусто.
    assert_eq!(bounds(2), (BslValue::Undefined, BslValue::Undefined));
    // Тот же `unbounded` со стороны `minOccurs` виден числом: внутри
    // это `u32::MAX`, а строкой его показывает только `МаксимальноВходит`.
    assert_eq!(bounds(3), (num(4_294_967_295), BslValue::Undefined));
    assert_eq!(bounds(4), (num(4_294_967_295), num(0)));
    // Разбор по модулю 2^32: 2^32 -> 0, а 10^20 - 1 -> 1 661 992 959.
    assert_eq!(bounds(5), (num(0), num(1_661_992_959)));
    assert_eq!(bounds(6), (num(7), num(5)));
    assert_eq!(bounds(7), (num(3), BslValue::Undefined));
    // Границы есть и у частицы, чей терм — вложенная группа модели.
    assert_eq!(bounds(8), (num(0), BslValue::Undefined));
}

/// `Значение` угадывается по САМОЙ записи, а не по объявленному типу:
/// булев литерал даёт `Булево` даже под `xs:string`, числовой префикс —
/// `Число` даже под `xs:date`, а запись без цифр остаётся строкой.
/// Все проверяемые здесь пары измерены строкой `XSD.TYPED_VALUE`.
#[test]
fn typed_value_is_read_from_the_lexical_form_not_the_declared_type() {
    let s = schema(concat!(
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">"#,
        r#"<xs:element name="ч" type="xs:int" default="5"/>"#,
        r#"<xs:element name="л" type="xs:boolean" default="true"/>"#,
        r#"<xs:element name="д" type="xs:date" default="2026-08-12"/>"#,
        r#"<xs:element name="вр" type="xs:time" default="08:30:00"/>"#,
        r#"<xs:element name="эксп" type="xs:double" default="1.5e3"/>"#,
        r#"<xs:element name="шест" type="xs:hexBinary" default="0A0B"/>"#,
        r#"<xs:element name="дво" type="xs:base64Binary" default="AQI="/>"#,
        r#"<xs:element name="урл" type="xs:anyURI" default="urn:x"/>"#,
        r#"<xs:element name="стрч" type="xs:string" default="7"/>"#,
        r#"<xs:element name="стрл" type="xs:string" default="True"/>"#,
        r#"<xs:element name="ноль" type="xs:int" default="0"/>"#,
        r#"<xs:element name="един" type="xs:string" default="1"/>"#,
        r#"<xs:element name="плюс" type="xs:int" default="+3"/>"#,
        r#"<xs:element name="точка" type="xs:decimal" default=".5"/>"#,
        r#"<xs:element name="дветочки" type="xs:string" default="1.2.3"/>"#,
        r#"<xs:element name="пробелы" type="xs:string" default=" 7 "/>"#,
        r#"<xs:element name="запятая" type="xs:string" default="1,5"/>"#,
        r#"<xs:element name="без" type="xs:int"/>"#,
        r#"</xs:schema>"#,
    ));
    let elements = prop(&s, "ОбъявленияЭлементов");
    let get = |name: &str| {
        let d = list_lookup(&elements, &[BslValue::Str(BslString::from_str(name))]).unwrap();
        prop(&d, "Значение")
    };
    let num = |s: &str| BslValue::Number(BslNumber::parse_canonical(s).unwrap());
    assert_eq!(get("ч"), num("5"));
    assert_eq!(get("л"), BslValue::Boolean(true));
    // Дата, время и показатель степени числами не остаются целиком:
    // берётся числовой префикс записи.
    assert_eq!(get("д"), num("2026"));
    assert_eq!(get("вр"), num("8"));
    assert_eq!(get("эксп"), num("1.5"));
    assert_eq!(get("шест"), num("0"));
    // Записи без цифр в префиксе остаются строками.
    assert_eq!(text_of(&get("дво")), "AQI=");
    assert_eq!(text_of(&get("урл")), "urn:x");
    // Объявленный тип не участвует ни в одну сторону.
    assert_eq!(get("стрч"), num("7"));
    assert_eq!(get("стрл"), BslValue::Boolean(true));
    assert_eq!(get("ноль"), BslValue::Boolean(false));
    assert_eq!(get("един"), BslValue::Boolean(true));
    assert_eq!(get("плюс"), num("3"));
    assert_eq!(get("точка"), num("0.5"));
    assert_eq!(get("дветочки"), num("1.2"));
    assert_eq!(get("пробелы"), num("7"));
    assert_eq!(get("запятая"), num("1"));
    // Без `default`/`fixed` значения нет вовсе.
    assert_eq!(get("без"), BslValue::Undefined);
}

#[test]
fn simple_type_exposes_base_name_variety_and_facets() {
    let s = schema(SAMPLE);
    let types = prop(&s, "ОпределенияТипов");
    let code = list_lookup(&types, &[BslValue::Str(BslString::from_str("Код"))]).unwrap();
    let base = prop(&code, "ИмяБазовогоТипа");
    assert_eq!(text_of(&prop(&base, "ЛокальноеИмя")), "string");
    assert_eq!(text_of(&prop(&base, "URIПространстваИмен")), XSD_NS);
    // Разрешённое определение платформа на этом этапе не заполняет.
    assert_eq!(prop(&code, "ОпределениеБазовогоТипа"), BslValue::Undefined);
    assert_eq!(
        prop(&code, "Вариант"),
        BslValue::Enum(EnumValue::XsVarietyAtomic)
    );
    let facets = prop(&code, "Фасеты");
    assert_eq!(count(&facets), 3);
    let min = item(&facets, 0);
    assert_eq!(
        prop(&min, "ТипКомпоненты"),
        BslValue::Enum(EnumValue::XsCompMinLengthFacet)
    );
    assert_eq!(text_of(&prop(&min, "ЛексическоеЗначение")), "2");
    assert_eq!(prop(&min, "Фиксированный"), BslValue::Undefined);
    assert_eq!(
        prop(&item(&facets, 1), "Фиксированный"),
        BslValue::Boolean(true)
    );
    // Фасет — ребёнок своего типа, а схема у него общая.
    assert_eq!(prop(&min, "Контейнер"), code);
    assert_eq!(prop(&min, "Схема"), s);
}

/// У образца, перечисления и пробельных символов `Фиксированный` НЕТ —
/// обращение к нему ошибка, а не `Неопределено` (измерено).
#[test]
fn only_numeric_facets_have_the_fixed_flag() {
    let s = schema(SAMPLE);
    let types = prop(&s, "ОпределенияТипов");
    let code = list_lookup(&types, &[BslValue::Str(BslString::from_str("Код"))]).unwrap();
    let pattern = item(&prop(&code, "Фасеты"), 2);
    assert_eq!(text_of(&prop(&pattern, "ЛексическоеЗначение")), "[А-Я]+");
    // Образец хранится СЫРОЙ строкой: движка регулярных выражений здесь
    // нет, и проверка по нему — работа отдельной задачи.
    assert_eq!(text_of(&prop(&pattern, "Значение")), "[А-Я]+");
    assert!(
        matches!(
            get_property(&pattern, "Фиксированный"),
            Err(RtError::UnknownColumn(_))
        ),
        "у фасета образца `Фиксированный` быть не должно"
    );
}

#[test]
fn complex_type_exposes_content_particle_and_attribute_uses() {
    let s = schema(SAMPLE);
    let types = prop(&s, "ОпределенияТипов");
    let t = list_lookup(&types, &[BslValue::Str(BslString::from_str("Тип"))]).unwrap();
    assert_eq!(prop(&t, "ИмяБазовогоТипа"), BslValue::Undefined);
    assert_eq!(prop(&t, "МетодНаследования"), BslValue::Undefined);
    assert_eq!(prop(&t, "МодельСодержимого"), BslValue::Undefined);
    assert_eq!(prop(&t, "Смешанный"), BslValue::Undefined);

    let particle = prop(&t, "Содержимое");
    assert_eq!(
        prop(&particle, "ТипКомпоненты"),
        BslValue::Enum(EnumValue::XsCompParticle)
    );
    // У этого `<xs:sequence>` границы вхождения не написаны, поэтому
    // обе пусты (измерено, проба `час МинимальноВходит`). Что бывает
    // там, где они написаны, проверяет
    // `particle_occurs_bounds_repeat_what_is_written`.
    assert_eq!(prop(&particle, "МинимальноВходит"), BslValue::Undefined);
    assert_eq!(prop(&particle, "МаксимальноВходит"), BslValue::Undefined);
    assert_eq!(prop(&particle, "Контейнер"), t);

    let group = prop(&particle, "Часть");
    assert_eq!(
        prop(&group, "ВидГруппы"),
        BslValue::Enum(EnumValue::XsGroupSequence)
    );
    let particles = prop(&group, "Фрагменты");
    assert_eq!(count(&particles), 3);
    // Вложенный `xs:choice` — это фрагмент, чей терм снова группа.
    let nested = prop(&item(&particles, 2), "Часть");
    assert_eq!(
        prop(&nested, "ВидГруппы"),
        BslValue::Enum(EnumValue::XsGroupChoice)
    );

    let attributes = prop(&t, "Атрибуты");
    assert_eq!(count(&attributes), 1);
    let use_ = item(&attributes, 0);
    assert_eq!(prop(&use_, "Обязательный"), BslValue::Boolean(true));
    // `Использование` платформа не заполняет — обязательность отдаёт
    // `Обязательный`.
    assert_eq!(prop(&use_, "Использование"), BslValue::Undefined);
    let decl = prop(&use_, "ОбъявлениеАтрибута");
    assert_eq!(text_of(&prop(&decl, "Имя")), "ид");
    assert_eq!(prop(&decl, "Контейнер"), use_);
}

/// Анонимный тип живёт у объявления, а НЕ в `ОпределенияТипов`.
#[test]
fn anonymous_type_belongs_to_its_declaration() {
    let s = schema(SAMPLE);
    let types = prop(&s, "ОпределенияТипов");
    assert_eq!(count(&types), 2, "именованных типов ровно два");
    let t = list_lookup(&types, &[BslValue::Str(BslString::from_str("Тип"))]).unwrap();
    let group = prop(&prop(&t, "Содержимое"), "Часть");
    let nested_decl = prop(&item(&prop(&group, "Фрагменты"), 1), "Часть");
    assert_eq!(text_of(&prop(&nested_decl, "Имя")), "вложенный");
    assert_eq!(
        prop(&nested_decl, "ЭтоГлобальноеОбъявление"),
        BslValue::Boolean(false)
    );
    let anon = prop(&nested_decl, "АнонимноеОпределениеТипа");
    assert_eq!(
        text_of(&prop(&anon, "Имя")),
        "",
        "у анонимного типа имени нет"
    );
    assert_eq!(prop(&anon, "Контейнер"), nested_decl);
    assert_eq!(count(&prop(&nested_decl, "Компоненты")), 1);
    // Локальное объявление под `elementFormDefault="qualified"` берёт
    // целевое пространство имён, а `Форма` остаётся пустой (измерено).
    assert_eq!(text_of(&prop(&nested_decl, "URIПространстваИмен")), "urn:t");
    assert_eq!(prop(&nested_decl, "Форма"), BslValue::Undefined);
}

#[test]
fn inheritance_keeps_the_lexical_base_name_and_method() {
    let s = schema(concat!(
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:t="urn:t" "#,
        r#"targetNamespace="urn:t">"#,
        r#"<xs:complexType name="База"><xs:sequence>"#,
        r#"<xs:element name="а" type="xs:string"/></xs:sequence></xs:complexType>"#,
        r#"<xs:complexType name="Расширение"><xs:complexContent>"#,
        r#"<xs:extension base="t:База"><xs:sequence>"#,
        r#"<xs:element name="б" type="xs:string"/></xs:sequence>"#,
        r#"</xs:extension></xs:complexContent></xs:complexType>"#,
        r#"<xs:complexType name="Сужение"><xs:complexContent>"#,
        r#"<xs:restriction base="t:База"><xs:sequence>"#,
        r#"<xs:element name="а" type="xs:string"/></xs:sequence>"#,
        r#"</xs:restriction></xs:complexContent></xs:complexType>"#,
        r#"<xs:complexType name="Простое"><xs:simpleContent>"#,
        r#"<xs:extension base="xs:string">"#,
        r#"<xs:attribute name="у" type="xs:string"/>"#,
        r#"</xs:extension></xs:simpleContent></xs:complexType>"#,
        r#"</xs:schema>"#,
    ));
    let types = prop(&s, "ОпределенияТипов");
    let by = |name: &str| list_lookup(&types, &[BslValue::Str(BslString::from_str(name))]).unwrap();

    let ext = by("Расширение");
    assert_eq!(
        text_of(&prop(&prop(&ext, "ИмяБазовогоТипа"), "ЛокальноеИмя")),
        "База"
    );
    assert_eq!(
        prop(&ext, "МетодНаследования"),
        BslValue::Enum(EnumValue::XsDerivationExtension)
    );
    // Содержимое наследника — ТОЛЬКО его собственное: базовое платформа
    // сюда не вливает.
    let group = prop(&prop(&ext, "Содержимое"), "Часть");
    assert_eq!(count(&prop(&group, "Фрагменты")), 1);

    assert_eq!(
        prop(&by("Сужение"), "МетодНаследования"),
        BslValue::Enum(EnumValue::XsDerivationRestriction)
    );
    let simple = by("Простое");
    assert_eq!(
        text_of(&prop(&prop(&simple, "ИмяБазовогоТипа"), "ЛокальноеИмя")),
        "string"
    );
    assert_eq!(prop(&simple, "Содержимое"), BslValue::Undefined);
    assert_eq!(count(&prop(&simple, "Атрибуты")), 1);
}

#[test]
fn list_and_union_keep_their_lexical_members() {
    let s = schema(concat!(
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:t="urn:t" "#,
        r#"targetNamespace="urn:t">"#,
        r#"<xs:simpleType name="Код"><xs:restriction base="xs:string"/></xs:simpleType>"#,
        r#"<xs:simpleType name="Коды"><xs:list itemType="t:Код"/></xs:simpleType>"#,
        r#"<xs:simpleType name="Оба"><xs:union memberTypes="xs:string xs:int"/></xs:simpleType>"#,
        r#"</xs:schema>"#,
    ));
    let types = prop(&s, "ОпределенияТипов");
    let by = |name: &str| list_lookup(&types, &[BslValue::Str(BslString::from_str(name))]).unwrap();

    let list = by("Коды");
    assert_eq!(
        prop(&list, "Вариант"),
        BslValue::Enum(EnumValue::XsVarietyList)
    );
    assert_eq!(
        text_of(&prop(&prop(&list, "ИмяТипаЭлемента"), "ЛокальноеИмя")),
        "Код"
    );

    let union = by("Оба");
    assert_eq!(
        prop(&union, "Вариант"),
        BslValue::Enum(EnumValue::XsVarietyUnion)
    );
    let members = prop(&union, "ИменаТиповОбъединения");
    assert_eq!(count(&members), 2);
    // Разрешённых определений платформа не заполняет — список пуст.
    assert_eq!(count(&prop(&union, "ОпределенияТиповОбъединения")), 0);
}

/// `ref` — это ОТДЕЛЬНОЕ объявление с пустым именем: тождества с
/// глобальным у него нет (измерено).
#[test]
fn references_are_separate_declarations() {
    let s = schema(concat!(
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:t="urn:t" "#,
        r#"targetNamespace="urn:t">"#,
        r#"<xs:element name="а" type="xs:string"/>"#,
        r#"<xs:attribute name="б" type="xs:string"/>"#,
        r#"<xs:complexType name="Т"><xs:sequence>"#,
        r#"<xs:element ref="t:а"/></xs:sequence>"#,
        r#"<xs:attribute ref="t:б"/></xs:complexType>"#,
        r#"</xs:schema>"#,
    ));
    let types = prop(&s, "ОпределенияТипов");
    let t = list_lookup(&types, &[BslValue::Str(BslString::from_str("Т"))]).unwrap();
    let group = prop(&prop(&t, "Содержимое"), "Часть");
    let by_ref = prop(&item(&prop(&group, "Фрагменты"), 0), "Часть");
    assert_eq!(text_of(&prop(&by_ref, "Имя")), "");
    assert_eq!(prop(&by_ref, "ИмяТипа"), BslValue::Undefined);
    assert_eq!(
        prop(&by_ref, "ЭтоГлобальноеОбъявление"),
        BslValue::Boolean(false)
    );
    assert_eq!(
        text_of(&prop(&prop(&by_ref, "Ссылка"), "ЛокальноеИмя")),
        "а"
    );
    let global = list_lookup(
        &prop(&s, "ОбъявленияЭлементов"),
        &[BslValue::Str(BslString::from_str("а"))],
    )
    .unwrap();
    assert_ne!(by_ref, global, "ссылка не тождественна объявлению");

    let attr_decl = prop(&item(&prop(&t, "Атрибуты"), 0), "ОбъявлениеАтрибута");
    assert_eq!(text_of(&prop(&attr_decl, "Имя")), "");
    assert_eq!(
        text_of(&prop(&prop(&attr_decl, "Ссылка"), "ЛокальноеИмя")),
        "б"
    );
}

/// QName разрешается по объявлениям `xmlns` в области видимости, а
/// неразрешимое имя даёт `Неопределено` — не «имя в пустом
/// пространстве».
#[test]
fn unresolvable_qnames_become_undefined() {
    let s = schema(concat!(
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">"#,
        r#"<xs:element name="а" type="нетТакого"/>"#,
        r#"<xs:element name="б" type="нет:такого"/>"#,
        r#"</xs:schema>"#,
    ));
    let elements = prop(&s, "ОбъявленияЭлементов");
    for name in ["а", "б"] {
        let d = list_lookup(&elements, &[BslValue::Str(BslString::from_str(name))]).unwrap();
        assert_eq!(prop(&d, "ИмяТипа"), BslValue::Undefined);
    }
    // Префикс не важен: схему опознают по URI пространства имён.
    let other_prefix =
        schema(r#"<xs:schema xmlns:xs="urn:чужое"><xs:element name="а"/></xs:schema>"#);
    assert_eq!(
        other_prefix,
        BslValue::Undefined,
        "корень не из пространства XML Schema схемой не считается"
    );
}

#[test]
fn broken_and_foreign_input_follows_the_platform() {
    // Корень не схема — `Неопределено`, а не ошибка.
    assert_eq!(schema("<а><б/></а>"), BslValue::Undefined);
    // Документ без корня — ошибка.
    let empty = create_schema(&new_builder(), &[crate::dom::new_document()]);
    assert!(matches!(empty, Err(RtError::Xsd(_))));
    // Не узел дерева и не один аргумент — тоже.
    assert!(matches!(
        create_schema(&new_builder(), &[BslValue::Undefined]),
        Err(RtError::Xsd(_))
    ));
    assert!(matches!(
        create_schema(&new_builder(), &[]),
        Err(RtError::MethodNotApplicable { .. })
    ));
    // Незнакомый элемент схемы и чужое пространство имён — пропускаются.
    let s = schema(concat!(
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:ч="urn:ч">"#,
        r#"<xs:чушь name="а"/><ч:своё/>текст<!--к-->"#,
        r#"<xs:element name="б" type="xs:string"/>"#,
        r#"<xs:element type="xs:string"/>"#,
        r#"<xs:element name="б" type="xs:int"/>"#,
        r#"</xs:schema>"#,
    ));
    let elements = prop(&s, "ОбъявленияЭлементов");
    assert_eq!(
        count(&elements),
        1,
        "дубль имени и элемент без имени выпали"
    );
    // Из двух одноимённых остаётся ПЕРВЫЙ.
    let kept = item(&elements, 0);
    assert_eq!(
        text_of(&prop(&prop(&kept, "ИмяТипа"), "ЛокальноеИмя")),
        "string"
    );
    assert_eq!(count(&prop(&s, "Компоненты")), 2, "оба дубля — компоненты");
}

/// Конструкции за границей модели — честная ошибка с именем
/// конструкции, а не молчаливый пропуск.
#[test]
fn skipped_constructs_load_without_component_nodes() {
    // `xs:import` и маски `xs:any`/`xs:anyAttribute` пропускаются без
    // узла в дереве — сознательное расхождение с платформой, у которой
    // они остаются компонентами (`XSD.IMPORT.COMPONENT`,
    // `XSD.WILDCARD.COMPONENT`: «компонент 2» там против «компонент 1»
    // здесь). Схема при этом обязана загружаться: на этом стоит разбор
    // EnterpriseData_1_0_1.xsd в `benchmarks/edata_writer.bsl`.
    let cases = [
        r#"<xs:import namespace="urn:ч"/><xs:element name="э" type="xs:string"/>"#,
        r#"<xs:complexType name="т"><xs:sequence><xs:element name="э" type="xs:string"/><xs:any/></xs:sequence></xs:complexType>"#,
        r#"<xs:complexType name="т"><xs:sequence><xs:element name="э" type="xs:string"/></xs:sequence><xs:anyAttribute/></xs:complexType>"#,
    ];
    for body in cases {
        let text = format!(
            r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:t="urn:t">{body}</xs:schema>"#
        );
        let schema = schema_of(&text)
            .unwrap_or_else(|e| panic!("схема с пропускаемой конструкцией не загрузилась: {e}"));
        // Пропущенная конструкция не оставляет узла: единственная
        // компонента схемы — объявление элемента либо составной тип.
        let components = prop(&schema, "Компоненты");
        assert_eq!(
            count(&components),
            1,
            "лишний узел от пропущенной конструкции"
        );
    }
}

#[test]
fn constructs_outside_the_model_are_named_errors() {
    let cases = [
        (
            r#"<xs:group name="г"><xs:sequence/></xs:group>"#,
            "xs:group",
        ),
        (r#"<xs:attributeGroup name="г"/>"#, "xs:attributeGroup"),
        (r#"<xs:include schemaLocation="а.xsd"/>"#, "xs:include"),
        (r#"<xs:redefine schemaLocation="а.xsd"/>"#, "xs:redefine"),
        (r#"<xs:notation name="н" public="п"/>"#, "xs:notation"),
        (
            r#"<xs:complexType name="т"><xs:sequence><xs:group ref="t:г"/></xs:sequence></xs:complexType>"#,
            "xs:group",
        ),
        (
            r#"<xs:element name="а"><xs:unique name="у"><xs:selector xpath="."/></xs:unique></xs:element>"#,
            "xs:unique",
        ),
    ];
    for (body, expected) in cases {
        let text = format!(
            r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:t="urn:t">{body}</xs:schema>"#
        );
        match schema_of(&text) {
            Err(RtError::Xsd(msg)) => assert!(
                msg.contains(expected),
                "в тексте ошибки нет «{expected}»: {msg}"
            ),
            other => panic!("ожидалась ошибка про «{expected}», получено {other:?}"),
        }
    }
}

#[test]
fn empty_schema_has_no_components_and_no_dom() {
    let s = new_schema();
    assert_eq!(text_of(&prop(&s, "ПространствоИмен")), "");
    assert_eq!(text_of(&prop(&s, "Версия")), "");
    assert_eq!(count(&prop(&s, "Компоненты")), 0);
    assert_eq!(count(&prop(&s, "ОбъявленияЭлементов")), 0);
    assert_eq!(prop(&s, "ЭлементDOM"), BslValue::Undefined);
    assert_eq!(prop(&s, "Схема"), s);
}

/// Набор схем: та же схема повторно проходит молча, ДРУГАЯ схема того
/// же пространства имён — ошибка (измерено обе ветки).
#[test]
fn schema_set_holds_one_schema_per_namespace() {
    let set = new_schema_set();
    let first = schema(SAMPLE);
    schema_set_add(&set, std::slice::from_ref(&first)).expect("первая схема");
    schema_set_add(&set, std::slice::from_ref(&first)).expect("та же схема повторно");
    assert_eq!(set.collection_len().unwrap(), 1);
    assert_eq!(schema_set_get(&set, 0).unwrap(), first);

    let same_ns = schema(SAMPLE);
    assert!(
        matches!(schema_set_add(&set, &[same_ns]), Err(RtError::Xsd(_))),
        "вторая схема того же пространства имён — ошибка"
    );
    let other = schema(concat!(
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:другое">"#,
        r#"<xs:element name="щ" type="xs:string"/></xs:schema>"#,
    ));
    schema_set_add(&set, &[other]).expect("другое пространство имён");
    assert_eq!(set.collection_len().unwrap(), 2);
    assert!(matches!(
        schema_set_add(&set, &[BslValue::Undefined]),
        Err(RtError::Xsd(_))
    ));
    assert!(matches!(
        schema_set_get(&set, 9),
        Err(RtError::IndexOutOfBounds { .. })
    ));
}

#[test]
fn expanded_name_is_a_value_not_a_reference() {
    let a = new_expanded_name("urn:t", "а");
    let b = new_expanded_name("urn:t", "а");
    assert_eq!(a, b, "равенство по содержимому");
    assert_ne!(a, new_expanded_name("urn:t", "б"));
    assert_eq!(a.to_string(), "{urn:t}а");
    // Пустой URI печатается ОДНИМ локальным именем, без скобок.
    assert_eq!(new_expanded_name("", "а").to_string(), "а");
}

/// Аннотация — обычная компонента: у неё есть документация и
/// информация приложения, а имени нет.
#[test]
fn annotation_carries_documentation_and_appinfo() {
    let s = schema(concat!(
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">"#,
        r#"<xs:annotation><xs:documentation xml:lang="ru">про схему</xs:documentation>"#,
        r#"<xs:appinfo source="urn:и">данные</xs:appinfo></xs:annotation>"#,
        r#"</xs:schema>"#,
    ));
    let annotation = prop(&s, "Аннотация");
    assert_eq!(
        prop(&annotation, "ТипКомпоненты"),
        BslValue::Enum(EnumValue::XsCompAnnotation)
    );
    assert_eq!(count(&prop(&annotation, "Компоненты")), 2);
    let doc = item(&prop(&annotation, "Документация"), 0);
    assert_eq!(text_of(&prop(&doc, "Язык")), "ru");
    assert_eq!(text_of(&prop(&doc, "Источник")), "");
    assert_eq!(prop(&doc, "Контейнер"), annotation);
    let appinfo = item(&prop(&annotation, "ИнформацияДляПриложения"), 0);
    assert_eq!(text_of(&prop(&appinfo, "Источник")), "urn:и");
    assert_eq!(
        prop(&appinfo, "ТипКомпоненты"),
        BslValue::Enum(EnumValue::XsCompAppInfo)
    );
    // Имени у аннотации нет вовсе.
    assert!(matches!(
        get_property(&annotation, "Имя"),
        Err(RtError::UnknownColumn(_))
    ));
}

#[test]
fn indexing_past_the_end_is_an_error() {
    let s = schema(SAMPLE);
    let elements = prop(&s, "ОбъявленияЭлементов");
    let list = elements
        .object_ref()
        .and_then(|object| object.downcast_ref::<SchemaListObject>())
        .expect("ожидалась коллекция");
    let (schema_rc, kind) = (list.schema.clone(), list.kind.clone());
    assert!(matches!(
        list_get(&schema_rc, &kind, 9),
        Err(RtError::IndexOutOfBounds { .. })
    ));
}
