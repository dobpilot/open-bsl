//! Корпус тестов модели XDTO: все проверки идут над общими фикстурами
//! (`SAMPLE` и помощники `model`/`type_of`/`prop`), поэтому живут вместе,
//! а не по одному при каждом файле — иначе харнесс пришлось бы копировать
//! в каждый из них.

use bsl_rt::TypeRef;

use super::*;

/// Зона тестов XDTO — неподвижный UTC+3: лексические формы с явным поясом
/// пересчитываются в местное время, и привязывать ожидания к зоне машины
/// значило бы получать разный результат на разных машинах.
fn test_zone() -> std::rc::Rc<dyn bsl_rt::TimeZone> {
    std::rc::Rc::new(bsl_rt::FixedTimeZone::new(3 * 3600).expect("допустимое смещение"))
}

#[test]
fn method_codes_are_static_and_dense() {
    let codes = objects::XDTO_METHODS
        .iter()
        .map(|method| method.code.get())
        .collect::<Vec<_>>();
    assert_eq!(
        codes,
        (1..=objects::XDTO_METHODS.len() as u16).collect::<Vec<_>>()
    );
}

/// Модель типов из текста XSD — тем же путём, что и в бою: дерево
/// строит `dom`, схему — `xsd`, а типы — этот модуль.
fn model(text: &str) -> Rc<XdtoModel> {
    let schema = crate::xsd::schema_of_text(text).expect("схема обязана разбираться");
    model_of_schema(&schema, test_zone()).expect("модель обязана строиться")
}

/// Схема из `measure-xdto.bsl`, сокращённая до того, что проверяют
/// тесты ниже. Имена и порядок объявлений — те же, поэтому измеренные
/// строки платформы читаются рядом с ожиданиями.
///
/// ОДНО ОТЛИЧИЕ ОТ `measure-xdto.bsl`: у `Code` здесь нет фасета
/// образца, он вынесен в отдельный тип `Pat`. Иначе `Code` стал бы
/// непригоден целиком — проверка по образцу отвечает честной ошибкой
/// «не поддерживается» (см. [`check_pattern`]), — а через `code`
/// проверяется совсем другое: окно списка, порядок заполнения и
/// приведение при записи. Раздельные типы на каждый вид фасета — то же
/// устройство, что у схемы `measure-xdto-validation.bsl`.
const SAMPLE: &str = concat!(
    r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:t="urn:test" "#,
    r#"targetNamespace="urn:test" elementFormDefault="qualified" "#,
    r#"attributeFormDefault="unqualified">"#,
    r#"<xs:simpleType name="Code"><xs:restriction base="xs:string">"#,
    r#"<xs:minLength value="2"/><xs:maxLength value="5"/>"#,
    r#"</xs:restriction></xs:simpleType>"#,
    r#"<xs:simpleType name="Pat"><xs:restriction base="xs:string">"#,
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

/// Получатель за значением: поверхность XDTO принимает объект.
fn ob(v: &BslValue) -> &dyn ObjectProtocol {
    v.object_ref().expect("объект XDTO").as_dyn()
}

fn prop(obj: &BslValue, name: &str) -> BslValue {
    get_property(ob(obj), name).unwrap_or_else(|e| panic!("член «{name}»: {e}"))
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
    let repr = repr_of(&props).expect("ожидалась коллекция");
    let len = collection_len(repr).expect("коллекция").expect("длина");
    (0..len)
        .map(|i| {
            text_of(&prop(
                &collection_get(repr, i).expect("элемент коллекции"),
                "Имя",
            ))
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
        vec![
            "id", "opt", "q", "fx", "name", "code", "def", "many5", "notype", "uq", "anon"
        ]
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
        let p = collection_lookup(ob(&props), &[str_value(prop_name)]).expect("поиск свойства");
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
    let by_name = |n: &str| collection_lookup(ob(&props), &[str_value(n)]).expect("поиск");
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
        collection_lookup(ob(&props), &[str_value("нетТакого")]).expect("поиск"),
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
    let by_name = |n: &str| collection_lookup(ob(&props), &[str_value(n)]).expect("поиск");
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
        collection_len(repr_of(&anon_props).expect("ожидалась коллекция"))
            .expect("коллекция")
            .expect("длина"),
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
        collection_lookup(ob(&prop(&simp, "Свойства")), &[str_value("__content")]).expect("поиск");
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
            value => {
                let repr = repr_of(value).expect("ожидалась коллекция");
                let len = collection_len(repr).expect("коллекция").expect("длина");
                (0..len)
                    .map(|i| {
                        let f = collection_get(repr, i).expect("фасет");
                        (prop(&f, "Вид"), text_of(&prop(&f, "Значение")))
                    })
                    .collect::<Vec<_>>()
            }
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
        ]
    );
    // Образец ЧИТАЕТСЯ как всякий другой фасет — отказывает только
    // проверка по нему (см. `pattern_facet_is_an_honest_unsupported_error`).
    assert_eq!(
        facets_of("urn:test", "Pat"),
        vec![(
            BslValue::Enum(EnumValue::XdtoFacetPattern),
            "[A-Z]+".to_string()
        )]
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

// --- проверка по фасетам ---------------------------------------------
//
// Схема ниже — та же, что у `measure-xdto-validation.bsl`: на каждый
// вид фасета свой тип, чтобы отказ нельзя было списать на соседний
// фасет, — поэтому ожидания тестов читаются прямо рядом со снятыми
// строками `measure-xdto-validation.platform.txt`.

const FACETS: &str = concat!(
    r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:t="urn:v" "#,
    r#"targetNamespace="urn:v" elementFormDefault="qualified" "#,
    r#"attributeFormDefault="unqualified">"#,
    r#"<xs:simpleType name="Col"><xs:restriction base="xs:string">"#,
    r#"<xs:enumeration value="red"/><xs:enumeration value="green"/>"#,
    r#"</xs:restriction></xs:simpleType>"#,
    r#"<xs:simpleType name="Len"><xs:restriction base="xs:string">"#,
    r#"<xs:minLength value="2"/><xs:maxLength value="5"/>"#,
    r#"</xs:restriction></xs:simpleType>"#,
    r#"<xs:simpleType name="Len2"><xs:restriction base="t:Len">"#,
    r#"<xs:maxLength value="3"/></xs:restriction></xs:simpleType>"#,
    r#"<xs:simpleType name="Fix"><xs:restriction base="xs:string">"#,
    r#"<xs:length value="3"/></xs:restriction></xs:simpleType>"#,
    r#"<xs:simpleType name="Rng"><xs:restriction base="xs:decimal">"#,
    r#"<xs:minInclusive value="0"/><xs:maxExclusive value="100"/>"#,
    r#"</xs:restriction></xs:simpleType>"#,
    r#"<xs:simpleType name="Rng2"><xs:restriction base="xs:decimal">"#,
    r#"<xs:minExclusive value="0"/><xs:maxInclusive value="100"/>"#,
    r#"</xs:restriction></xs:simpleType>"#,
    r#"<xs:simpleType name="Dig"><xs:restriction base="xs:decimal">"#,
    r#"<xs:totalDigits value="4"/><xs:fractionDigits value="2"/>"#,
    r#"</xs:restriction></xs:simpleType>"#,
    r#"<xs:simpleType name="Pat"><xs:restriction base="xs:string">"#,
    r#"<xs:pattern value="[A-Z]+"/></xs:restriction></xs:simpleType>"#,
    r#"<xs:simpleType name="Lst"><xs:list itemType="t:Len"/></xs:simpleType>"#,
    r#"<xs:simpleType name="LstMax"><xs:restriction base="t:Lst">"#,
    r#"<xs:maxLength value="2"/></xs:restriction></xs:simpleType>"#,
    r#"<xs:simpleType name="Uni"><xs:union memberTypes="t:Col xs:int"/></xs:simpleType>"#,
    r#"<xs:simpleType name="ColN"><xs:restriction base="xs:decimal">"#,
    r#"<xs:enumeration value="1.0"/><xs:enumeration value="2"/>"#,
    r#"</xs:restriction></xs:simpleType>"#,
    r#"<xs:simpleType name="Bin"><xs:restriction base="xs:base64Binary">"#,
    r#"<xs:maxLength value="2"/></xs:restriction></xs:simpleType>"#,
    r#"<xs:simpleType name="Dt"><xs:restriction base="xs:date">"#,
    r#"<xs:minInclusive value="2020-01-01"/><xs:maxExclusive value="2030-01-01"/>"#,
    r#"</xs:restriction></xs:simpleType>"#,
    r#"<xs:complexType name="T"><xs:sequence>"#,
    r#"<xs:element name="el" type="t:Len" minOccurs="0"/>"#,
    r#"<xs:element name="ml" type="t:Len" minOccurs="0" maxOccurs="unbounded"/>"#,
    r#"<xs:element name="ip" type="xs:int" minOccurs="0"/>"#,
    r#"<xs:element name="pt" type="t:Pat" minOccurs="0"/>"#,
    r#"<xs:element name="bn" type="t:Bin" minOccurs="0"/>"#,
    r#"</xs:sequence>"#,
    r#"<xs:attribute name="ac" type="t:Col"/>"#,
    r#"</xs:complexType>"#,
    r#"</xs:schema>"#,
);

/// Значение своего типа из лексической формы — тем же путём, что и
/// `ФабрикаXDTO.Создать`, то есть С проверкой фасетов.
fn checked(f: &BslValue, name: &str, lexical: &str) -> RtResult<BslValue> {
    let t = factory_type(ob(f), &[str_value("urn:v"), str_value(name)])?;
    let value = factory_create(ob(f), &[t, str_value(lexical)])?;
    get_property(ob(&value), "Значение")
}

/// Значение встроенного типа тем же путём.
fn checked_builtin(f: &BslValue, name: &str, lexical: &str) -> RtResult<BslValue> {
    let t = factory_type(ob(f), &[str_value(XSD_NS), str_value(name)])?;
    let value = factory_create(ob(f), &[t, str_value(lexical)])?;
    get_property(ob(&value), "Значение")
}

/// Перечисление, длины и границы — та же сетка принятых и отвергнутых
/// значений, что снята на платформе.
#[test]
fn facet_checks_follow_the_measured_grid() {
    let f = factory(FACETS);
    // Перечисление: только перечисленные записи, с учётом регистра.
    assert_eq!(text_of(&checked(&f, "Col", "red").expect("red")), "red");
    assert!(checked(&f, "Col", "синий").is_err());
    assert!(checked(&f, "Col", "RED").is_err());
    assert!(checked(&f, "Col", "").is_err());
    // Длины: обе границы включающие, пустая строка короче двух.
    for lexical in ["аб", "абв", "абвгд"] {
        assert!(checked(&f, "Len", lexical).is_ok(), "{lexical}");
    }
    for lexical in ["а", "абвгде", ""] {
        assert!(checked(&f, "Len", lexical).is_err(), "{lexical}");
    }
    // `length` — ровно столько символов.
    assert!(checked(&f, "Fix", "абв").is_ok());
    assert!(checked(&f, "Fix", "аб").is_err());
    assert!(checked(&f, "Fix", "абвг").is_err());
    // Границы: включающая пропускает саму себя, исключающая — нет.
    assert!(checked(&f, "Rng", "0").is_ok());
    assert!(checked(&f, "Rng", "99.5").is_ok());
    assert!(checked(&f, "Rng", "-1").is_err());
    assert!(checked(&f, "Rng", "100").is_err());
    assert!(checked(&f, "Rng2", "0").is_err());
    assert!(checked(&f, "Rng2", "100").is_ok());
    assert!(checked(&f, "Rng2", "101").is_err());
    // Разряды: всего и в дробной части; хвостовые нули не считаются,
    // потому что их снимает нормализация числа.
    assert!(checked(&f, "Dig", "12.34").is_ok());
    assert!(checked(&f, "Dig", "1234").is_ok());
    assert!(checked(&f, "Dig", "1.200").is_ok());
    assert!(checked(&f, "Dig", "123.45").is_err());
    assert!(checked(&f, "Dig", "1.234").is_err());
    assert!(checked(&f, "Dig", "12345").is_err());
    // Границы у ДАТЫ сравниваются так же, как у числа (измерено).
    assert!(checked(&f, "Dt", "2026-01-01").is_ok());
    assert!(checked(&f, "Dt", "2019-01-01").is_err());
    assert!(checked(&f, "Dt", "2030-01-01").is_err());
}

/// Фасеты НАСЛЕДУЮТСЯ: и по цепочке схемы, и по табличной цепочке
/// встроенных типов, куда разбор лексической формы не спускается.
#[test]
fn facets_are_inherited_along_the_whole_chain() {
    let f = factory(FACETS);
    // `Len2` ограничивает `Len`: своя верхняя граница строже, нижняя
    // досталась от базового типа.
    assert!(checked(&f, "Len2", "абв").is_ok());
    assert!(checked(&f, "Len2", "абвг").is_err());
    assert!(checked(&f, "Len2", "а").is_err());
    // Встроенная цепочка: диапазон у `int` от него самого, у `byte` —
    // от `byte`, «разряды дробной части» у `integer` — от `integer`.
    assert!(checked_builtin(&f, "int", "42").is_ok());
    assert!(checked_builtin(&f, "int", "3000000000").is_err());
    assert!(checked_builtin(&f, "int", "-3000000000").is_err());
    assert!(checked_builtin(&f, "byte", "200").is_err());
    assert!(checked_builtin(&f, "unsignedByte", "-1").is_err());
    assert!(checked_builtin(&f, "positiveInteger", "0").is_err());
    assert!(checked_builtin(&f, "integer", "1.5").is_err());
    // А у самого `decimal` разрядов не ограничено.
    assert!(checked_builtin(&f, "decimal", "1.5").is_ok());
}

/// Длина считается по виду значения: символы у строки, БАЙТЫ у
/// двоичных данных, ЭЛЕМЕНТЫ у списочного типа (измерено).
#[test]
fn facet_length_counts_characters_bytes_and_items() {
    let f = factory(FACETS);
    // Кириллица — по символам, а не по байтам UTF-8: «аб» проходит
    // нижнюю границу в два символа.
    assert!(checked(&f, "Len", "аб").is_ok());
    // Двоичные данные: «0LA=» — два байта, «0LDQsQ==» — четыре.
    assert!(checked(&f, "Bin", "0LA=").is_ok());
    assert!(checked(&f, "Bin", "0LDQsQ==").is_err());
    // ЗАПИСЬ двоичных данных идёт мимо лексической формы (обратной у
    // них здесь нет), и фасет достаётся им по самому значению.
    let t = factory_type(ob(&f), &[str_value("urn:v"), str_value("T")]).expect("тип");
    let o = factory_create(ob(&f), std::slice::from_ref(&t)).expect("экземпляр");
    let two = checked_builtin(&f, "base64Binary", "0LA=").expect("два байта");
    let four = checked_builtin(&f, "base64Binary", "0LDQsQ==").expect("четыре байта");
    assert!(set_property(ob(&o), "bn", two).is_ok());
    assert!(set_property(ob(&o), "bn", four).is_err());
    // Списочный тип: длина — число элементов, а сами элементы
    // проверяются типом элемента.
    assert!(checked(&f, "Lst", "аб вг").is_ok());
    assert!(checked(&f, "Lst", "аб в").is_err());
    assert!(checked(&f, "LstMax", "аб вг").is_ok());
    assert!(checked(&f, "LstMax", "аб вг де").is_err());
}

/// Перечисление сравнивает ЗНАЧЕНИЯ, а не записи: при перечисленных
/// «1.0» и «2» проходят и «1», и «2.00» (измерено).
#[test]
fn enumeration_facet_compares_values_not_spelling() {
    let f = factory(FACETS);
    assert!(checked(&f, "ColN", "1.0").is_ok());
    assert!(checked(&f, "ColN", "1").is_ok());
    assert!(checked(&f, "ColN", "2.00").is_ok());
    assert!(checked(&f, "ColN", "3").is_err());
}

/// Член объединения выбирается С УЧЁТОМ фасетов: «5» разбирается
/// строкой первым членом, но его перечисление эту строку отвергает, и
/// значением выходит число (измерено).
#[test]
fn union_picks_the_first_member_whose_facets_accept() {
    let f = factory(FACETS);
    assert_eq!(text_of(&checked(&f, "Uni", "red").expect("red")), "red");
    assert_eq!(number_of(&checked(&f, "Uni", "5").expect("5")), 5);
    assert!(checked(&f, "Uni", "синий").is_err());
}

/// Проверка стоит во всех измеренных точках: запись свойства, список,
/// чтение документа.
#[test]
fn facet_checks_reach_writes_and_reads() {
    let f = factory(FACETS);
    let t = factory_type(ob(&f), &[str_value("urn:v"), str_value("T")]).expect("тип");
    let o = factory_create(ob(&f), std::slice::from_ref(&t)).expect("экземпляр");
    // Присваивание и `Установить`.
    assert!(set_property(ob(&o), "el", str_value("аб")).is_ok());
    assert!(set_property(ob(&o), "el", str_value("а")).is_err());
    assert!(object_set(ob(&o), &[str_value("ac"), str_value("green")]).is_ok());
    assert!(object_set(ob(&o), &[str_value("ac"), str_value("синий")]).is_err());
    // Приведение к лексической форме идёт ДО проверки: число 5
    // становится строкой «5», и она короче двух символов.
    assert!(set_property(ob(&o), "el", number_value(42)).is_ok());
    assert!(set_property(ob(&o), "el", number_value(5)).is_err());
    // Встроенные фасеты на записи: «1.5» не целое, 3000000000 вне
    // диапазона `xs:int`.
    assert!(set_property(ob(&o), "ip", number_value(42)).is_ok());
    assert!(set_property(ob(&o), "ip", number_value(3_000_000_000)).is_err());
    // Список: `Добавить`, `Установить` и `Вставить` проверяют так же.
    let list = get_property(ob(&o), "ml").expect("список");
    assert!(list_add(ob(&list), &[str_value("аб")]).is_ok());
    assert!(list_add(ob(&list), &[str_value("а")]).is_err());
    assert!(list_set(ob(&list), &[number_value(0), str_value("вгд")]).is_ok());
    assert!(list_set(ob(&list), &[number_value(0), str_value("в")]).is_err());
    assert!(list_insert(ob(&list), &[number_value(0), str_value("вг")]).is_ok());
    assert!(list_insert(ob(&list), &[number_value(0), str_value("в")]).is_err());
    // Чтение документа с типом проверяет и элемент, и атрибут, а
    // валидный документ читается по-прежнему.
    let read = |text: &str| factory_read_xml(ob(&f), &[reader(text), t.clone()]);
    let ok = read(r#"<т xmlns="urn:v" ac="red"><el>аб</el></т>"#).expect("валидный документ");
    assert_eq!(text_of(&get_property(ob(&ok), "el").expect("el")), "аб");
    assert!(read(r#"<т xmlns="urn:v" ac="синий"/>"#).is_err());
    assert!(read(r#"<т xmlns="urn:v"><el>а</el></т>"#).is_err());
    assert!(read(r#"<т xmlns="urn:v"><ip>3000000000</ip></т>"#).is_err());
}

/// Умолчание схемы, нарушающее фасет, валит построение ФАБРИКИ целиком
/// (измерено: `СоздатьФабрикуXDTO` от такой схемы — ошибка).
#[test]
fn a_facet_violating_default_breaks_the_whole_factory() {
    let bad = concat!(
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:t="urn:w" "#,
        r#"targetNamespace="urn:w" elementFormDefault="qualified">"#,
        r#"<xs:simpleType name="Len"><xs:restriction base="xs:string">"#,
        r#"<xs:minLength value="2"/></xs:restriction></xs:simpleType>"#,
        r#"<xs:complexType name="T2"><xs:sequence>"#,
        r#"<xs:element name="bad" type="t:Len" default="а" minOccurs="0"/>"#,
        r#"</xs:sequence></xs:complexType></xs:schema>"#,
    );
    let schema = crate::xsd::schema_of_text(bad).expect("схема разбирается");
    assert!(model_of_schema(&schema, test_zone()).is_err());
    // Годное умолчание того же вида модель строит.
    let good = bad.replace(r#"default="а""#, r#"default="аб""#);
    let schema = crate::xsd::schema_of_text(&good).expect("схема разбирается");
    assert!(model_of_schema(&schema, test_zone()).is_ok());
}

/// Фасет образца — честная ошибка «не поддерживается», и она
/// одинакова для годного и негодного значения: разбирать образец
/// нечем, а частично истолкованный образец хуже отказа.
///
/// Образцы ВСТРОЕННЫХ типов из-под этого правила выведены: иначе
/// отказом отвечала бы любая запись в `xs:int`, чей предок `xs:integer`
/// несёт образец `[\-+]?[0-9]+`.
#[test]
fn pattern_facet_is_an_honest_unsupported_error() {
    let f = factory(FACETS);
    let err = checked(&f, "Pat", "AB").expect_err("образец не поддерживается");
    let text = err.to_string();
    assert!(text.contains("не поддерживается"), "текст отказа: {text}");
    assert!(text.contains("образц"), "текст отказа: {text}");
    assert!(checked(&f, "Pat", "аб").is_err());
    // Запись в свойство такого типа — тот же отказ.
    let t = factory_type(ob(&f), &[str_value("urn:v"), str_value("T")]).expect("тип");
    let o = factory_create(ob(&f), std::slice::from_ref(&t)).expect("экземпляр");
    assert!(set_property(ob(&o), "pt", str_value("AB")).is_err());
    // Встроенные типы с образцом работают: целые — по своему разбору,
    // `xs:Name` — с ЗАВЫШЕННОЙ терпимостью (платформа отвергает «1имя»,
    // здесь оно проходит; расхождение названо в шапке модуля).
    assert!(checked_builtin(&f, "int", "42").is_ok());
    assert!(checked_builtin(&f, "Name", "имя").is_ok());
    assert!(checked_builtin(&f, "Name", "1имя").is_ok());
}

/// Значение по умолчанию — `ЗначениеXDTO` из `default` и из `fixed`.
#[test]
fn default_value_comes_from_both_default_and_fixed() {
    let m = model(SAMPLE);
    let root = type_of(&m, "urn:test", "RootType");
    let props = prop(&root, "Свойства");
    let by_name = |n: &str| collection_lookup(ob(&props), &[str_value(n)]).expect("поиск");
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

/// Лексическая форма с ПОЯСОМ пересчитывается в местное время той зоны,
/// в которой построена фабрика, а не машины.
///
/// ИЗМЕРЕНО на 8.3.27 (комментарий у `facets::apply_zone`):
/// `2026-08-12T18:41:17Z` дало 21:41:17 на машине с +03:00, а
/// `…+02:00` — 19:41:17. Здесь тот же пересчёт делается ДВАЖДЫ на двух
/// разных зонах: до переноса зоны в прогон второй результат был бы
/// недостижим, потому что смещение бралось из процессного кэша
/// `/etc/localtime` и на всю программу было одно.
#[test]
fn a_written_zone_is_converted_into_the_factories_own_zone() {
    let parse_in = |offset_hours: i32, lexical: &str| {
        let schema = crate::xsd::schema_of_text(SAMPLE).expect("схема");
        let zone: Rc<dyn bsl_rt::TimeZone> =
            Rc::new(bsl_rt::FixedTimeZone::new(offset_hours * 3600).expect("допустимое смещение"));
        let m = model_of_schema(&schema, zone).expect("модель");
        let index = m.find(XSD_NS, "dateTime").expect("встроенный тип");
        value_from_lexical(&m, index, lexical).expect("лексическая форма")
    };
    let civil = |v: &BslValue| match v {
        BslValue::Date(d) => d.to_civil(),
        other => panic!("ожидалась дата, получено {other:?}"),
    };

    // Момент один и тот же; зоны разные — и результат разный.
    let east = civil(&parse_in(3, "2026-08-12T18:41:17Z"));
    assert_eq!((east.hour, east.minute, east.second), (21, 41, 17));
    let mid = civil(&parse_in(2, "2026-08-12T18:41:17Z"));
    assert_eq!((mid.hour, mid.minute, mid.second), (20, 41, 17));

    // Записанный пояс тоже учитывается: `+02:00` в зоне +03:00 — 19:41:17.
    let written = civil(&parse_in(3, "2026-08-12T18:41:17+02:00"));
    assert_eq!((written.hour, written.minute, written.second), (19, 41, 17));

    // Без пояса запись остаётся как есть, в какой бы зоне ни читали.
    for hours in [0, 3, -5] {
        let bare = civil(&parse_in(hours, "2026-08-12T18:41:17"));
        assert_eq!((bare.hour, bare.minute, bare.second), (18, 41, 17));
    }
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
            BslValue::Type(TypeRef::Native(TypeId::String)),
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
            BslValue::Type(TypeRef::Native(TypeId::Number)),
            "{name}"
        );
    }
    assert_eq!(
        type_of_value("boolean", "true"),
        BslValue::Type(TypeRef::Native(TypeId::Boolean))
    );
    assert_eq!(
        type_of_value("date", "2026-08-12"),
        BslValue::Type(TypeRef::Native(TypeId::Date))
    );
    assert_eq!(
        type_of_value("dateTime", "2026-08-12T18:41:17"),
        BslValue::Type(TypeRef::Native(TypeId::Date))
    );
    assert_eq!(
        type_of_value("time", "18:41:17"),
        BslValue::Type(TypeRef::Native(TypeId::Date))
    );
    assert_eq!(
        type_of_value("base64Binary", "0LDQsQ=="),
        BslValue::Type(TypeRef::Native(TypeId::BinaryData))
    );
    assert_eq!(
        type_of_value("hexBinary", "D0B0D0B1"),
        BslValue::Type(TypeRef::Native(TypeId::BinaryData))
    );
    assert_eq!(
        type_of_value("QName", "просто"),
        BslValue::Type(TypeRef::Object(&crate::xsd::EXPANDED_NAME_TYPE))
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
        BslValue::Type(TypeRef::Object(&crate::xdto::objects::OBJECT_TYPE_TYPE))
    );
    assert_eq!(
        crate::xdto::objects::OBJECT_TYPE_TYPE.type_display,
        "Тип объекта XDTO"
    );
    let code = type_of(&m, "urn:test", "Code");
    assert_eq!(code.to_string(), "{urn:test}Code");
    assert_eq!(
        code.type_of().unwrap(),
        BslValue::Type(TypeRef::Object(&crate::xdto::objects::VALUE_TYPE_TYPE))
    );
    let props = prop(&root, "Свойства");
    assert_eq!(
        props.type_of().unwrap(),
        BslValue::Type(TypeRef::Object(&crate::xdto::objects::PROPERTIES_TYPE))
    );
    assert_eq!(props.to_string(), "КоллекцияСвойствXDTO");
    let name = collection_lookup(ob(&props), &[str_value("name")]).expect("поиск");
    assert_eq!(name.to_string(), "name", "свойство печатается именем");
    assert_eq!(
        name.type_of().unwrap(),
        BslValue::Type(TypeRef::Object(&crate::xdto::objects::PROPERTY_TYPE))
    );
    // Анонимный тип печатается ПУСТОЙ строкой, хотя URI у него есть.
    let anon = prop(
        &collection_lookup(ob(&props), &[str_value("anon")]).expect("поиск"),
        "Тип",
    );
    assert_eq!(anon.to_string(), "");
    assert_eq!(text_of(&prop(&anon, "URIПространстваИмен")), "urn:test");
    let facets = prop(&code, "Фасеты");
    assert_eq!(
        facets.type_of().unwrap(),
        BslValue::Type(TypeRef::Object(&crate::xdto::objects::FACETS_TYPE))
    );
    assert_eq!(facets.to_string(), "КоллекцияФасетовXDTO");
    {
        let repr = repr_of(&facets).expect("ожидалась коллекция");
        let f = collection_get(repr, 0).expect("фасет");
        assert_eq!(
            f.type_of().unwrap(),
            BslValue::Type(TypeRef::Object(&crate::xdto::objects::FACET_TYPE))
        );
        assert_eq!(f.to_string(), "ФасетXDTO");
    }
    let def = prop(
        &collection_lookup(ob(&props), &[str_value("def")]).expect("поиск"),
        "ЗначениеПоУмолчанию",
    );
    assert_eq!(
        def.type_of().unwrap(),
        BslValue::Type(TypeRef::Object(&crate::xdto::objects::VALUE_TYPE))
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
                assert_eq!(ours.type_name(), measured, "тип BSL для {name}");
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
    let error = model_of_schema(&schema, test_zone()).expect_err("тип не разрешается");
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
    let cyclic = model_of_schema(&schema, test_zone())
        .expect("модель строится: цикл здесь только у значений");
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
    let error = model_of_schema(&schema, test_zone()).expect_err("цикл наследования");
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
    assert!(
        model_of_schema(&schema, test_zone()).is_err(),
        "мусор в default"
    );

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
        model_of_schema(&schema, test_zone()).is_err(),
        "испорченная дата в default"
    );

    // Неизвестный член — `RtError`, а не паника.
    let m = model(SAMPLE);
    let root = type_of(&m, "urn:test", "RootType");
    assert!(get_property(ob(&root), "НетТакогоЧлена").is_err());
    // Члены типа ЗНАЧЕНИЯ на типе объекта не отвечают, и наоборот.
    assert!(get_property(ob(&root), "Фасеты").is_err());
    assert!(get_property(ob(&type_of(&m, "urn:test", "Code")), "Свойства").is_err());
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
    factory_value(model_of_schemas(&schemas, test_zone()).expect("модель обязана строиться"))
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
    let row = factory_type(ob(&f), &[str_value("urn:b"), str_value("Row")]).expect("тип");
    assert_eq!(row.to_string(), "{urn:b}Row");
    let code = factory_type(ob(&f), &[str_value("urn:a"), str_value("Code")]).expect("тип");
    assert_eq!(code.to_string(), "{urn:a}Code");
    // Свойство схемы B ссылается на тип схемы A — и ссылка связана.
    let by_name =
        collection_lookup(ob(&prop(&row, "Свойства")), &[str_value("code")]).expect("поиск");
    assert_eq!(prop(&by_name, "Тип"), code);
    // Порядок схем в наборе на разрешение не влияет.
    let reversed = factory_of_texts(&[b, a]);
    assert_eq!(
        factory_type(ob(&reversed), &[str_value("urn:a"), str_value("Code")])
            .expect("тип")
            .to_string(),
        "{urn:a}Code"
    );
    // Встроенные типы объявлены ОДИН раз на всю модель, а не по разу
    // на схему: иначе `find` возвращал бы первый из двух одинаковых.
    let string = factory_type(ob(&f), &[str_value(XSD_NS), str_value("string")]).expect("тип");
    assert_eq!(string.to_string(), format!("{{{XSD_NS}}}string"));
    // Пустой набор — это фабрика с одними встроенными типами.
    let empty = factory_of_texts(&[]);
    assert_eq!(
        factory_type(ob(&empty), &[str_value(XSD_NS), str_value("string")]).expect("тип"),
        factory_type(ob(&empty), &[str_value(XSD_NS), str_value("string")]).expect("тип")
    );
    assert_eq!(
        factory_type(ob(&empty), &[str_value("urn:a"), str_value("Code")]).expect("тип"),
        BslValue::Undefined
    );
}

/// `Тип` берёт пару (URI, имя) или расширенное имя, неизвестное имя
/// даёт `Неопределено`, а два обращения за одним именем — равные
/// значения (всё измерено).
#[test]
fn factory_type_takes_a_pair_or_an_expanded_name() {
    let f = factory(SAMPLE);
    let pair = factory_type(ob(&f), &[str_value("urn:test"), str_value("RootType")]).expect("тип");
    assert_eq!(pair.to_string(), "{urn:test}RootType");
    let expanded = crate::xsd::new_expanded_name("urn:test", "RootType");
    assert_eq!(factory_type(ob(&f), &[expanded]).expect("тип"), pair);
    // Два обращения за одним именем равны — тип это ссылка в модель.
    assert_eq!(
        factory_type(ob(&f), &[str_value("urn:test"), str_value("RootType")]).expect("тип"),
        pair
    );
    // Неизвестное имя и чужой URI — `Неопределено`, а не ошибка.
    for args in [
        [str_value("urn:test"), str_value("НетТакого")],
        [str_value("urn:нет"), str_value("RootType")],
        // Пустой URI (измерено на `Тип("", "RootType")`).
        [str_value(""), str_value("RootType")],
    ] {
        assert_eq!(
            factory_type(ob(&f), &args).expect("поиск"),
            BslValue::Undefined
        );
    }
    // Одна строка, три аргумента, числа вместо имён и вызов без
    // аргументов — ошибка (измерено все четыре).
    assert!(factory_type(ob(&f), &[str_value("RootType")]).is_err());
    assert!(
        factory_type(
            ob(&f),
            &[
                str_value("urn:test"),
                str_value("RootType"),
                number_value(1)
            ]
        )
        .is_err()
    );
    assert!(factory_type(ob(&f), &[number_value(5), number_value(5)]).is_err());
    assert!(factory_type(ob(&f), &[]).is_err());
    // Получатель обязан быть фабрикой.
    assert!(factory_type(ob(&pair), &[str_value("urn:test"), str_value("RootType")]).is_err());
}

/// `Создать` от типа ЗНАЧЕНИЯ: без лексики — `Неопределено`, с
/// лексикой — `ЗначениеXDTO` с обоими членами.
#[test]
fn factory_create_builds_a_value_from_its_lexical_form() {
    let f = factory(SAMPLE);
    let code = factory_type(ob(&f), &[str_value("urn:test"), str_value("Code")]).expect("тип");
    let value = factory_create(ob(&f), &[code.clone(), str_value("AB")]).expect("значение");
    assert_eq!(
        value.type_of().unwrap(),
        BslValue::Type(TypeRef::Object(&crate::xdto::objects::VALUE_TYPE))
    );
    assert_eq!(text_of(&prop(&value, "Значение")), "AB");
    assert_eq!(text_of(&prop(&value, "ЛексическоеЗначение")), "AB");
    // Лексическая форма разбирается по ТИПУ: свой тип наследует
    // отображение базового, а встроенный числовой даёт число.
    let int = factory_type(ob(&f), &[str_value(XSD_NS), str_value("int")]).expect("тип");
    let number = factory_create(ob(&f), &[int.clone(), str_value("-42")]).expect("значение");
    assert_eq!(number_of(&prop(&number, "Значение")), -42);
    // Без лексической формы — `Неопределено` (измерено).
    assert_eq!(
        factory_create(ob(&f), std::slice::from_ref(&int)).expect("вызов"),
        BslValue::Undefined
    );
    // Третий аргумент платформа принимает, четвёртый — уже нет.
    assert!(factory_create(ob(&f), &[int.clone(), str_value("1"), number_value(1)]).is_ok());
    assert!(
        factory_create(
            ob(&f),
            &[
                int.clone(),
                str_value("1"),
                number_value(1),
                number_value(1)
            ]
        )
        .is_err()
    );
    // Не разбирающаяся форма — ошибка, а не подстановка.
    assert!(factory_create(ob(&f), &[int, str_value("ерунда")]).is_err());
    // Первый аргумент — обязательно тип XDTO, а лексическая форма —
    // обязательно строка (нестроковую см. в шапке модуля).
    assert!(factory_create(ob(&f), &[str_value("string"), str_value("аб")]).is_err());
    assert!(factory_create(ob(&f), &[code, number_value(42)]).is_err());
    assert!(factory_create(ob(&f), &[]).is_err());
}

/// `Создать` от типа ОБЪЕКТА даёт экземпляр: он печатается своим
/// именем и отдаёт свой тип методом `Тип()`.
#[test]
fn factory_create_builds_an_object_that_knows_its_type() {
    let f = factory(SAMPLE);
    let root = factory_type(ob(&f), &[str_value("urn:test"), str_value("RootType")]).expect("тип");
    let object = factory_create(ob(&f), std::slice::from_ref(&root)).expect("экземпляр");
    assert_eq!(object.to_string(), "ОбъектXDTO");
    assert_eq!(
        object.type_of().unwrap(),
        BslValue::Type(TypeRef::Object(&crate::xdto::objects::OBJECT_TYPE))
    );
    assert_eq!(
        crate::xdto::objects::OBJECT_TYPE.type_display,
        "Объект XDTO"
    );
    assert_eq!(object_type(ob(&object), &[]).expect("тип"), root);
    // Аргументов у `Тип()` нет, а два экземпляра одного типа не равны
    // (измерено обе стороны).
    assert!(object_type(ob(&object), &[number_value(1)]).is_err());
    assert_ne!(object, factory_create(ob(&f), &[root]).expect("экземпляр"));
    // Лексической формы тип объекта не берёт, абстрактный тип
    // экземпляров не имеет (измерено).
    let abstr =
        factory_type(ob(&f), &[str_value("urn:test"), str_value("AbstrType")]).expect("тип");
    assert!(factory_create(ob(&f), &[abstr]).is_err());
    let empty =
        factory_type(ob(&f), &[str_value("urn:test"), str_value("EmptyType")]).expect("тип");
    assert!(factory_create(ob(&f), &[empty.clone(), str_value("аб")]).is_err());
    assert!(factory_create(ob(&f), &[empty]).is_ok());
    // Незаполненное свойство — `Неопределено`, а постороннее имя —
    // ошибка (измерено обе стороны).
    assert_eq!(
        get_property(ob(&object), "name").expect("свойство читается"),
        BslValue::Undefined
    );
    assert!(get_property(ob(&object), "нетТакого").is_err());
    // `Тип` у экземпляра — метод, а не член: обращение как к свойству
    // отвечает ошибкой (измерено).
    assert!(get_property(ob(&object), "Тип").is_err());
}

/// Фабрика по набору схем строится только из набора: путь, схема и
/// прочее — ошибка (измерено), а `Неопределено` значит «без схем».
#[test]
fn a_factory_is_built_from_a_schema_set_or_from_nothing() {
    let empty = factory_of_schema_set(&BslValue::Undefined, test_zone()).expect("фабрика без схем");
    assert_eq!(empty.to_string(), "ФабрикаXDTO");
    assert_eq!(
        empty.type_of().unwrap(),
        BslValue::Type(TypeRef::Object(&crate::xdto::objects::FACTORY_TYPE))
    );
    assert_eq!(
        crate::xdto::objects::FACTORY_TYPE.type_display,
        "Фабрика XDTO"
    );
    assert!(is_factory(&empty));
    let set = crate::xsd::new_schema_set();
    assert!(factory_of_schema_set(&set, test_zone()).is_ok());
    // Путь к файлу, схема и число сюда не годятся.
    for wrong in [
        str_value("/tmp/схема.xsd"),
        crate::xsd::new_schema(),
        number_value(1),
    ] {
        assert!(
            factory_of_schema_set(&wrong, test_zone()).is_err(),
            "{wrong:?}"
        );
    }
    // Две фабрики от одного и того же набора не равны (измерено на
    // двух фабриках от одного файла).
    assert_ne!(
        factory_of_schema_set(&set, test_zone()).expect("фабрика"),
        factory_of_schema_set(&set, test_zone()).expect("фабрика")
    );
    // `ЗначениеЗаполнено` от фабрики — ошибка (измерено).
    assert!(empty.is_filled().is_err());
    // Постороннего члена у фабрики нет.
    assert!(get_property(ob(&empty), "Пакеты").is_err());
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
    let f = factory_of_file(&[str_value(&path.to_string_lossy())], test_zone()).expect("фабрика");
    assert_eq!(
        factory_type(ob(&f), &[str_value("urn:f"), str_value("Code")])
            .expect("тип")
            .to_string(),
        "{urn:f}Code"
    );
    let missing = dir.join("open-bsl-xdto-factory-нет-такого.xsd");
    let error = factory_of_file(&[str_value(&missing.to_string_lossy())], test_zone())
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
    assert!(factory_of_file(&[str_value(&broken.to_string_lossy())], test_zone()).is_err());
    // Ни без аргумента, ни с двумя, ни с нестроковым (измерено).
    assert!(factory_of_file(&[], test_zone()).is_err());
    assert!(factory_of_file(&[number_value(1)], test_zone()).is_err());
    assert!(
        factory_of_file(
            &[
                str_value(&path.to_string_lossy()),
                str_value(&path.to_string_lossy())
            ],
            test_zone()
        )
        .is_err()
    );
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&broken);
}

// --- экземпляр ---------------------------------------------------------

/// Экземпляр `RootType` из фабрики над [`SAMPLE`].
fn instance(f: &BslValue, name: &str) -> BslValue {
    let t = factory_type(ob(f), &[str_value("urn:test"), str_value(name)]).expect("тип");
    factory_create(ob(f), &[t]).expect("экземпляр")
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
    assert_eq!(
        list.type_of().unwrap(),
        BslValue::Type(TypeRef::Object(&crate::xdto::objects::LIST_TYPE))
    );
    assert_eq!(crate::xdto::objects::LIST_TYPE.type_display, "Список XDTO");
    assert_eq!(list.collection_len().expect("длина"), 0);
    // Постороннее имя — ошибка, а не `Неопределено` (измерено).
    assert!(get_property(ob(&o), "нетТакого").is_err());
    // Унаследованное свойство читается у наследника так же.
    assert_eq!(prop(&instance(&f, "ExtType"), "def"), number_value(7));
}

/// Запись идёт через лексическую форму типа-приёмника — отсюда и
/// приведения, и отказы (измерено поимённо).
#[test]
fn writing_a_property_goes_through_the_lexical_form() {
    let f = factory(SAMPLE);
    let o = instance(&f, "RootType");
    set_property(ob(&o), "name", str_value("аб")).expect("строка в строку");
    assert_eq!(text_of(&prop(&o, "name")), "аб");
    // Регистр имени не важен и на записи (измерено).
    set_property(ob(&o), "NAME", number_value(5)).expect("число в строку");
    assert_eq!(text_of(&prop(&o, "name")), "5");
    set_property(ob(&o), "name", BslValue::Boolean(true)).expect("булево в строку");
    assert_eq!(text_of(&prop(&o, "name")), "true");
    let day = BslValue::Date(bsl_rt::BslDate::from_civil(2026, 8, 13, 0, 0, 0).expect("дата"));
    set_property(ob(&o), "name", day.clone()).expect("дата в строку");
    assert_eq!(text_of(&prop(&o, "name")), "2026-08-13T00:00:00");
    // В `xs:int` строка цифрами проходит, а «true» — нет: это не его
    // лексическая форма (измерено обе стороны).
    set_property(ob(&o), "id", str_value("5")).expect("строка в число");
    assert_eq!(prop(&o, "id"), number_value(5));
    assert!(set_property(ob(&o), "id", BslValue::Boolean(true)).is_err());
    // `Неопределено` и `Null` не пишутся вовсе — сброс делает
    // `Сбросить` (измерено).
    assert!(set_property(ob(&o), "name", BslValue::Undefined).is_err());
    assert!(set_property(ob(&o), "name", BslValue::Null).is_err());
    // `ЗначениеXDTO` принимается, и берётся из него ЗНАЧЕНИЕ: тип
    // источника может быть другим (измерено).
    let int_type = factory_type(
        ob(&f),
        &[
            str_value("http://www.w3.org/2001/XMLSchema"),
            str_value("string"),
        ],
    )
    .expect("тип");
    let value = factory_create(ob(&f), &[int_type, str_value("5")]).expect("значение");
    set_property(ob(&o), "id", value).expect("значение XDTO в число");
    assert_eq!(prop(&o, "id"), number_value(5));
    // Множественное свойство присваиванием не пишется (измерено).
    assert!(set_property(ob(&o), "code", str_value("AB")).is_err());
    assert!(set_property(ob(&o), "нетТакого", str_value("аб")).is_err());
    // Свойство типа `anyType` принимает что угодно как есть (измерено).
    set_property(ob(&o), "notype", number_value(5)).expect("число в anyType");
    assert_eq!(prop(&o, "notype"), number_value(5));
}

/// Список — окно в хранилище владельца, а не снимок.
#[test]
fn a_list_is_a_window_into_its_owner() {
    let f = factory(SAMPLE);
    let o = instance(&f, "RootType");
    let list = prop(&o, "code");
    list_add(ob(&list), &[str_value("AB")]).expect("добавление");
    // Видно через второе чтение свойства, и два чтения РАВНЫ
    // (измерено обе стороны).
    assert_eq!(prop(&o, "code").collection_len().expect("длина"), 1);
    assert_eq!(prop(&o, "code"), prop(&o, "code"));
    assert_eq!(prop(&list, "Владелец"), o);
    // В списке лежит само значение, а не `ЗначениеXDTO` (измерено).
    assert_eq!(text_of(&list_get(ob(&list), 0).expect("элемент")), "AB");
    assert!(list_get(ob(&list), 1).is_err());
    // Приведение то же, что при записи свойства.
    list_add(ob(&prop(&o, "many5")), &[number_value(5)]).expect("число в строку");
    assert_eq!(
        text_of(&list_get(ob(&prop(&o, "many5")), 0).expect("элемент")),
        "5"
    );
    assert!(list_add(ob(&list), &[BslValue::Undefined]).is_err());
    // `Вставить` встаёт на место указанного элемента, `Удалить` и
    // `Очистить` работают по позиции (измерено).
    list_insert(ob(&list), &[number_value(0), str_value("CD")]).expect("вставка");
    assert_eq!(text_of(&list_get(ob(&list), 0).expect("элемент")), "CD");
    assert_eq!(list.collection_len().expect("длина"), 2);
    list_set(ob(&list), &[number_value(0), str_value("EF")]).expect("установка");
    assert_eq!(text_of(&list_get(ob(&list), 0).expect("элемент")), "EF");
    assert!(list_set(ob(&list), &[number_value(9), str_value("EF")]).is_err());
    // `Вставить` требует ЗАНЯТОЙ позиции: ни за концом, ни в пустой
    // список платформа не вставляет (измерено оба).
    assert!(list_insert(ob(&list), &[number_value(2), str_value("EF")]).is_err());
    list_delete(ob(&list), &number_value(0)).expect("удаление");
    assert_eq!(list.collection_len().expect("длина"), 1);
    list_clear(ob(&list)).expect("очистка");
    assert_eq!(list.collection_len().expect("длина"), 0);
    assert!(list_delete(ob(&list), &number_value(0)).is_err());
    assert!(list_insert(ob(&list), &[number_value(0), str_value("EF")]).is_err());
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
        object_is_set(ob(&o), &[str_value("def")]).expect("установлено"),
        BslValue::Boolean(false)
    );
    object_set(ob(&o), &[str_value("name"), str_value("аб")]).expect("установка");
    assert_eq!(
        object_is_set(ob(&o), &[str_value("name")]).expect("установлено"),
        BslValue::Boolean(true)
    );
    assert_eq!(
        text_of(&object_get(ob(&o), &[str_value("name")]).expect("чтение")),
        "аб"
    );
    // Свойство можно назвать и объектом `СвойствоXDTO` (измерено).
    let properties = prop(&object_type(ob(&o), &[]).expect("тип"), "Свойства");
    let name = collection_lookup(ob(&properties), &[str_value("name")]).expect("свойство");
    assert_eq!(
        text_of(&object_get(ob(&o), std::slice::from_ref(&name)).expect("чтение")),
        "аб"
    );
    object_unset(ob(&o), &[name]).expect("сброс");
    assert_eq!(prop(&o, "name"), BslValue::Undefined);
    // Множественное свойство `Получить` не отдаёт, а `ПолучитьСписок`
    // отдаёт — и это тот же список (измерено).
    assert!(object_get(ob(&o), &[str_value("code")]).is_err());
    let list = object_get_list(ob(&o), &[str_value("code")]).expect("список");
    list_add(ob(&list), &[str_value("AB")]).expect("добавление");
    assert_eq!(prop(&o, "code").collection_len().expect("длина"), 1);
    assert!(object_get_list(ob(&o), &[str_value("name")]).is_err());
    // Постороннее имя — ошибка у всех четырёх (измерено).
    assert!(object_get(ob(&o), &[str_value("нетТакого")]).is_err());
    assert!(object_is_set(ob(&o), &[str_value("нетТакого")]).is_err());
    assert!(object_unset(ob(&o), &[str_value("нетТакого")]).is_err());
    assert!(object_get_list(ob(&o), &[str_value("нетТакого")]).is_err());
    // `Свойства()` — коллекция свойств СВОЕГО типа, `Владелец()` у
    // отдельно созданного объекта — `Неопределено` (измерено).
    assert_eq!(
        object_properties(ob(&o), &[])
            .expect("свойства")
            .collection_len()
            .expect("длина"),
        properties.collection_len().expect("длина")
    );
    assert_eq!(
        object_owner(ob(&o), &[]).expect("владелец"),
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
            ob(&prop(&object_type(ob(&o), &[]).expect("тип"), "Свойства")),
            &[str_value("anon")],
        )
        .expect("свойство"),
        "Тип",
    );
    let nested = factory_create(ob(&f), &[anon]).expect("экземпляр анонимного типа");
    set_property(ob(&o), "anon", nested.clone()).expect("объект в объектное свойство");
    // Записан ТОТ ЖЕ объект, и владелец у него — приёмник (измерено).
    assert_eq!(prop(&o, "anon"), nested);
    assert_eq!(object_owner(ob(&nested), &[]).expect("владелец"), o);
    // Посторонний тип в это свойство не пишется (измерено; наследник
    // объявленного — пишется, но в этой схеме объектного свойства с
    // ИМЕНОВАННЫМ типом нет, и проверено это на платформе).
    assert!(set_property(ob(&o), "anon", instance(&f, "EmptyType")).is_err());
    // `Проверить` смотрит и внутрь: вложенный объект пуст, а `inner`
    // у него обязателен (измерено).
    assert!(object_validate(ob(&o), &[]).is_err());
    object_validate(ob(&instance(&f, "EmptyType")), &[]).expect("пустой тип проходит");
    set_property(ob(&nested), "inner", number_value(1)).expect("запись во вложенный");
    set_property(ob(&o), "name", str_value("аб")).expect("запись");
    set_property(ob(&o), "uq", str_value("вг")).expect("запись");
    set_property(ob(&o), "id", number_value(1)).expect("запись");
    list_add(ob(&prop(&o, "many5")), &[str_value("я")]).expect("добавление");
    object_validate(ob(&o), &[]).expect("заполненный объект проходит");
    // Верхняя граница тоже проверяется: у `many5` она 5 (измерено).
    for _ in 0..5 {
        list_add(ob(&prop(&o, "many5")), &[str_value("я")]).expect("добавление");
    }
    assert!(object_validate(ob(&o), &[]).is_err());
}

/// Последовательность — порядок заполнения свойств-элементов; у
/// упорядоченного типа её нет вовсе.
#[test]
fn the_sequence_follows_the_order_of_filling() {
    let f = factory(SAMPLE);
    // `xs:sequence` — упорядоченный тип, у него `Неопределено`
    // (измерено), а `xs:choice` и `xs:all` — последовательные.
    assert_eq!(
        object_sequence(ob(&instance(&f, "RootType")), &[]).expect("последовательность"),
        BslValue::Undefined
    );
    let o = instance(&f, "ChoiceType");
    let seq = object_sequence(ob(&o), &[]).expect("последовательность");
    assert_eq!(seq.to_string(), "ПоследовательностьXDTO");
    assert_eq!(
        seq.type_of().unwrap(),
        BslValue::Type(TypeRef::Object(&crate::xdto::objects::SEQUENCE_TYPE))
    );
    assert_eq!(
        crate::xdto::objects::SEQUENCE_TYPE.type_display,
        "Последовательность XDTO"
    );
    assert_eq!(seq.collection_len().expect("длина"), 0);
    // Порядок: заполнение элементов, атрибут в него не попадает
    // (измерено).
    list_add(ob(&prop(&o, "cb")), &[str_value("аб")]).expect("добавление");
    set_property(ob(&o), "ca", str_value("вг")).expect("запись");
    set_property(ob(&o), "cat", str_value("атрибут")).expect("запись атрибута");
    list_add(ob(&prop(&o, "cb")), &[str_value("де")]).expect("добавление");
    assert_eq!(seq.collection_len().expect("длина"), 3);
    assert_eq!(
        text_of(&sequence_value(ob(&seq), &[number_value(1)]).expect("значение")),
        "вг"
    );
    assert_eq!(
        sequence_property(ob(&seq), &[number_value(1)])
            .expect("свойство")
            .to_string(),
        "ca"
    );
    assert!(sequence_value(ob(&seq), &[number_value(3)]).is_err());
    // Повторная запись одиночного свойства своё место сохраняет
    // (измерено).
    set_property(ob(&o), "ca", str_value("же")).expect("повторная запись");
    assert_eq!(seq.collection_len().expect("длина"), 3);
    assert_eq!(
        text_of(&sequence_value(ob(&seq), &[number_value(1)]).expect("значение")),
        "же"
    );
    // `Владелец` у последовательности — ЧЛЕН (измерено), а два вызова
    // `Последовательность()` дают равные значения.
    assert_eq!(prop(&seq, "Владелец"), o);
    assert_eq!(object_sequence(ob(&o), &[]).expect("вторая"), seq);
    // `Добавить` берёт именно `СвойствоXDTO` и именно элемент, а
    // заполнение видно через само свойство (измерено).
    let properties = prop(&object_type(ob(&o), &[]).expect("тип"), "Свойства");
    let ca = collection_lookup(ob(&properties), &[str_value("ca")]).expect("свойство");
    let cat = collection_lookup(ob(&properties), &[str_value("cat")]).expect("свойство");
    sequence_add(ob(&seq), &[ca, str_value("зи")]).expect("добавление");
    assert_eq!(seq.collection_len().expect("длина"), 4);
    assert_eq!(text_of(&prop(&o, "ca")), "зи");
    assert!(sequence_add(ob(&seq), &[cat, str_value("к")]).is_err());
    assert!(sequence_add(ob(&seq), &[str_value("ca"), str_value("к")]).is_err());
    // `Очистить` забывает элементы, атрибут уцелевает (измерено).
    sequence_clear(ob(&seq)).expect("очистка");
    assert_eq!(seq.collection_len().expect("длина"), 0);
    assert_eq!(prop(&o, "ca"), BslValue::Undefined);
    assert_eq!(text_of(&prop(&o, "cat")), "атрибут");
}

// --- чтение и запись XML ---------------------------------------------

/// Схема ввода-вывода — та же, что у фикстуры `xdto-xml-io`, и потому
/// ожидания ниже читаются рядом с её эталоном, снятым с платформы.
const IO_SAMPLE: &str = concat!(
    r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:t="urn:test" "#,
    r#"targetNamespace="urn:test" elementFormDefault="qualified" "#,
    r#"attributeFormDefault="unqualified">"#,
    r#"<xs:simpleType name="Codes"><xs:list itemType="xs:int"/></xs:simpleType>"#,
    r#"<xs:complexType name="Inner"><xs:sequence>"#,
    r#"<xs:element name="in" type="xs:string"/></xs:sequence>"#,
    r#"<xs:attribute name="qi" type="xs:string" form="qualified"/></xs:complexType>"#,
    r#"<xs:complexType name="InnerExt"><xs:complexContent>"#,
    r#"<xs:extension base="t:Inner"><xs:sequence>"#,
    r#"<xs:element name="more" type="xs:string"/>"#,
    r#"</xs:sequence></xs:extension></xs:complexContent></xs:complexType>"#,
    r#"<xs:complexType name="RootType"><xs:sequence>"#,
    r#"<xs:element name="name" type="xs:string"/>"#,
    r#"<xs:element name="num" type="xs:int" minOccurs="0"/>"#,
    r#"<xs:element name="dec" type="xs:decimal" minOccurs="0"/>"#,
    r#"<xs:element name="when" type="xs:date" minOccurs="0"/>"#,
    r#"<xs:element name="flag" type="xs:boolean" minOccurs="0"/>"#,
    r#"<xs:element name="bin" type="xs:base64Binary" minOccurs="0"/>"#,
    r#"<xs:element name="codes" type="t:Codes" minOccurs="0"/>"#,
    r#"<xs:element name="tag" type="xs:string" minOccurs="0" maxOccurs="unbounded"/>"#,
    r#"<xs:element name="nested" type="t:Inner" minOccurs="0"/>"#,
    r#"<xs:element name="nilt" type="xs:int" minOccurs="0" nillable="true"/>"#,
    r#"<xs:element name="uq" type="xs:string" minOccurs="0" form="unqualified"/>"#,
    r#"<xs:element name="notype" minOccurs="0"/></xs:sequence>"#,
    r#"<xs:attribute name="id" type="xs:int"/>"#,
    r#"<xs:attribute name="qa" type="xs:string" form="qualified"/></xs:complexType>"#,
    r#"<xs:complexType name="ChoiceType"><xs:choice minOccurs="0" maxOccurs="unbounded">"#,
    r#"<xs:element name="ca" type="xs:string"/>"#,
    r#"<xs:element name="cb" type="xs:string"/></xs:choice></xs:complexType>"#,
    "</xs:schema>",
);

/// Составной документ по этой схеме — тот же, что в фикстуре.
const IO_DOC: &str = concat!(
    r#"<t:root xmlns:t="urn:test" id="7">"#,
    r#"<t:name>аб</t:name><t:num>42</t:num><t:dec>12.50</t:dec>"#,
    r#"<t:when>2026-08-13</t:when><t:flag>true</t:flag>"#,
    r#"<t:bin>0LDQsQ==</t:bin><t:codes>1 2 3</t:codes>"#,
    r#"<t:tag>один</t:tag><t:tag>два</t:tag>"#,
    r#"<t:nested t:qi="х"><t:in>вг</t:in></t:nested>"#,
    r#"<uq>де</uq></t:root>"#,
);

/// Читатель над текстом.
fn reader(text: &str) -> BslValue {
    let value = crate::xml::new_xml_reader();
    crate::xml::set_string(crate::xml::arg_object(&value).unwrap(), &[str_value(text)])
        .expect("источник");
    value
}

/// Писатель в строку.
fn writer() -> BslValue {
    let value = crate::xml::new_xml_writer();
    crate::xml::set_string(crate::xml::arg_object(&value).unwrap(), &[]).expect("приёмник");
    value
}

/// Разбор текста типом схемы.
fn read_with(f: &BslValue, type_name: &str, text: &str) -> RtResult<BslValue> {
    factory_read_xml(ob(f), &[reader(text), type_of_factory(f, type_name)])
}

/// Тип фабрики по имени в `urn:test`.
fn type_of_factory(f: &BslValue, name: &str) -> BslValue {
    factory_type(ob(f), &[str_value("urn:test"), str_value(name)]).expect("тип")
}

/// Запись значения и снятие получившегося текста.
fn write_out(f: &BslValue, value: &BslValue, args: &[&str]) -> RtResult<String> {
    let w = writer();
    let mut call = vec![w.clone(), value.clone()];
    for a in args {
        call.push(str_value(a));
    }
    factory_write_xml(ob(f), &call)?;
    match crate::xml::close_writer(crate::xml::arg_object(&w).unwrap())? {
        BslValue::Str(s) => Ok(s.to_string()),
        other => panic!("писатель отдал не строку: {other:?}"),
    }
}

/// Схема из `measure-xdto-order.bsl`: пять типов одной формы,
/// различающихся ровно масками и наследованием.
const ORDER_SAMPLE: &str = concat!(
    r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:t="urn:test" "#,
    r#"targetNamespace="urn:test" elementFormDefault="qualified">"#,
    r#"<xs:complexType name="Closed"><xs:sequence>"#,
    r#"<xs:element name="a" type="xs:string" minOccurs="0"/>"#,
    r#"<xs:element name="b" type="xs:string" minOccurs="0"/>"#,
    r#"</xs:sequence></xs:complexType>"#,
    r#"<xs:complexType name="OpenElem"><xs:sequence>"#,
    r#"<xs:element name="a" type="xs:string" minOccurs="0"/>"#,
    r#"<xs:element name="b" type="xs:string" minOccurs="0"/>"#,
    r###"<xs:any namespace="##any" processContents="lax" minOccurs="0"/>"###,
    r#"</xs:sequence></xs:complexType>"#,
    r#"<xs:complexType name="OpenAttr"><xs:sequence>"#,
    r#"<xs:element name="a" type="xs:string" minOccurs="0"/>"#,
    r#"<xs:element name="b" type="xs:string" minOccurs="0"/>"#,
    r#"</xs:sequence>"#,
    r###"<xs:anyAttribute namespace="##any" processContents="lax"/>"###,
    r#"</xs:complexType>"#,
    r#"<xs:complexType name="DeepAny"><xs:sequence>"#,
    r#"<xs:element name="a" type="xs:string" minOccurs="0"/>"#,
    r#"<xs:sequence>"#,
    r###"<xs:any namespace="##any" processContents="lax" minOccurs="0"/>"###,
    r#"</xs:sequence>"#,
    r#"<xs:element name="b" type="xs:string" minOccurs="0"/>"#,
    r#"</xs:sequence></xs:complexType>"#,
    r#"<xs:complexType name="ExtOpen"><xs:complexContent>"#,
    r#"<xs:extension base="t:OpenElem"><xs:sequence>"#,
    r#"<xs:element name="d" type="xs:string" minOccurs="0"/>"#,
    r#"</xs:sequence></xs:extension></xs:complexContent></xs:complexType>"#,
    r#"<xs:complexType name="ExtClosed"><xs:complexContent>"#,
    r#"<xs:extension base="t:Closed"><xs:sequence>"#,
    r#"<xs:element name="d" type="xs:string" minOccurs="0"/>"#,
    r#"</xs:sequence></xs:extension></xs:complexContent></xs:complexType>"#,
    r#"<xs:complexType name="ExtAny"><xs:complexContent>"#,
    r#"<xs:extension base="xs:anyType"><xs:sequence>"#,
    r#"<xs:element name="a" type="xs:string" minOccurs="0"/>"#,
    r#"<xs:element name="b" type="xs:string" minOccurs="0"/>"#,
    r#"</xs:sequence></xs:extension></xs:complexContent></xs:complexType>"#,
    r#"</xs:schema>"#,
);

/// Маска делает тип ОТКРЫТЫМ, а открытый пишется в порядке
/// ЗАПОЛНЕНИЯ, тогда как закрытый — в схемном (измерено, якоря
/// `XDTO.WRITE_ORDER.*`; фикстура — xdto-write-order).
///
/// Заполнение везде одно: сначала `b`, потом `a`.
#[test]
fn a_wildcard_opens_the_type_and_switches_the_write_order() {
    let f = factory(ORDER_SAMPLE);
    let filled = |name: &str| {
        let o = factory_create(ob(&f), &[type_of_factory(&f, name)]).expect("экземпляр");
        set_property(ob(&o), "b", str_value("бэ")).expect("b");
        set_property(ob(&o), "a", str_value("а")).expect("a");
        o
    };
    let is_open = |name: &str| prop(&type_of_factory(&f, name), "Открытый");
    let is_sequenced = |name: &str| prop(&type_of_factory(&f, name), "Последовательный");
    // Порядок элементов виден и без разбора разметки: у закрытого
    // типа `a` идёт раньше `b`.
    let schema_order = |name: &str| {
        let text = write_out(&f, &filled(name), &["к"]).expect("запись");
        let at = |tag: &str| text.find(tag).expect("элемент в выгрузке");
        at("<a>") < at("<b>")
    };
    let yes = BslValue::Boolean(true);
    let no = BslValue::Boolean(false);

    // `ExtAny` расширяет `anyType` ЯВНО: сам `anyType` открыт, но
    // открытости не передаёт (иначе открытым стал бы каждый тип —
    // базовый заполнен всегда).
    for name in ["Closed", "ExtClosed", "ExtAny"] {
        assert_eq!(is_open(name), no, "{name}");
        assert_eq!(is_sequenced(name), no, "{name}");
        assert!(schema_order(name), "{name}: ожидался схемный порядок");
    }
    // Любая из двух масок поодиночке, маска на глубине и маска,
    // унаследованная от базового типа.
    for name in ["OpenElem", "OpenAttr", "DeepAny", "ExtOpen"] {
        assert_eq!(is_open(name), yes, "{name}");
        assert_eq!(is_sequenced(name), yes, "{name}");
        assert!(!schema_order(name), "{name}: ожидался порядок заполнения");
    }
    // Упорядоченность маска НЕ трогает: тип остаётся
    // последовательностью.
    assert_eq!(prop(&type_of_factory(&f, "OpenElem"), "Упорядоченный"), yes);
}

#[test]
fn reading_a_composite_document_maps_every_lexical_form() {
    let f = factory(IO_SAMPLE);
    let o = read_with(&f, "RootType", IO_DOC).expect("документ обязан читаться");
    // Значения приходят ТИПИЗИРОВАННЫМИ — это и отличает чтение с
    // типом от чтения без него (измерено: без типа те же свойства
    // остаются строками «12.50» и «true»).
    assert_eq!(text_of(&prop(&o, "name")), "аб");
    assert_eq!(number_of(&prop(&o, "id")), 7);
    assert_eq!(number_of(&prop(&o, "num")), 42);
    assert_eq!(decimal_of(&prop(&o, "dec")), "12.5");
    assert_eq!(prop(&o, "flag"), BslValue::Boolean(true));
    assert_eq!(
        prop(&o, "when").type_of().unwrap(),
        BslValue::Type(TypeRef::Native(TypeId::Date))
    );
    assert_eq!(
        prop(&o, "bin").type_of().unwrap(),
        BslValue::Type(TypeRef::Native(TypeId::BinaryData))
    );
    // Множественное свойство накапливается в списке, вложенное —
    // рекурсивно, и владельцем ему становится родитель.
    let tags = prop(&o, "tag");
    assert_eq!(tags.collection_len().expect("длина"), 2);
    assert_eq!(text_of(&list_get(ob(&tags), 0).expect("первый")), "один");
    assert_eq!(text_of(&list_get(ob(&tags), 1).expect("второй")), "два");
    let nested = prop(&o, "nested");
    assert_eq!(text_of(&prop(&nested, "in")), "вг");
    assert_eq!(text_of(&prop(&nested, "qi")), "х");
    assert_eq!(object_owner(ob(&nested), &[]).expect("владелец"), o);
    // Неквалифицированное свойство ищется по ПУСТОМУ пространству
    // имён, квалифицированное — по целевому.
    assert_eq!(text_of(&prop(&o, "uq")), "де");
    // Списочный простой тип разворачивается в массив значений.
    assert_eq!(prop(&o, "codes").collection_len().expect("длина"), 3);
    // Не встретившееся свойство остаётся НЕзаполненным.
    assert_eq!(
        object_is_set(ob(&o), &[str_value("nilt")]).expect("признак"),
        BslValue::Boolean(false)
    );
}

#[test]
fn reading_rejects_a_document_that_does_not_match_the_type() {
    let f = factory(IO_SAMPLE);
    let bad = [
        // Элемент, которого у типа нет.
        r#"<t:root xmlns:t="urn:test"><t:name>а</t:name><t:чужое>б</t:чужое></t:root>"#,
        // Атрибут, которого у типа нет.
        r#"<t:root xmlns:t="urn:test" чужой="1"><t:name>а</t:name></t:root>"#,
        // Лексическая форма не разбирается объявленным типом.
        r#"<t:root xmlns:t="urn:test"><t:name>а</t:name><t:num>ab</t:num></t:root>"#,
        // Порядок в типе-последовательности нарушен.
        r#"<t:root xmlns:t="urn:test"><t:num>42</t:num><t:name>а</t:name></t:root>"#,
        // Одиночное свойство встретилось дважды.
        r#"<t:root xmlns:t="urn:test"><t:name>а</t:name><t:name>б</t:name></t:root>"#,
        // Множественное свойство разорвано соседним элементом.
        concat!(
            r#"<t:root xmlns:t="urn:test"><t:name>а</t:name><t:tag>1</t:tag>"#,
            r#"<t:nested><t:in>в</t:in></t:nested><t:tag>2</t:tag></t:root>"#,
        ),
        // Значащий текст в составном типе.
        r#"<t:root xmlns:t="urn:test">мусор<t:name>а</t:name></t:root>"#,
        // Обязательного свойства нет вовсе.
        r#"<t:root xmlns:t="urn:test"/>"#,
        // Чужой корень читается тем же типом и спотыкается на том же.
        r#"<чужой xmlns="urn:иное"/>"#,
    ];
    for text in bad {
        let err = read_with(&f, "RootType", text);
        assert!(
            matches!(err, Err(RtError::Xdto(_))),
            "документ обязан быть отвергнут: {text}"
        );
    }
    // А необязательное свойство пропускать можно.
    assert!(
        read_with(
            &f,
            "RootType",
            r#"<t:root xmlns:t="urn:test"><t:name>а</t:name><t:flag>true</t:flag></t:root>"#,
        )
        .is_ok()
    );
}

#[test]
fn reading_without_a_type_refuses_instead_of_guessing() {
    let f = factory(IO_SAMPLE);
    // Платформа читает такой вызов в ОТКРЫТОЕ содержимое `anyType`;
    // здесь его нет, и отказ честнее подмены разбором по схеме.
    let err = factory_read_xml(ob(&f), &[reader(IO_DOC)]);
    assert!(matches!(err, Err(RtError::Xdto(_))));
    // Тип обязан быть типом, а источник — читателем.
    assert!(factory_read_xml(ob(&f), &[reader(IO_DOC), str_value("RootType")]).is_err());
    assert!(factory_read_xml(ob(&f), &[str_value(IO_DOC)]).is_err());
}

#[test]
fn reading_leaves_the_reader_on_the_next_node() {
    let f = factory(IO_SAMPLE);
    let r = reader(concat!(
        r#"<об xmlns:t="urn:test"><t:root id="1"><t:name>а</t:name></t:root>"#,
        r#"<t:root id="2"><t:name>б</t:name></t:root><хвост/></об>"#,
    ));
    // Обёртку читатель проходит сам, а дальше два элемента подряд
    // читаются двумя вызовами БЕЗ `Прочитать()` между ними —
    // измерено, что после разбора читатель стоит на следующем узле.
    crate::xml::read(crate::xml::arg_object(&r).unwrap()).expect("обёртка");
    crate::xml::read(crate::xml::arg_object(&r).unwrap()).expect("первый корень");
    let t = type_of_factory(&f, "RootType");
    let first = factory_read_xml(ob(&f), &[r.clone(), t.clone()]).expect("первый");
    assert_eq!(
        text_of(&crate::xml::name(crate::xml::arg_object(&r).unwrap()).expect("имя")),
        "t:root"
    );
    let second = factory_read_xml(ob(&f), &[r.clone(), t]).expect("второй");
    assert_eq!(number_of(&prop(&first, "id")), 1);
    assert_eq!(number_of(&prop(&second, "id")), 2);
    assert_eq!(
        text_of(&crate::xml::name(crate::xml::arg_object(&r).unwrap()).expect("имя")),
        "хвост"
    );
    // На документе из одного корня читатель после разбора исчерпан.
    let single = reader(IO_DOC);
    let t = type_of_factory(&f, "RootType");
    factory_read_xml(ob(&f), &[single.clone(), t]).expect("корень");
    assert_eq!(
        crate::xml::read(crate::xml::arg_object(&single).unwrap()).expect("шаг"),
        BslValue::Boolean(false)
    );
}

#[test]
fn nil_and_xsi_type_survive_the_round_trip() {
    let f = factory(IO_SAMPLE);
    // `xsi:nil` заполняет свойство ПУСТЫМ значением и обратно
    // пишется тем же атрибутом (измерено).
    let o = read_with(
        &f,
        "RootType",
        concat!(
            r#"<t:root xmlns:t="urn:test" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">"#,
            r#"<t:name>а</t:name><t:nilt xsi:nil="true"/></t:root>"#,
        ),
    )
    .expect("документ с nil");
    assert_eq!(prop(&o, "nilt"), BslValue::Undefined);
    assert_eq!(
        object_is_set(ob(&o), &[str_value("nilt")]).expect("признак"),
        BslValue::Boolean(true)
    );
    assert!(
        write_out(&f, &o, &["к"])
            .expect("запись")
            .contains(r#"<nilt xsi:nil="true"/>"#)
    );
    // `xsi:type` у СВОЙСТВА выбирает наследника, а неизвестное имя
    // платформа игнорирует (измерено обе стороны).
    let derived = read_with(
        &f,
        "RootType",
        concat!(
            r#"<t:root xmlns:t="urn:test" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">"#,
            r#"<t:name>а</t:name><t:nested xsi:type="t:InnerExt">"#,
            r#"<t:in>в</t:in><t:more>г</t:more></t:nested></t:root>"#,
        ),
    )
    .expect("документ с xsi:type");
    let nested = prop(&derived, "nested");
    assert_eq!(
        object_type(ob(&nested), &[]).expect("тип").to_string(),
        "{urn:test}InnerExt"
    );
    assert!(
        write_out(&f, &derived, &["к"])
            .expect("запись")
            .contains(r#"<nested xsi:type="InnerExt">"#)
    );
    let unknown = read_with(
        &f,
        "Inner",
        concat!(
            r#"<точка xmlns="urn:test" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" "#,
            r#"xsi:type="НетТакого"><in>в</in></точка>"#,
        ),
    )
    .expect("неизвестный xsi:type игнорируется");
    assert_eq!(
        object_type(ob(&unknown), &[]).expect("тип").to_string(),
        "{urn:test}Inner"
    );
}

#[test]
fn writing_follows_the_measured_lexical_forms() {
    let f = factory(IO_SAMPLE);
    let o = factory_create(ob(&f), &[type_of_factory(&f, "RootType")]).expect("экземпляр");
    set_property(ob(&o), "name", str_value("аб")).expect("строка");
    set_property(ob(&o), "dec", str_value("12.50")).expect("дробное");
    set_property(ob(&o), "when", str_value("2026-08-13")).expect("дата");
    set_property(ob(&o), "flag", BslValue::Boolean(true)).expect("булево");
    set_property(ob(&o), "bin", str_value("0LDQsQ==")).expect("двоичное");
    let text = write_out(&f, &o, &["к"]).expect("запись");
    // Хвостовой ноль срезан, дата без времени, булево словом,
    // двоичное — base64 (всё измерено).
    assert!(text.contains("<dec>12.5</dec>"), "{text}");
    assert!(text.contains("<when>2026-08-13</when>"), "{text}");
    assert!(text.contains("<flag>true</flag>"), "{text}");
    assert!(text.contains("<bin>0LDQsQ==</bin>"), "{text}");
    // Объявления на элементе, с которого начинается запись: своё
    // умолчательное плюс `xs` и `xsi`.
    assert!(
        text.starts_with(concat!(
            r#"<к xmlns="urn:test" xmlns:xs="http://www.w3.org/2001/XMLSchema" "#,
            r#"xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">"#,
        )),
        "{text}"
    );
    // Имя по умолчанию — имя ТИПА, а не объявленного элемента.
    assert!(
        write_out(&f, &o, &[])
            .expect("запись")
            .starts_with("<RootType ")
    );
    // Пустая строка даёт схлопнутый элемент, а не пару тегов.
    set_property(ob(&o), "name", str_value("")).expect("пустая строка");
    assert!(
        write_out(&f, &o, &["к"])
            .expect("запись")
            .contains("<name/>")
    );
    // Записывать можно только экземпляры.
    assert!(factory_write_xml(ob(&f), &[writer(), number_value(5)]).is_err());
    // Пустое имя платформа отвергает — и здесь тоже.
    assert!(factory_write_xml(ob(&f), &[writer(), o.clone(), str_value("")]).is_err());
}

#[test]
fn write_rejects_element_name_with_colon() {
    let f = factory(IO_SAMPLE);
    let o = factory_create(ob(&f), &[type_of_factory(&f, "RootType")]).expect("экземпляр");
    set_property(ob(&o), "name", str_value("аб")).expect("строка");
    // Измерено (`зп имя с двоеточием`): `ЗаписатьXML(Зпис, ОбП,
    // "t:мой")` на 8.3.27 — ошибка, а не документ с префиксом `t`,
    // который никто не объявлял.
    assert!(
        matches!(write_out(&f, &o, &["t:мой"]), Err(RtError::Xdto(_))),
        "имя с двоеточием обязано быть отвергнуто"
    );
    // Имя без двоеточия по-прежнему пишется.
    assert!(
        write_out(&f, &o, &["мой"])
            .expect("запись")
            .starts_with("<мой ")
    );
}

/// Тесты на предел глубины гоняются в потоке со стеком главного
/// (8 МиБ): предел калиброван под него, а libtest даёт тестовому
/// потоку 2 МиБ, и запись на полной глубине туда честно не помещается
/// — кадр спуска в debug-сборке стоит около 8 КиБ, то есть все
/// `MAX_XDTO_DEPTH` уровней съедают порядка 4 МиБ.
fn on_main_sized_stack(body: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(body)
        .expect("поток не создался")
        .join()
        .expect("тест в потоке упал");
}

#[test]
fn write_of_cyclic_instance_is_a_catchable_error() {
    on_main_sized_stack(|| {
        let f = factory(IO_SAMPLE);
        let o = factory_create(ob(&f), &[type_of_factory(&f, "RootType")]).expect("экземпляр");
        set_property(ob(&o), "name", str_value("аб")).expect("строка");
        // `notype` объявлен без типа, то есть `anyType`, и принимает
        // любое значение — в том числе своего же владельца. Такой цикл
        // обязан упереться в предел глубины и вернуться ошибкой, а не
        // уронить стек процесса.
        set_property(ob(&o), "notype", o.clone()).expect("ссылка на себя");
        assert!(
            matches!(write_out(&f, &o, &["к"]), Err(RtError::Xdto(_))),
            "циклический экземпляр обязан давать перехватываемую ошибку"
        );
    });
}

#[test]
fn write_deeper_than_limit_is_a_catchable_error() {
    on_main_sized_stack(|| {
        let f = factory(IO_SAMPLE);
        let root = factory_create(ob(&f), &[type_of_factory(&f, "RootType")]).expect("экземпляр");
        set_property(ob(&root), "name", str_value("аб")).expect("строка");
        // Честная цепочка без цикла, но глубже предела: спуск записи
        // рекурсивный, и ограничение у него то же, что у разбора.
        let mut current = root.clone();
        for _ in 0..MAX_XDTO_DEPTH + 5 {
            let next =
                factory_create(ob(&f), &[type_of_factory(&f, "RootType")]).expect("экземпляр");
            set_property(ob(&next), "name", str_value("аб")).expect("строка");
            set_property(ob(&current), "notype", next.clone()).expect("вложение");
            current = next;
        }
        assert!(
            matches!(write_out(&f, &root, &["к"]), Err(RtError::Xdto(_))),
            "цепочка глубже предела обязана давать перехватываемую ошибку"
        );
    });
}

#[test]
fn writing_orders_properties_by_the_type_or_by_the_filling() {
    let f = factory(IO_SAMPLE);
    // У УПОРЯДОЧЕННОГО типа порядок модельный, независимо от того, в
    // каком порядке свойства заполняли (измерено).
    let o = factory_create(ob(&f), &[type_of_factory(&f, "RootType")]).expect("экземпляр");
    set_property(ob(&o), "num", number_value(42)).expect("число");
    set_property(ob(&o), "name", str_value("аб")).expect("строка");
    set_property(ob(&o), "id", number_value(7)).expect("атрибут");
    assert_eq!(
        write_out(&f, &o, &["к"]).expect("запись"),
        concat!(
            r#"<к xmlns="urn:test" xmlns:xs="http://www.w3.org/2001/XMLSchema" "#,
            "xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" id=\"7\">\n",
            "\t<name>аб</name>\n\t<num>42</num>\n</к>",
        )
    );
    // У ПОСЛЕДОВАТЕЛЬНОГО (`xs:choice`) — порядок заполнения.
    let c = factory_create(ob(&f), &[type_of_factory(&f, "ChoiceType")]).expect("экземпляр");
    list_add(ob(&prop(&c, "cb")), &[str_value("б1")]).expect("добавление");
    list_add(ob(&prop(&c, "ca")), &[str_value("а1")]).expect("добавление");
    list_add(ob(&prop(&c, "cb")), &[str_value("б2")]).expect("добавление");
    let text = write_out(&f, &c, &["в"]).expect("запись");
    assert!(
        text.contains("\t<cb>б1</cb>\n\t<ca>а1</ca>\n\t<cb>б2</cb>\n"),
        "{text}"
    );
}

#[test]
fn writing_declares_namespaces_the_way_the_platform_does() {
    let f = factory(IO_SAMPLE);
    let o = factory_create(ob(&f), &[type_of_factory(&f, "RootType")]).expect("экземпляр");
    set_property(ob(&o), "name", str_value("аб")).expect("строка");
    set_property(ob(&o), "uq", str_value("де")).expect("неквалифицированное");
    // Неквалифицированное свойство отменяет умолчательное объявление.
    assert!(
        write_out(&f, &o, &["к"])
            .expect("запись")
            .contains(r#"<uq xmlns="">де</uq>"#)
    );
    // Квалифицированному АТРИБУТУ умолчательное объявление не годится
    // — ему заводится префикс `d<глубина>p<номер>`, а сам элемент
    // остаётся на умолчании (измерено).
    set_property(ob(&o), "qa", str_value("де")).expect("атрибут");
    let text = write_out(&f, &o, &["к"]).expect("запись");
    assert!(
        text.starts_with(r#"<к xmlns="urn:test" xmlns:d1p1="urn:test" "#),
        "{text}"
    );
    assert!(text.contains(r#" d1p1:qa="де""#), "{text}");
    // На втором уровне номер глубины другой, и там уже сам элемент
    // уходит под префикс, а за ним и его дети (измерено).
    let o2 = factory_create(ob(&f), &[type_of_factory(&f, "RootType")]).expect("экземпляр");
    set_property(ob(&o2), "name", str_value("аб")).expect("строка");
    let inner = factory_create(ob(&f), &[type_of_factory(&f, "Inner")]).expect("вложенный");
    set_property(ob(&inner), "in", str_value("вг")).expect("строка");
    set_property(ob(&inner), "qi", str_value("х")).expect("атрибут");
    set_property(ob(&o2), "nested", inner).expect("вложение");
    let text = write_out(&f, &o2, &["к"]).expect("запись");
    assert!(
        text.contains(r#"<d2p1:nested xmlns:d2p1="urn:test" d2p1:qi="х">"#),
        "{text}"
    );
    assert!(text.contains("<d2p1:in>вг</d2p1:in>"), "{text}");
    // Чужой URI элемента заставляет объявиться и свойства.
    let o3 = factory_create(ob(&f), &[type_of_factory(&f, "RootType")]).expect("экземпляр");
    set_property(ob(&o3), "name", str_value("аб")).expect("строка");
    let text = write_out(&f, &o3, &["мой", "urn:иное"]).expect("запись");
    assert!(text.starts_with(r#"<мой xmlns="urn:иное" "#), "{text}");
    assert!(
        text.contains(r#"<name xmlns="urn:test">аб</name>"#),
        "{text}"
    );
}

#[test]
fn any_type_properties_carry_the_type_of_their_value() {
    let f = factory(IO_SAMPLE);
    let o = factory_create(ob(&f), &[type_of_factory(&f, "RootType")]).expect("экземпляр");
    set_property(ob(&o), "name", str_value("аб")).expect("строка");
    // Свойство типа `anyType` принимает что угодно, и запись
    // помечает элемент типом ЗНАЧЕНИЯ (измерено поимённо).
    for (value, marked) in [
        (
            number_value(5),
            r#"<notype xsi:type="xs:decimal">5</notype>"#,
        ),
        (
            BslValue::Boolean(true),
            r#"<notype xsi:type="xs:boolean">true</notype>"#,
        ),
        (
            str_value("аб"),
            r#"<notype xsi:type="xs:string">аб</notype>"#,
        ),
    ] {
        set_property(ob(&o), "notype", value).expect("любое значение");
        let text = write_out(&f, &o, &["к"]).expect("запись");
        assert!(text.contains(marked), "{text}");
    }
    // Обратно: текст без пометки читается СТРОКОЙ, с пометкой —
    // значением помеченного типа.
    let plain = read_with(
        &f,
        "RootType",
        r#"<t:root xmlns:t="urn:test"><t:name>а</t:name><t:notype>5</t:notype></t:root>"#,
    )
    .expect("без пометки");
    assert_eq!(text_of(&prop(&plain, "notype")), "5");
    let typed = read_with(
        &f,
        "RootType",
        concat!(
            r#"<t:root xmlns:t="urn:test" xmlns:xs="http://www.w3.org/2001/XMLSchema" "#,
            r#"xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">"#,
            r#"<t:name>а</t:name><t:notype xsi:type="xs:decimal">5</t:notype></t:root>"#,
        ),
    )
    .expect("с пометкой");
    assert_eq!(number_of(&prop(&typed, "notype")), 5);
}

#[test]
fn the_round_trip_is_stable_after_the_first_pass() {
    let f = factory(IO_SAMPLE);
    let first = read_with(&f, "RootType", IO_DOC).expect("разбор");
    let once = write_out(&f, &first, &["root", "urn:test"]).expect("запись");
    // Второй круг обязан дать ТО ЖЕ САМОЕ: лексические формы уже
    // канонические (у платформы измерено то же — «Да»).
    let second = read_with(&f, "RootType", &once).expect("повторный разбор");
    let twice = write_out(&f, &second, &["root", "urn:test"]).expect("повторная запись");
    assert_eq!(once, twice);
    // Первый круг уже канонизует запись: «12.50» становится «12.5».
    assert!(once.contains("<dec>12.5</dec>"), "{once}");
    assert!(once.contains("<codes>1 2 3</codes>"), "{once}");
    assert!(once.contains(r#"<uq xmlns="">де</uq>"#), "{once}");
}

#[test]
fn values_of_simple_types_read_and_write_as_values() {
    let f = factory(IO_SAMPLE);
    let xsd = factory_type(
        ob(&f),
        &[
            str_value("http://www.w3.org/2001/XMLSchema"),
            str_value("int"),
        ],
    )
    .expect("тип");
    let value = factory_read_xml(ob(&f), &[reader("<число>42</число>"), xsd.clone()])
        .expect("значение обязано читаться");
    assert_eq!(number_of(&prop(&value, "Значение")), 42);
    assert_eq!(text_of(&prop(&value, "ЛексическоеЗначение")), "42");
    // Обратно значение пишется своим типом, а пространство XML Schema
    // становится умолчательным, поэтому `xs` не объявляется.
    assert_eq!(
        write_out(&f, &value, &[]).expect("запись"),
        concat!(
            r#"<int xmlns="http://www.w3.org/2001/XMLSchema" "#,
            r#"xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">42</int>"#,
        )
    );
    // Посторонний атрибут у элемента простого типа платформа
    // отвергает (измерено), и битая лексика — тоже.
    assert!(factory_read_xml(ob(&f), &[reader(r#"<ч а="1">42</ч>"#), xsd.clone()]).is_err());
    assert!(factory_read_xml(ob(&f), &[reader("<ч>ab</ч>"), xsd]).is_err());
}

#[test]
fn base64_wraps_at_the_measured_width() {
    // 48 байт дают ровно 64 символа и переноса не получают, 49 —
    // получают, и разделитель именно CR LF (измерено).
    let short = vec![b'0'; 48];
    let long = vec![b'0'; 49];
    assert_eq!(encode_base64(&short).len(), 64);
    assert!(!encode_base64(&short).contains('\r'));
    assert!(encode_base64(&long).contains("\r\n"));
    assert_eq!(encode_base64(&long).lines().count(), 2);
    assert_eq!(encode_base64(&[]), "");
    // Круг с разборщиком: перенос при чтении игнорируется.
    assert_eq!(decode_base64(&encode_base64(&long)), Some(long));
    // `hexBinary` пишется ЗАГЛАВНЫМИ и одной строкой (измерено).
    assert_eq!(encode_hex(&[0xd0, 0xb0, 0xd0, 0xb1]), "D0B0D0B1");
    assert_eq!(
        decode_hex(&encode_hex(&[0x0f, 0xff])),
        Some(vec![0x0f, 0xff])
    );
}

#[test]
fn an_empty_lexical_form_is_zero_only_for_numbers() {
    let m = model(IO_SAMPLE);
    // Измерено: `Создать(xs:int, "")` даёт 0, а пустой элемент числа
    // читается тем же нулём; у даты и булева пустая запись — ошибка.
    let int = m.find(XSD_NS, "int").expect("int");
    assert_eq!(
        decimal_of(&value_from_lexical(&m, int, "").expect("ноль")),
        "0"
    );
    let date = m.find(XSD_NS, "date").expect("date");
    assert!(value_from_lexical(&m, date, "").is_err());
    let boolean = m.find(XSD_NS, "boolean").expect("boolean");
    assert!(value_from_lexical(&m, boolean, "").is_err());
    // А `xs:double` поблажки не получил: измерено, что и
    // `Создать(xs:double, "")`, и пустой элемент этого типа платформа
    // отвергает (`чв Создать пустой dbl`, `чв пустое dbl`).
    let double = m.find(XSD_NS, "double").expect("double");
    assert!(value_from_lexical(&m, double, "").is_err());
}

// --- СериализаторXDTO ---------------------------------------------

/// Сериализатор над фабрикой из `IO_SAMPLE`. Схема тут ни при чём —
/// примитивы идут встроенными типами XML Schema, которые есть у любой
/// модели, — но фабрика конструктору обязательна.
fn serializer() -> BslValue {
    serializer_of_factory(&factory(IO_SAMPLE)).expect("сериализатор")
}

/// Запись значения сериализатором и снятие текста.
fn serialize(value: &BslValue, args: &[BslValue]) -> RtResult<String> {
    let w = writer();
    let mut call = vec![w.clone(), value.clone()];
    call.extend(args.iter().cloned());
    serializer_write_xml(ob(&serializer()), &call)?;
    match crate::xml::close_writer(crate::xml::arg_object(&w).unwrap())? {
        BslValue::Str(s) => Ok(s.to_string()),
        other => panic!("писатель отдал не строку: {other:?}"),
    }
}

/// Обратное чтение текста сериализатором.
fn deserialize(text: &str, args: &[BslValue]) -> RtResult<BslValue> {
    let mut call = vec![reader(text)];
    call.extend(args.iter().cloned());
    serializer_read_xml(ob(&serializer()), &call)
}

/// Дата как значение — тем же путём, что и разбор лексики.
fn date_value(y: i64, m: u32, d: u32, hh: u32, mm: u32, ss: u32) -> BslValue {
    BslValue::Date(bsl_rt::BslDate::from_civil(y, m, d, hh, mm, ss).expect("дата"))
}

/// `ДвоичныеДанные` из байтов.
fn binary_value(bytes: &[u8]) -> BslValue {
    BslValue::Object(Rc::new(BslObject::BinaryData(Rc::from(bytes))))
}

#[test]
fn xdto_serializer_needs_a_factory() {
    // Измерено: конструктор берёт ТОЛЬКО фабрику — ни строки, ни
    // числа, ни типа XDTO, ни пустого вызова.
    assert!(serializer_of_factory(&factory(IO_SAMPLE)).is_ok());
    assert!(serializer_of_factory(&BslValue::Undefined).is_err());
    assert!(serializer_of_factory(&str_value("urn:test")).is_err());
    assert!(serializer_of_factory(&number_value(5)).is_err());
    let f = factory(IO_SAMPLE);
    assert!(serializer_of_factory(&type_of_factory(&f, "RootType")).is_err());
    // Два сериализатора над одной фабрикой не равны (измерено «Нет»),
    // а сам он печатается своим именем типа.
    let a = serializer_of_factory(&f).expect("первый");
    let b = serializer_of_factory(&f).expect("второй");
    assert!(a != b);
    assert_eq!(a.to_string(), "СериализаторXDTO");
    assert_eq!(
        a.type_of().expect("ТипЗнч"),
        BslValue::Type(TypeRef::Object(&crate::xdto::objects::SERIALIZER_TYPE))
    );
}

#[test]
fn xdto_serializer_writes_the_measured_element_for_every_primitive() {
    // Каждая строка снята с 8.3.27 (`measure-xdto-serializer.bsl`,
    // пробы `зап …`) через фабрику из ФАЙЛА XSD, то есть без единого
    // типа конфигурации.
    let xsd = r#" xmlns="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance""#;
    let both = r#" xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance""#;
    assert_eq!(
        serialize(&number_value(42), &[]).expect("число"),
        format!("<decimal{xsd}>42</decimal>")
    );
    assert_eq!(
        serialize(&str_value("аб"), &[]).expect("строка"),
        format!("<string{xsd}>аб</string>")
    );
    // Пустая лексическая форма схлопывает элемент — то же правило, что
    // у записи фабрики.
    assert_eq!(
        serialize(&str_value(""), &[]).expect("пустая строка"),
        format!("<string{xsd}/>")
    );
    assert_eq!(
        serialize(&date_value(2026, 8, 13, 10, 20, 30), &[]).expect("дата"),
        format!("<dateTime{xsd}>2026-08-13T10:20:30</dateTime>")
    );
    assert_eq!(
        serialize(&BslValue::Boolean(true), &[]).expect("истина"),
        format!("<boolean{xsd}>true</boolean>")
    );
    assert_eq!(
        serialize(&BslValue::Boolean(false), &[]).expect("ложь"),
        format!("<boolean{xsd}>false</boolean>")
    );
    assert_eq!(
        serialize(&binary_value(&[0xD0, 0xB0, 0xD0, 0xB1]), &[]).expect("двоичные"),
        format!("<base64Binary{xsd}>0LDQsQ==</base64Binary>")
    );
    // У этих двух пространство имён СВОЁ, и оба объявления `xs` и
    // `xsi` появляются, потому что умолчательным стал не XML Schema.
    assert_eq!(
        serialize(&BslValue::Undefined, &[]).expect("неопределено"),
        format!(r#"<Undefined xmlns=""{both} xsi:nil="true"/>"#)
    );
    assert_eq!(
        serialize(&BslValue::Null, &[]).expect("null"),
        format!(r#"<Null xmlns="{V8_CORE_NS}"{both}/>"#)
    );
}

#[test]
fn xdto_serializer_round_trips_every_supported_primitive() {
    // Круг замыкается лексикой: записанное читается обратно тем же
    // значением (измерено — `круг число`, `круг строка`, `круг дата`,
    // `круг булево`, `круг неопределено`, `круг двоичные`).
    for value in [
        number_value(42),
        BslValue::Number(BslNumber::parse_canonical("12.5").expect("12.5")),
        str_value("аб"),
        str_value(""),
        date_value(2026, 8, 13, 10, 20, 30),
        date_value(2026, 8, 13, 0, 0, 0),
        BslValue::Boolean(true),
        BslValue::Boolean(false),
        BslValue::Undefined,
        binary_value(&[0xD0, 0xB0, 0xD0, 0xB1]),
    ] {
        let text = serialize(&value, &[]).expect("запись");
        let back = deserialize(&text, &[]).expect("чтение");
        assert_eq!(back, value, "круг по {text}");
    }
    // А `Null` кругом НЕ замыкается, и это измерено: `<Null/>`
    // пространства базовых типов 1С в модели фабрики из XSD не
    // значится, поэтому обратно приходит пустая строка («круг NULL» ->
    // `Строка []`).
    let text = serialize(&BslValue::Null, &[]).expect("запись null");
    assert_eq!(deserialize(&text, &[]).expect("чтение null"), str_value(""));
}

#[test]
fn xdto_serializer_takes_the_element_name_and_uri_from_the_arguments() {
    let both = r#" xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance""#;
    // Заданное имя сбрасывает пространство имён в ПУСТОЕ, если URI не
    // назван, — не в пространство типа (измерено).
    assert_eq!(
        serialize(&number_value(42), &[str_value("мой")]).expect("имя"),
        format!(r#"<мой xmlns=""{both}>42</мой>"#)
    );
    assert_eq!(
        serialize(
            &number_value(42),
            &[str_value("мой"), str_value("urn:иное")]
        )
        .expect("имя и URI"),
        format!(r#"<мой xmlns="urn:иное"{both}>42</мой>"#)
    );
    // Пустое имя и `Неопределено` вместо него — это «имя по
    // умолчанию», а не ошибка, и пустое имя отбрасывает ЗАОДНО и URI
    // (измерено обе пробы).
    let default = concat!(
        r#"<decimal xmlns="http://www.w3.org/2001/XMLSchema" "#,
        r#"xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">42</decimal>"#
    );
    assert_eq!(
        serialize(&number_value(42), &[str_value("")]).expect("пустое имя"),
        default
    );
    assert_eq!(
        serialize(&number_value(42), &[BslValue::Undefined]).expect("имя Неопределено"),
        default
    );
    assert_eq!(
        serialize(&number_value(42), &[str_value(""), str_value("urn:иное")])
            .expect("пустое имя с URI"),
        default
    );
    // URI `Неопределено` при заданном имени — это пустой URI.
    assert_eq!(
        serialize(&number_value(42), &[str_value("мой"), BslValue::Undefined])
            .expect("URI Неопределено"),
        format!(r#"<мой xmlns=""{both}>42</мой>"#)
    );
    // Имя с двоеточием сериализатор пишет КАК ЕСТЬ — там, где фабрика
    // его отвергает (измерено обе стороны).
    assert_eq!(
        serialize(&number_value(42), &[str_value("t:мой")]).expect("имя с двоеточием"),
        format!(r#"<t:мой xmlns=""{both}>42</t:мой>"#)
    );
    // Нестроковые имя и URI — ошибка, как на платформе.
    assert!(serialize(&number_value(42), &[number_value(5)]).is_err());
    assert!(serialize(&number_value(42), &[str_value("мой"), number_value(5)]).is_err());
    // Арность: одного аргумента мало, пятого не бывает.
    let s = serializer();
    assert!(serializer_write_xml(ob(&s), &[writer()]).is_err());
    assert!(
        serializer_write_xml(
            ob(&s),
            &[
                writer(),
                number_value(42),
                str_value("м"),
                str_value("u"),
                str_value("лишний"),
            ]
        )
        .is_err()
    );
}

#[test]
fn xdto_serializer_reads_by_element_name_and_falls_back_to_a_string() {
    let xsd = r#" xmlns="http://www.w3.org/2001/XMLSchema""#;
    // Имя элемента — это расширенное имя типа в модели.
    assert_eq!(
        deserialize(&format!("<decimal{xsd}>42</decimal>"), &[]).expect("decimal"),
        number_value(42)
    );
    assert_eq!(
        deserialize(&format!("<int{xsd}>42</int>"), &[]).expect("int"),
        number_value(42)
    );
    // Пустая лексическая форма числа — ноль (измерено).
    assert_eq!(
        deserialize(&format!("<decimal{xsd}/>"), &[]).expect("пустой decimal"),
        number_value(0)
    );
    // Цифровая запись булева — та же, что у `xs:boolean` вообще.
    assert_eq!(
        deserialize(&format!("<boolean{xsd}>1</boolean>"), &[]).expect("boolean"),
        BslValue::Boolean(true)
    );
    // Текст простого элемента сохраняется как есть, включая краевые
    // пробелы (измерено `[ аб ]`).
    assert_eq!(
        deserialize(&format!("<string{xsd}> аб </string>"), &[]).expect("string"),
        str_value(" аб ")
    );
    // Всё, чему в модели типа не нашлось, читается СТРОКОЙ: и чужое
    // имя, и то же имя без пространства имён, и элементы пакета
    // базовых типов 1С, которых в фабрике из XSD нет.
    assert_eq!(
        deserialize("<чужой>привет</чужой>", &[]).expect("чужой"),
        str_value("привет")
    );
    assert_eq!(
        deserialize("<decimal>42</decimal>", &[]).expect("без пространства"),
        str_value("42")
    );
    assert_eq!(
        deserialize(
            &format!(r#"<UUID xmlns="{V8_CORE_NS}">01234567-89ab-cdef-0123-456789abcdef</UUID>"#),
            &[]
        )
        .expect("UUID"),
        str_value("01234567-89ab-cdef-0123-456789abcdef")
    );
    // `xsi:nil` перевешивает имя — и у известного типа, и у чужого.
    let nil = r#" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:nil="true""#;
    assert_eq!(
        deserialize(&format!("<decimal{xsd}{nil}/>"), &[]).expect("nil у decimal"),
        BslValue::Undefined
    );
    assert_eq!(
        deserialize(&format!("<чужой{nil}/>"), &[]).expect("nil у чужого"),
        BslValue::Undefined
    );
    // А без пометки тот же `<Undefined/>` — просто пустая строка.
    assert_eq!(
        deserialize(r#"<Undefined xmlns=""/>"#, &[]).expect("Undefined без nil"),
        str_value("")
    );
}

#[test]
fn xdto_serializer_honours_xsi_type_on_the_element_it_reads() {
    // У сериализатора пометка действует ПРЯМО на читаемом элементе —
    // в отличие от `ФабрикаXDTO.ПрочитатьXML`, где на нём она не
    // значит ничего (измерено обе стороны).
    let head = concat!(
        r#" xmlns:xs="http://www.w3.org/2001/XMLSchema""#,
        r#" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance""#
    );
    assert_eq!(
        deserialize(
            &format!(r#"<мой{head} xsi:type="xs:decimal">42</мой>"#),
            &[]
        )
        .expect("xsi:type"),
        number_value(42)
    );
    // Неизвестное имя типа платформа ИГНОРИРУЕТ, а не ругается.
    assert_eq!(
        deserialize(&format!(r#"<мой{head} xsi:type="xs:чепуха">42</мой>"#), &[])
            .expect("неизвестный xsi:type"),
        str_value("42")
    );
}

#[test]
fn xdto_serializer_reads_by_an_explicit_bsl_type() {
    // Заданный тип отменяет имя элемента: `<int>` с `Тип("Число")`
    // разбирается как `xs:decimal` и даёт 42 (измерено).
    assert_eq!(
        deserialize(
            "<что>42</что>",
            &[BslValue::Type(TypeRef::Native(TypeId::Number))]
        )
        .expect("число"),
        number_value(42)
    );
    assert_eq!(
        deserialize(
            "<когда>2026-08-13T10:20:30</когда>",
            &[BslValue::Type(TypeRef::Native(TypeId::Date))]
        )
        .expect("дата"),
        date_value(2026, 8, 13, 10, 20, 30)
    );
    assert_eq!(
        deserialize(
            "<флаг>true</флаг>",
            &[BslValue::Type(TypeRef::Native(TypeId::Boolean))]
        )
        .expect("булево"),
        BslValue::Boolean(true)
    );
    assert_eq!(
        deserialize(
            "<что>аб</что>",
            &[BslValue::Type(TypeRef::Native(TypeId::String))]
        )
        .expect("строка"),
        str_value("аб")
    );
    assert_eq!(
        deserialize(
            "<что>0LDQsQ==</что>",
            &[BslValue::Type(TypeRef::Native(TypeId::BinaryData))]
        )
        .expect("двоичные"),
        binary_value(&[0xD0, 0xB0, 0xD0, 0xB1])
    );
    // Отображения нет — платформа отвечает «Отсутствует отображение
    // для типа», и здесь то же (измерено на трёх типах).
    for id in [TypeId::Array, TypeId::Null, TypeId::Undefined] {
        let err = deserialize("<м/>", &[BslValue::Type(TypeRef::Native(id))])
            .expect_err("нет отображения");
        assert!(err.to_string().contains("отсутствует отображение"), "{err}");
    }
    // Лексическая форма проверяется и при заданном типе.
    assert!(
        deserialize(
            "<что>аб</что>",
            &[BslValue::Type(TypeRef::Native(TypeId::Number))]
        )
        .is_err()
    );
    // Второй аргумент — именно значение `Тип`, а не тип XDTO.
    let f = factory(IO_SAMPLE);
    assert!(deserialize("<м/>", &[type_of_factory(&f, "RootType")]).is_err());
    // Третьего аргумента у чтения нет.
    assert!(
        deserialize(
            "<м/>",
            &[
                BslValue::Type(TypeRef::Native(TypeId::Number)),
                str_value("лишний")
            ]
        )
        .is_err()
    );
}

#[test]
fn xdto_serializer_refuses_what_needs_configuration_metadata() {
    // Коллекции 1С платформа пишет типами пакета
    // `http://v8.1c.ru/8.1/data/core`: глобальный сериализатор выдаёт
    // `<Array xmlns="http://v8.1c.ru/8.1/data/core">`, а тот же массив
    // через фабрику из XSD она уже отвергает (измерено оба раза).
    // Отказ обязан НАЗЫВАТЬ причину — метаданные конфигурации.
    let collections = [
        BslValue::new_array(vec![number_value(1)]),
        BslValue::new_structure(bsl_rt::ShapeTable::default().empty(), Vec::new()),
        BslValue::new_map(),
        BslValue::new_table(),
    ];
    for value in collections {
        let err = serialize(&value, &[]).expect_err("коллекция не пишется");
        let text = err.to_string();
        assert!(text.contains("метаданных конфигурации"), "{text}");
        assert!(text.contains(V8_CORE_NS), "{text}");
    }
    // Экземпляры XDTO сериализатор тоже не пишет — измерено, что
    // 8.3.27 отвечает «Несоответствие типов (параметр номер '2')», —
    // но причина у них другая, и текст отказа другой.
    let f = factory(IO_SAMPLE);
    let o = factory_create(ob(&f), &[type_of_factory(&f, "RootType")]).expect("экземпляр");
    let err = serialize(&o, &[]).expect_err("экземпляр не пишется");
    assert!(err.to_string().contains("не умеет писать"), "{err}");
    // Составной тип схемы не читается тоже — и здесь платформа как раз
    // называет отсутствующее отображение.
    let err = deserialize(
        r#"<RootType xmlns="urn:test"><name>аб</name></RootType>"#,
        &[],
    )
    .expect_err("тип схемы не читается");
    assert!(err.to_string().contains("отсутствует отображение"), "{err}");
}

#[test]
fn xdto_serializer_leaves_the_reader_on_the_next_node() {
    // Позиция та же, что у фабрики: два соседних элемента читаются
    // двумя вызовами без `Прочитать()` между ними (измерено «42|аб»).
    let s = serializer();
    let doc = concat!(
        "<об>",
        r#"<decimal xmlns="http://www.w3.org/2001/XMLSchema">42</decimal>"#,
        r#"<string xmlns="http://www.w3.org/2001/XMLSchema">аб</string>"#,
        "</об>"
    );
    let r = reader(doc);
    crate::xml::with_reader(crate::xml::arg_object(&r).unwrap(), |state| {
        state.parser.as_mut().expect("разборщик").read()?;
        Ok(())
    })
    .expect("шаг к корню");
    let first = serializer_read_xml(ob(&s), std::slice::from_ref(&r)).expect("первый");
    let second = serializer_read_xml(ob(&s), std::slice::from_ref(&r)).expect("второй");
    assert_eq!(first, number_value(42));
    assert_eq!(second, str_value("аб"));
    // После корня читатель исчерпан, и следующее чтение — ошибка.
    assert!(serializer_read_xml(ob(&s), &[r]).is_err());
}

#[test]
fn xdto_serializer_says_plainly_what_it_does_not_implement() {
    // Три члена ИЗМЕРЕНЫ существующими у 8.3.27, и отказ — не «метод
    // не обнаружен» на компиляции, а перехватываемая ошибка. Причины у
    // них РАЗНЫЕ, и текст обязан их различать.
    let s = serializer();
    // Этим двум нужен тип `ТипДанныхXML`, которого здесь нет.
    for method in ["XMLТип", "XMLТипЗнч"] {
        let err = serializer_unsupported(ob(&s), method).to_string();
        assert!(err.contains("не поддерживается"), "{err}");
        assert!(err.contains("ТипДанныхXML"), "{err}");
    }
    // А этому — нет: измерено, что он отдаёт булево (снимок
    // `measure-xdto-serializer.platform.txt`, строки 33, 133, 134, 140,
    // 152, 153). Ссылаться на `ТипДанныхXML` здесь значило бы записать
    // в диагностику то, что опровергается собственным замером.
    let err = serializer_unsupported(ob(&s), "ВозможностьЧтенияXML").to_string();
    assert!(err.contains("не поддерживается"), "{err}");
    assert!(err.contains("булево"), "{err}");
    assert!(!err.contains("ТипДанныхXML"), "{err}");
    // У чужого получателя это обычное «метод неприменим».
    let err = serializer_unsupported(ob(&factory(IO_SAMPLE)), "XMLТип");
    assert!(matches!(err, RtError::MethodNotApplicable { .. }), "{err}");
}

#[test]
fn xdto_serializer_refuses_an_undefined_name_that_comes_with_a_uri() {
    // Снимок, строка 108: `ЗаписатьXML(Зпис, 42, Неопределено,
    // "urn:иное")` -> «Несоответствие типов (параметр номер '4')».
    let s = serializer();
    let w = writer();
    let err = serializer_write_xml(
        ob(&s),
        &[
            w.clone(),
            number_value(42),
            BslValue::Undefined,
            str_value("urn:иное"),
        ],
    )
    .expect_err("имя «Неопределено» вместе с URI отвергается");
    assert!(err.to_string().contains("параметр номер '4'"), "{err}");
    // Соседи по сетке измерены иначе, и отказ не должен задеть их.
    // Пустое имя с тем же URI — умолчание (строка 158).
    assert_eq!(
        serialize(&number_value(42), &[str_value(""), str_value("urn:иное")]).expect("умолчание"),
        r#"<decimal xmlns="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">42</decimal>"#
    );
    // `Неопределено` в обоих аргументах сразу — тоже умолчание
    // (строка 179), как и имя `Неопределено` без URI (строка 107).
    assert_eq!(
        serialize(
            &number_value(42),
            &[BslValue::Undefined, BslValue::Undefined]
        )
        .expect("умолчание"),
        r#"<decimal xmlns="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">42</decimal>"#
    );
    // А заданное имя с `Неопределено` вместо URI — пустое пространство
    // имён (строка 159).
    assert_eq!(
        serialize(&number_value(42), &[str_value("мой"), BslValue::Undefined])
            .expect("пустое пространство"),
        r#"<мой xmlns="" xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">42</мой>"#
    );
}

#[test]
fn xdto_serializer_generates_a_prefix_when_the_uri_is_the_instance_namespace() {
    // Строки 178 и 180 снимка: `xmlns:xsi` не объявляется вовсе, когда
    // умолчательным стало само пространство экземпляров, а пометке
    // `nil` платформа заводит порождённый префикс для того же URI.
    assert_eq!(
        serialize(&number_value(42), &[str_value("мой"), str_value(XSI_NS)]).expect("без xsi"),
        r#"<мой xmlns="http://www.w3.org/2001/XMLSchema-instance" xmlns:xs="http://www.w3.org/2001/XMLSchema">42</мой>"#
    );
    assert_eq!(
        serialize(&BslValue::Undefined, &[str_value("мой"), str_value(XSI_NS)])
            .expect("порождённый префикс"),
        concat!(
            r#"<мой xmlns="http://www.w3.org/2001/XMLSchema-instance" "#,
            r#"xmlns:d1p1="http://www.w3.org/2001/XMLSchema-instance" "#,
            r#"xmlns:xs="http://www.w3.org/2001/XMLSchema" d1p1:nil="true"/>"#
        )
    );
    // Глубина считается ПО ДОКУМЕНТУ, включая элемент, открытый до
    // вызова: строка 181 снимка даёт на втором уровне `d2p1`.
    let s = serializer();
    let w = writer();
    crate::xml::write_start_element(crate::xml::arg_object(&w).unwrap(), &[str_value("об")])
        .expect("обёртка");
    serializer_write_xml(
        ob(&s),
        &[
            w.clone(),
            BslValue::Undefined,
            str_value("мой"),
            str_value(XSI_NS),
        ],
    )
    .expect("запись внутрь");
    crate::xml::write_end_element(crate::xml::arg_object(&w).unwrap()).expect("конец обёртки");
    let text = match crate::xml::close_writer(crate::xml::arg_object(&w).unwrap())
        .expect("закрытие")
    {
        BslValue::Str(s) => s.to_string(),
        other => panic!("писатель отдал не строку: {other:?}"),
    };
    assert!(
        text.contains(r#"xmlns:d2p1="http://www.w3.org/2001/XMLSchema-instance""#),
        "{text}"
    );
    assert!(text.contains(r#"d2p1:nil="true""#), "{text}");
}
