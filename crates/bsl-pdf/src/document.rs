//! `ДокументPDF»: читатель контейнера PDF и поверхность встроенного языка.
//!
//! Писательское ядро (объектная модель, шрифты, примитивы страниц) живёт в
//! `bsl_rt::pdf` — им пользуется и раскладка табличного документа; здесь
//! разбор существующего файла, вложения и измеренная на 8.3.27 поверхность
//! «ДокументPDF» с коллекциями страниц и вложений.

use std::cell::RefCell;
use std::rc::Rc;

use bsl_rt::pdf::{inflate_with_limit, write_value, zlib_compress, PdfValue};
use bsl_rt::{
    BslNumber, BslString, BslValue, CallContext, EnumValue, ObjectProtocol, RtError, RtResult,
    TypeDescriptor, TypeId,
};

// ---------------------------------------------------------------------------
// Чтение контейнера
// ---------------------------------------------------------------------------
//
// Читатель разбирает ровно столько формата, сколько нужно поверхности
// `ДокументPDF`: таблицу перекрёстных ссылок (классическую и потоком),
// объекты, объектные потоки, фильтр `FlateDecode` с предикторами PNG — и
// дерево страниц. Содержимое страниц (текст, графика, шрифты) не
// разбирается вовсе: наружу видны только размер страницы и её поворот,
// потому что больше платформа про страницу и не отдаёт (измерено,
// `tests/conformance/measure/measure-pdf-read.bsl`).
//
// # Что измерено на 8.3.27 и здесь воспроизведено
//
// * размеры страницы отдаются в МИЛЛИМЕТРАХ целым числом: `/MediaBox
//   [0 0 100 200]` даёт 35 на 71 (100 пунктов — 35,28 мм, 200 — 70,56),
//   `[0 0 100.62 200]` — 35, `[0 0 100.64 200]` — 36, то есть округление
//   к ближайшему;
// * начало рамки не важно: `[10 20 110 220]` даёт те же 35 на 71;
// * видимую область задаёт `/CropBox`, а не `/MediaBox`: страница A4 с
//   `/CropBox [50 60 545.32 781.92]` отдала 175 на 255;
// * `/CropBox` ПЕРЕСЕКАЕТСЯ с `/MediaBox`: `[-100 -100 700 900]` поверх
//   A4 отдал 210 на 297, а не 282 на 353;
// * `/MediaBox` и `/CropBox` НАСЛЕДУЮТСЯ от узлов `/Pages` (страница без
//   собственной рамки под узлом с `/CropBox` отдала 175 на 255), а
//   `/Rotate` НЕ наследуется (страница под узлом с `/Rotate 90` отдала
//   ориентацию 0) — это расхождение с разделом 7.7.3.4 спецификации, где
//   наследуемым объявлен и он;
// * рамки без площади дают НОЛЬ на ноль по обеим сторонам сразу:
//   переставленная по X `[595.32 0 0 841.92]`, нулевая `[0 0 0 0]` и
//   отрицательная `[0 0 -100 -200]` — все три отдали 0 на 0;
// * страница без `/MediaBox` вообще — 216 на 279, то есть US Letter
//   (612 на 792 пункта);
// * `Ориентация` — это ЧИСЛО, значение `/Rotate`, приведённое к кратному
//   прямому углу округлением к ближайшему и затем к остатку от деления на
//   360: измерено 180 -> 180, 270 -> 270, -90 -> 270, 450 -> 90, 44 -> 0,
//   45 -> 90, 46 -> 90, 135 -> 180, 315 -> 0, 350 -> 0 (две последние —
//   ровно те половинки, на которых видно направление округления);
// * поля страницы (`ПолеСлева` и три соседа) приходят из `/TrimBox` —
//   именно его платформа кладёт в свои файлы рядом с одинаковым
//   `/BleedBox` (`probe-empty.pdf`: обе рамки [28.35 28.35 566.97 813.57]
//   при A4, то есть отступ ровно 10 мм, умолчание полей табличного
//   документа). Несимметричное правило разобрано у `margins_of`;
// * пустое дерево страниц (`/Count 0`, пустой `/Kids`) — законный файл с
//   нулём страниц, а трейлер без `/Root` — ошибка;
// * `Страницы` у документа, который ещё ничего не прочитал, — это
//   `Неопределено`, а не ошибка и не пустая коллекция;
// * инкрементальное обновление (цепочка `/Prev`) и файл с `xref`-потоком и
//   объектным потоком читаются наравне с классическим.
//
// # Враждебный вход
//
// Разбор идёт по данным пользователя, поэтому каждый обход, способный
// зациклиться, ограничен явно: цепочка `/Prev` — множеством уже виденных
// смещений, косвенные ссылки — стеком разбираемых номеров, дерево
// страниц — множеством номеров и глубиной, распаковка — пределом на
// размер выхода. Ни один битый или злонамеренный файл не должен подвесить
// разбор — он обязан кончиться `RtError` с внятным текстом.

/// Предел вложенности значений PDF (массивы и словари друг в друге) и
/// глубины дерева страниц. Настоящие файлы не подходят к нему близко.
const MAX_DEPTH: usize = 64;

/// Предел на длину цепочки таблиц перекрёстных ссылок (`/Prev` и
/// `/XRefStm`). Инкрементальных обновлений в живом файле единицы.
const MAX_XREF_SECTIONS: usize = 1024;

/// Предел на размер РАСПАКОВАННОГО потока. Без него подделанный поток
/// раздувает память: `FlateDecode` умеет отдавать гигабайты из килобайт.
const MAX_STREAM_OUT: usize = 256 << 20;

/// Предел на число страниц в дереве. Дерево обходится с множеством
/// посещённых номеров, поэтому предел нужен только для страниц, вписанных
/// в `/Kids` НЕ ссылкой, а словарём на месте.
const MAX_PAGES: usize = 1 << 20;

/// Страница разобранного файла: только то, что отдаёт платформа.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PdfPageInfo {
    width_mm: i64,
    height_mm: i64,
    rotate: i64,
    /// Поля в миллиметрах в порядке «слева, справа, сверху, снизу» —
    /// ровно том, в каком их читает [`PdfPageInfo::margin`].
    margins: [i64; 4],
}

/// Какое из четырёх полей страницы нужно.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PdfMargin {
    Left,
    Right,
    Top,
    Bottom,
}

impl PdfPageInfo {
    /// Ширина видимой области в миллиметрах.
    pub fn width_mm(&self) -> i64 {
        self.width_mm
    }

    /// Высота видимой области в миллиметрах.
    pub fn height_mm(&self) -> i64 {
        self.height_mm
    }

    /// Поворот страницы в градусах: 0, 90, 180 или 270.
    pub fn rotate(&self) -> i64 {
        self.rotate
    }

    /// Поле страницы в миллиметрах. Бывает отрицательным (измерено).
    pub fn margin(&self, which: PdfMargin) -> i64 {
        match which {
            PdfMargin::Left => self.margins[0],
            PdfMargin::Right => self.margins[1],
            PdfMargin::Top => self.margins[2],
            PdfMargin::Bottom => self.margins[3],
        }
    }
}

/// Предел на число вложений. Как и [`MAX_PAGES`], он нужен ради записей,
/// вписанных в дерево имён словарём на месте, а не ссылкой: их множество
/// посещённых номеров не ловит.
const MAX_ATTACHMENTS: usize = 1 << 16;

/// Связь вложения с документом — `/AFRelationship` файловой спецификации,
/// она же свойство `ТипСвязи`.
///
/// Членов ровно пять, и это ИЗМЕРЕНО перебором на 8.3.27: перечисление
/// называется `ТипСвязиВложенияPDF`, у него есть `Источник`/`Source`,
/// `Данные`/`Data`, `Альтернатива`/`Alternative`, `Дополнение`/`Supplement`
/// и `НеУстановлено`/`Unspecified`, а `Схема`, `ДанныеФормы` и
/// `ЗашифрованныеДанные` (они есть в таблице 43 спецификации) платформа не
/// знает. Неизвестное имя в файле читается как `НеУстановлено` — измерено
/// на `/AFRelationship /Nonsense`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PdfRelation {
    /// `/Source` — вложение является источником документа.
    Source,
    /// `/Data` — данные, по которым документ построен.
    Data,
    /// `/Alternative` — альтернативное представление.
    Alternative,
    /// `/Supplement` — дополнение к документу.
    Supplement,
    /// `/Unspecified` — связь не указана; ею же читается и неизвестное имя.
    #[default]
    Unspecified,
}

impl PdfRelation {
    /// Связь по имени из файла. Всё неизвестное — `Unspecified` (измерено).
    fn from_pdf_name(name: &str) -> PdfRelation {
        match name {
            "Source" => PdfRelation::Source,
            "Data" => PdfRelation::Data,
            "Alternative" => PdfRelation::Alternative,
            "Supplement" => PdfRelation::Supplement,
            _ => PdfRelation::Unspecified,
        }
    }

    /// Имя `/AFRelationship`, каким его пишет и платформа.
    pub fn pdf_name(self) -> &'static str {
        match self {
            PdfRelation::Source => "Source",
            PdfRelation::Data => "Data",
            PdfRelation::Alternative => "Alternative",
            PdfRelation::Supplement => "Supplement",
            PdfRelation::Unspecified => "Unspecified",
        }
    }
}

/// Вложение PDF — то, что платформа отдаёт как `ВложениеPDF`.
///
/// Наружу видны ровно четыре свойства, и все четыре ИЗМЕРЕНЫ перебором:
/// `ИмяФайла`, `ТипСодержимого`, `Содержимое` и `ТипСвязи`. Ни `Имя`, ни
/// `Описание`, ни `Размер`, ни `ДатаСоздания` платформа не знает, поэтому
/// `/Desc` и `/Params` из файла сюда не попадают вовсе.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PdfAttachment {
    name: String,
    content_type: String,
    relation: PdfRelation,
    data: Vec<u8>,
}

#[cfg_attr(not(test), allow(dead_code))]
impl PdfAttachment {
    /// Собрать вложение из имени, типа содержимого, связи и байтов.
    pub fn new(
        name: String,
        content_type: String,
        relation: PdfRelation,
        data: Vec<u8>,
    ) -> PdfAttachment {
        PdfAttachment {
            name,
            content_type,
            relation,
            data,
        }
    }

    /// `ИмяФайла` — имя из `/UF`, а если его нет, из `/F`.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// `ТипСодержимого` — `/Subtype` потока встроенного файла. Пустая
    /// строка, если его в файле нет (измерено).
    pub fn content_type(&self) -> &str {
        &self.content_type
    }

    /// `ТипСвязи` — `/AFRelationship`.
    pub fn relation(&self) -> PdfRelation {
        self.relation
    }

    /// `Содержимое` — распакованные байты встроенного файла.
    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

/// Разобранный файл PDF.
///
/// Кроме страниц и вложений здесь лежат ИСХОДНЫЕ БАЙТЫ файла и всё, что
/// нужно, чтобы дописать к ним инкрементальное обновление: номер объекта
/// каталога, сам каталог, его словарь `/Names` и смещение последней
/// таблицы перекрёстных ссылок. Держать файл целиком в памяти — плата за
/// запись, которая ничего не теряет: страницы, шрифты и содержимое
/// остаются ровно теми байтами, что пришли с диска.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PdfFile {
    pages: Vec<PdfPageInfo>,
    attachments: Vec<PdfAttachment>,
    source: Vec<u8>,
    /// Номер объекта каталога. `None`, если `/Root` в трейлере — словарь на
    /// месте, а не ссылка: такой каталог инкрементальным обновлением не
    /// переписать.
    catalog_number: Option<u32>,
    catalog: Vec<(String, PdfValue)>,
    /// Разрешённый словарь каталога `/Names` без `/EmbeddedFiles`: при
    /// записи он переносится целиком, чтобы не потерять чужие деревья имён
    /// (`/Dests`, `/JavaScript`).
    names_dict: Vec<(String, PdfValue)>,
    /// Смещение таблицы перекрёстных ссылок, с которой начался разбор, —
    /// оно же `/Prev` нового обновления.
    startxref: usize,
    /// Номер, с которого можно заводить новые объекты.
    next_object: u32,
}

impl PdfFile {
    /// Вложения в порядке обхода дерева имён.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn attachments(&self) -> &[PdfAttachment] {
        &self.attachments
    }

    /// Разобрать байты файла.
    ///
    /// # Errors
    ///
    /// [`RtError::Pdf`] на любом входе, который не является читаемым PDF:
    /// нет заголовка `%PDF-`, нет или испорчен `startxref`, битая таблица
    /// перекрёстных ссылок, неизвестный фильтр или предиктор, цикл в
    /// ссылках, в дереве страниц или в дереве имён вложений, отсутствующий
    /// `/Root`. Отдельным текстом сообщается о зашифрованном файле:
    /// расшифровки здесь нет.
    pub fn parse(data: &[u8]) -> RtResult<PdfFile> {
        Reader::new(data)?.read_file()
    }

    /// Число страниц.
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// Страница по номеру с нуля.
    pub fn page(&self, index: usize) -> Option<&PdfPageInfo> {
        self.pages.get(index)
    }
}

fn pdf_err(text: impl Into<String>) -> RtError {
    RtError::Pdf(text.into())
}

/// Байты ТЕКСТОВОЙ строки PDF (раздел 7.9.2.2) в строку языка: с меткой
/// `FE FF` это UTF-16BE, иначе UTF-8.
///
/// Спецификация на месте UTF-8 называет `PDFDocEncoding`, но платформа
/// читает именно UTF-8. Измерены ровно пять написаний имени (строки
/// «имя 0» — «имя 4» в `measure-pdf-attachments.platform.txt`): хекс-строка
/// `/F <6865782E747874>` возвращается как есть, «6865782e747874»;
/// восьмеричные экраны в `/UF` — тоже как есть, с обратными косыми; сырые
/// байты UTF-8 в `/F` дают «сырой.txt»; сырой UTF-16BE с меткой `FE FF` в
/// `/UF` — «шестнадцать.txt»; при обоих ключах сразу побеждает `/UF`.
/// Байты, которые не UTF-8, платформа тоже НЕ перекодирует: имя `/F` в
/// cp1251 («файл.txt») она отдаёт как четыре знака замены U+FFFD и целый
/// хвост «.txt» — по одному знаку замены на негодный байт, ровно как
/// `from_utf8_lossy` здесь (якорь `PDF.ATTACH_NAME_NON_UTF8`, замер
/// `measure-pdf.bsl`).
fn decode_text_string(bytes: &[u8]) -> String {
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        let units: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
            .collect();
        return String::from_utf16_lossy(&units);
    }
    String::from_utf8_lossy(bytes).into_owned()
}

/// Записать строку файловой спецификации СЫРЫМИ байтами: экранируются
/// только три знака, обязательных для того, чтобы строка кончилась там,
/// где надо.
///
/// Отличие от [`write_str`] измерено и существенно: разбор строк у
/// платформы неполон. Восьмеричные экраны `\ooo` она НЕ снимает, а
/// шестнадцатеричную форму `<...>` НЕ разбирает — и то и другое отдаёт
/// текстом как есть (пробы «имя 1» и «имя 0» в
/// `measure-pdf-attachments.platform.txt`). Поэтому имя, записанное
/// через `write_str`, который уводит всё непечатное в `\ooo`, вернулось
/// бы к ней как «\376\377\000i\000n\000c», а не как «inc.txt».
/// А вот эти три экрана платформа как раз СНИМАЕТ: `/F (a\(b\)c\\d.txt)`
/// она читает как «a(b)c\d.txt», и вложение с именем «па(ра)\кет.txt»
/// переживает у неё круговой прогон через запись и чтение (якорь
/// `PDF.ATTACH_STR_ESCAPES_READ`, замер `measure-pdf.bsl`). То есть
/// экранирование трёх разделителей здесь не только синтаксическая
/// необходимость записи, названная в первом абзаце, но и единственная
/// форма, которую читатель на той стороне понимает. Сама платформа пишет
/// `/F` в UTF-8 и `/UF` в UTF-16BE, оба сырыми байтами, — здесь ровно то
/// же самое.
fn write_raw_str(out: &mut Vec<u8>, bytes: &[u8]) {
    out.push(b'(');
    for &b in bytes {
        if matches!(b, b'(' | b')' | b'\\') {
            out.push(b'\\');
        }
        out.push(b);
    }
    out.push(b')');
}

/// Занять следующий номер объекта. Проверенное сложение: номера растут от
/// того, что было в чужом файле, и упереться в потолок `u32` они обязаны
/// ошибкой, а не заворачиванием на уже занятый объект.
fn take_object_number(next: &mut u32) -> RtResult<u32> {
    let taken = *next;
    *next = next
        .checked_add(1)
        .ok_or_else(|| pdf_err("в файле кончились номера объектов"))?;
    Ok(taken)
}

/// Имя вложения в UTF-16BE с меткой порядка байтов — то, что идёт в `/UF`.
fn utf16be_with_bom(text: &str) -> Vec<u8> {
    let mut out = vec![0xFE, 0xFF];
    for unit in text.encode_utf16() {
        out.extend_from_slice(&unit.to_be_bytes());
    }
    out
}

impl PdfFile {
    /// Записать документ с ЭТИМ набором вложений — инкрементальным
    /// обновлением поверх исходных байт (раздел 7.5.6).
    ///
    /// # Почему обновление, а не перезапись
    ///
    /// Платформа поступает иначе: `ДокументPDF.Записать` собирает файл
    /// заново. В снятом образце `tests/conformance/pdf/attach-platform.pdf`
    /// объекты перенумерованы с единицы подряд (с `1 0 obj` по `23 0 obj`),
    /// каталог — `14 0 obj` с `/PageMode /UseAttachments` и
    /// `/AF [ 7 0 R 8 0 R 9 0 R ]`, добавлен `/Metadata` с XMP
    /// «1C:Enterprise (8.3.27.2130)», а таблица `xref` классическая, с CRLF
    /// и одной секцией `0 24` при `/Size 24` в трейлере.
    /// Повторить это можно только удержав в памяти ВЕСЬ граф объектов
    /// файла, а читатель здесь берёт из него лишь геометрию страниц и
    /// вложения: всё остальное — шрифты, содержимое страниц, аннотации —
    /// он не разбирает и разбирать не должен. Инкрементальное обновление
    /// сохраняет их точно, потому что не трогает ни одного байта исходного
    /// файла, и ИЗМЕРЕНО, что платформа такой файл читает: собранное этим
    /// способом обновление она открыла как «страниц 1, вложений 2» и
    /// отдала оба вложения с верным содержимым.
    ///
    /// Плата — размер: байты старых встроенных файлов остаются в файле
    /// мусором, а новые пишутся заново даже для неизменённых вложений.
    /// Взамен запись не зависит от того, что читатель умеет разбирать.
    ///
    /// # Errors
    ///
    /// [`RtError::Pdf`], если `/Root` в трейлере был словарём на месте, а
    /// не ссылкой (такой каталог нечем заменить), или если номера объектов
    /// не помещаются в `u32`.
    pub fn write_with_attachments(&self, attachments: &[PdfAttachment]) -> RtResult<Vec<u8>> {
        let Some(catalog_number) = self.catalog_number else {
            return Err(pdf_err(
                "каталог документа записан словарём на месте, а не объектом: \
                 такой файл нечем обновить",
            ));
        };
        let mut out = self.source.clone();
        if !out.ends_with(b"\n") {
            out.push(b'\n');
        }
        let mut next = self.next_object;
        // Порядок записи в `/Names` — по имени, как требует раздел 7.9.6 и
        // как пишет платформа (измерено: её вывод отсортирован по
        // кодовым единицам UTF-16).
        let mut ordered: Vec<&PdfAttachment> = attachments.iter().collect();
        ordered.sort_by_key(|item| item.name.encode_utf16().collect::<Vec<u16>>());
        let mut offsets: Vec<(u32, usize)> = Vec::new();
        let mut names: Vec<(Vec<u8>, u32)> = Vec::new();
        for item in &ordered {
            let stream_number = take_object_number(&mut next)?;
            let spec_number = take_object_number(&mut next)?;
            let packed = zlib_compress(&item.data);
            offsets.push((stream_number, out.len()));
            let mut dict = vec![
                (
                    "Type".to_string(),
                    PdfValue::Name("EmbeddedFile".to_string()),
                ),
                (
                    "Filter".to_string(),
                    PdfValue::Name("FlateDecode".to_string()),
                ),
            ];
            // Пустой `/Subtype` платформа при записи заменяет на
            // `application/octet-stream` (измерено на её собственном
            // выводе), и здесь то же самое: иначе перечитанное вложение
            // меняло бы тип на пустой.
            let content_type = if item.content_type.is_empty() {
                "application/octet-stream"
            } else {
                item.content_type.as_str()
            };
            dict.push((
                "Subtype".to_string(),
                PdfValue::Name(content_type.to_string()),
            ));
            dict.push((
                "Params".to_string(),
                PdfValue::Dict(vec![(
                    "Size".to_string(),
                    PdfValue::Integer(item.data.len() as i64),
                )]),
            ));
            out.extend_from_slice(format!("{stream_number} 0 obj\n").as_bytes());
            write_value(&mut out, &PdfValue::Stream { dict, data: packed });
            out.extend_from_slice(b"\nendobj\n");

            offsets.push((spec_number, out.len()));
            out.extend_from_slice(
                format!("{spec_number} 0 obj\n<< /Type /Filespec /F ").as_bytes(),
            );
            write_raw_str(&mut out, item.name.as_bytes());
            out.extend_from_slice(b" /UF ");
            write_raw_str(&mut out, &utf16be_with_bom(&item.name));
            out.extend_from_slice(
                format!(
                    " /EF << /F {stream_number} 0 R >> /AFRelationship /{} >>\nendobj\n",
                    item.relation.pdf_name()
                )
                .as_bytes(),
            );
            names.push((utf16be_with_bom(&item.name), spec_number));
        }

        let names_number = take_object_number(&mut next)?;
        offsets.push((names_number, out.len()));
        out.extend_from_slice(format!("{names_number} 0 obj\n<< /Names [").as_bytes());
        for (key, spec_number) in &names {
            out.push(b' ');
            write_raw_str(&mut out, key);
            out.extend_from_slice(format!(" {spec_number} 0 R").as_bytes());
        }
        out.extend_from_slice(b" ] >>\nendobj\n");

        let names_dict_number = take_object_number(&mut next)?;
        offsets.push((names_dict_number, out.len()));
        let mut names_dict: Vec<(String, PdfValue)> = self
            .names_dict
            .iter()
            .filter(|(key, _)| key != "EmbeddedFiles")
            .cloned()
            .collect();
        names_dict.push(("EmbeddedFiles".to_string(), PdfValue::Ref(names_number)));
        out.extend_from_slice(format!("{names_dict_number} 0 obj\n").as_bytes());
        write_value(&mut out, &PdfValue::Dict(names_dict));
        out.extend_from_slice(b"\nendobj\n");

        let mut catalog: Vec<(String, PdfValue)> = self
            .catalog
            .iter()
            .filter(|(key, _)| key != "Names")
            .cloned()
            .collect();
        catalog.push(("Names".to_string(), PdfValue::Ref(names_dict_number)));
        offsets.push((catalog_number, out.len()));
        out.extend_from_slice(format!("{catalog_number} 0 obj\n").as_bytes());
        write_value(&mut out, &PdfValue::Dict(catalog));
        out.extend_from_slice(b"\nendobj\n");

        let xref_at = out.len();
        offsets.sort_unstable();
        out.extend_from_slice(b"xref\n");
        let mut at = 0;
        while at < offsets.len() {
            let mut end = at + 1;
            while end < offsets.len() && offsets[end].0 == offsets[end - 1].0 + 1 {
                end += 1;
            }
            out.extend_from_slice(format!("{} {}\n", offsets[at].0, end - at).as_bytes());
            for (_, offset) in &offsets[at..end] {
                out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
            }
            at = end;
        }
        out.extend_from_slice(
            format!(
                "trailer\n<< /Size {next} /Root {catalog_number} 0 R /Prev {} >>\n\
                 startxref\n{xref_at}\n%%EOF\n",
                self.startxref
            )
            .as_bytes(),
        );
        Ok(out)
    }
}

/// Пробельные знаки PDF (таблица 1 спецификации).
fn is_ws(b: u8) -> bool {
    matches!(b, 0x00 | 0x09 | 0x0A | 0x0C | 0x0D | 0x20)
}

/// Разделители PDF (таблица 2): на них кончается любая лексема.
fn is_delim(b: u8) -> bool {
    matches!(
        b,
        b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
    )
}

fn is_regular(b: u8) -> bool {
    !is_ws(b) && !is_delim(b)
}

/// Значение по ключу словаря. Ключи PDF регистрозависимы.
fn dict_get<'a>(dict: &'a [(String, PdfValue)], key: &str) -> Option<&'a PdfValue> {
    dict.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

/// Лексер и разборщик значений. Потоки он не разбирает — за ними нужен
/// `/Length`, а тот бывает косвенной ссылкой, то есть уже [`Reader`].
struct Lexer<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    fn new(data: &'a [u8], pos: usize) -> Self {
        Lexer { data, pos }
    }

    fn peek(&self) -> Option<u8> {
        self.data.get(self.pos).copied()
    }

    /// Пробелы и комментарии `%` до конца строки.
    fn skip_ws(&mut self) {
        loop {
            match self.peek() {
                Some(b) if is_ws(b) => self.pos += 1,
                Some(b'%') => {
                    while let Some(b) = self.peek() {
                        self.pos += 1;
                        if b == b'\n' || b == b'\r' {
                            break;
                        }
                    }
                }
                _ => return,
            }
        }
    }

    /// Слово из «обычных» знаков: `obj`, `endobj`, `stream`, `true`, число.
    fn token(&mut self) -> &'a [u8] {
        let start = self.pos;
        while let Some(b) = self.peek() {
            if is_regular(b) {
                self.pos += 1;
            } else {
                break;
            }
        }
        &self.data[start..self.pos]
    }

    fn expect_keyword(&mut self, word: &str) -> RtResult<()> {
        self.skip_ws();
        let tok = self.token();
        if tok == word.as_bytes() {
            Ok(())
        } else {
            Err(pdf_err(format!(
                "ожидалось «{word}», получено «{}»",
                String::from_utf8_lossy(tok)
            )))
        }
    }

    /// Целое без знака: номер объекта, поколение, длина.
    fn unsigned(&mut self) -> Option<u64> {
        self.skip_ws();
        let start = self.pos;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.pos += 1;
        }
        if self.pos == start {
            return None;
        }
        std::str::from_utf8(&self.data[start..self.pos])
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
    }

    fn parse_value(&mut self, depth: usize) -> RtResult<PdfValue> {
        if depth > MAX_DEPTH {
            return Err(pdf_err("слишком глубокая вложенность значений"));
        }
        self.skip_ws();
        let Some(b) = self.peek() else {
            return Err(pdf_err("значение оборвалось на конце файла"));
        };
        match b {
            b'/' => self.parse_name(),
            b'(' => self.parse_literal_string(),
            b'[' => {
                self.pos += 1;
                let mut items = Vec::new();
                loop {
                    self.skip_ws();
                    match self.peek() {
                        Some(b']') => {
                            self.pos += 1;
                            return Ok(PdfValue::Array(items));
                        }
                        None => return Err(pdf_err("массив без закрывающей скобки")),
                        _ => items.push(self.parse_value(depth + 1)?),
                    }
                }
            }
            b'<' => {
                if self.data.get(self.pos + 1) == Some(&b'<') {
                    let dict = self.parse_dict(depth)?;
                    Ok(PdfValue::Dict(dict))
                } else {
                    self.parse_hex_string()
                }
            }
            b'+' | b'-' | b'.' | b'0'..=b'9' => self.parse_number_or_ref(),
            b't' | b'f' | b'n' => {
                let tok = self.token();
                match tok {
                    b"true" => Ok(PdfValue::Bool(true)),
                    b"false" => Ok(PdfValue::Bool(false)),
                    b"null" => Ok(PdfValue::Null),
                    other => Err(pdf_err(format!(
                        "неизвестная лексема «{}»",
                        String::from_utf8_lossy(other)
                    ))),
                }
            }
            other => Err(pdf_err(format!(
                "значение не начинается ни с чего осмысленного: байт 0x{other:02X}"
            ))),
        }
    }

    fn parse_dict(&mut self, depth: usize) -> RtResult<Vec<(String, PdfValue)>> {
        if depth > MAX_DEPTH {
            return Err(pdf_err("слишком глубокая вложенность значений"));
        }
        self.pos += 2; // `<<`
        let mut out = Vec::new();
        loop {
            self.skip_ws();
            match self.peek() {
                Some(b'>') => {
                    if self.data.get(self.pos + 1) == Some(&b'>') {
                        self.pos += 2;
                        return Ok(out);
                    }
                    return Err(pdf_err("одиночная «>» в словаре"));
                }
                Some(b'/') => {
                    let PdfValue::Name(key) = self.parse_name()? else {
                        unreachable!("parse_name отдаёт только имя");
                    };
                    let value = self.parse_value(depth + 1)?;
                    out.push((key, value));
                }
                None => return Err(pdf_err("словарь без закрывающих «>>»")),
                Some(other) => {
                    return Err(pdf_err(format!(
                        "ключ словаря должен начинаться с «/», получен байт 0x{other:02X}"
                    )))
                }
            }
        }
    }

    /// Имя `/Name`. Escape-последовательности `#xx` раскрываются, поэтому
    /// `/A#20B` и `/A B` — разные имена, а `/Ко#Д` — ошибка.
    fn parse_name(&mut self) -> RtResult<PdfValue> {
        self.pos += 1; // `/`
        let mut bytes = Vec::new();
        while let Some(b) = self.peek() {
            if !is_regular(b) {
                break;
            }
            self.pos += 1;
            if b == b'#' {
                let hi = self.peek().and_then(hex_digit);
                let lo = self.data.get(self.pos + 1).copied().and_then(hex_digit);
                match (hi, lo) {
                    (Some(h), Some(l)) => {
                        bytes.push(h * 16 + l);
                        self.pos += 2;
                    }
                    _ => {
                        return Err(pdf_err(
                            "в имени после «#» ожидались две шестнадцатеричные цифры",
                        ))
                    }
                }
            } else {
                bytes.push(b);
            }
        }
        // Имена в PDF — последовательности байтов; для нас это ключи
        // словарей, все из ASCII, поэтому не-UTF-8 заменяется заменяющим
        // знаком и просто не совпадёт ни с одним известным ключом.
        Ok(PdfValue::Name(String::from_utf8_lossy(&bytes).into_owned()))
    }

    /// Строка `(...)`: экранирование обратной косой и БАЛАНС вложенных
    /// круглых скобок (раздел 7.3.4.2).
    fn parse_literal_string(&mut self) -> RtResult<PdfValue> {
        self.pos += 1; // `(`
        let mut out = Vec::new();
        let mut depth = 1usize;
        while let Some(b) = self.peek() {
            self.pos += 1;
            match b {
                b'\\' => {
                    let Some(esc) = self.peek() else {
                        return Err(pdf_err("строка оборвалась после обратной косой черты"));
                    };
                    self.pos += 1;
                    match esc {
                        b'n' => out.push(b'\n'),
                        b'r' => out.push(b'\r'),
                        b't' => out.push(b'\t'),
                        b'b' => out.push(0x08),
                        b'f' => out.push(0x0C),
                        b'(' => out.push(b'('),
                        b')' => out.push(b')'),
                        b'\\' => out.push(b'\\'),
                        // Перевод строки после косой — склейка строк.
                        b'\n' => {}
                        b'\r' => {
                            if self.peek() == Some(b'\n') {
                                self.pos += 1;
                            }
                        }
                        b'0'..=b'7' => {
                            let mut code = u32::from(esc - b'0');
                            for _ in 0..2 {
                                match self.peek() {
                                    Some(d @ b'0'..=b'7') => {
                                        code = code * 8 + u32::from(d - b'0');
                                        self.pos += 1;
                                    }
                                    _ => break,
                                }
                            }
                            out.push((code & 0xFF) as u8);
                        }
                        other => out.push(other),
                    }
                }
                b'(' => {
                    depth += 1;
                    out.push(b'(');
                }
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(PdfValue::Str(out));
                    }
                    out.push(b')');
                }
                other => out.push(other),
            }
        }
        Err(pdf_err("строка без закрывающей круглой скобки"))
    }

    /// Строка `<...>`: шестнадцатеричная, нечётный хвост дополняется нулём.
    fn parse_hex_string(&mut self) -> RtResult<PdfValue> {
        self.pos += 1; // `<`
        let mut out = Vec::new();
        let mut half: Option<u8> = None;
        while let Some(b) = self.peek() {
            self.pos += 1;
            if b == b'>' {
                if let Some(h) = half {
                    out.push(h * 16);
                }
                return Ok(PdfValue::Str(out));
            }
            if is_ws(b) {
                continue;
            }
            let Some(d) = hex_digit(b) else {
                return Err(pdf_err(format!(
                    "в шестнадцатеричной строке недопустимый байт 0x{b:02X}"
                )));
            };
            match half.take() {
                None => half = Some(d),
                Some(h) => out.push(h * 16 + d),
            }
        }
        Err(pdf_err("шестнадцатеричная строка без закрывающей «>»"))
    }

    /// Число, а если за ним стоят второе целое и `R` — косвенная ссылка.
    fn parse_number_or_ref(&mut self) -> RtResult<PdfValue> {
        let start = self.pos;
        if matches!(self.peek(), Some(b'+') | Some(b'-')) {
            self.pos += 1;
        }
        let mut seen_dot = false;
        while let Some(b) = self.peek() {
            match b {
                b'0'..=b'9' => self.pos += 1,
                b'.' if !seen_dot => {
                    seen_dot = true;
                    self.pos += 1;
                }
                // Второй знак минуса внутри числа встречается в файлах от
                // небрежных писателей («--5»); остаток числа просто
                // отбрасывается вместе с ним.
                _ => break,
            }
        }
        let text = std::str::from_utf8(&self.data[start..self.pos])
            .map_err(|_| pdf_err("число содержит не-ASCII"))?;
        let value = if seen_dot {
            let x: f64 = text
                .parse()
                .map_err(|_| pdf_err(format!("не число: «{text}»")))?;
            return Ok(PdfValue::Real(x));
        } else {
            text.parse::<i64>()
                .map_err(|_| pdf_err(format!("не целое: «{text}»")))?
        };
        // Косвенная ссылка `N G R`: пробуем и откатываемся, если не вышло.
        if value >= 0 {
            let save = self.pos;
            let mut probe = Lexer::new(self.data, self.pos);
            if probe.unsigned().is_some() {
                probe.skip_ws();
                if probe.token() == b"R" {
                    self.pos = probe.pos;
                    // Номер объекта шире `u32` — честная ошибка, а не молча
                    // срезанная ссылка на чужой объект.
                    let number = u32::try_from(value)
                        .map_err(|_| pdf_err("слишком большой номер объекта в ссылке"))?;
                    return Ok(PdfValue::Ref(number));
                }
            }
            self.pos = save;
        }
        Ok(PdfValue::Integer(value))
    }
}

fn hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Где лежит объект: прямо в файле или внутри объектного потока.
#[derive(Debug, Clone, Copy)]
enum XrefLoc {
    /// Смещение от начала файла.
    Offset(usize),
    /// Номер объектного потока и порядковый номер объекта в нём.
    InStream { stream: u32, index: usize },
}

/// Разобранный объектный поток `/Type /ObjStm`: распакованные байты и
/// таблица «номер объекта — смещение внутри».
struct ObjStm {
    data: Vec<u8>,
    entries: Vec<(u32, usize)>,
}

struct Reader<'a> {
    data: &'a [u8],
    xref: std::collections::HashMap<u32, XrefLoc>,
    trailer: Vec<(String, PdfValue)>,
    cache: std::collections::HashMap<u32, PdfValue>,
    /// Стек номеров разбираемых сейчас объектов — защита от `1 0 obj 1 0 R`.
    active: Vec<u32>,
    streams: std::collections::HashMap<u32, std::rc::Rc<ObjStm>>,
    /// Смещение таблицы, с которой начался разбор: `/Prev` будущего
    /// инкрементального обновления.
    startxref: usize,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> RtResult<Reader<'a>> {
        // Заголовок разрешено смещать от начала файла (раздел 7.5.2), но
        // не дальше первого килобайта.
        let head = &data[..data.len().min(1024)];
        if !head.windows(5).any(|w| w == b"%PDF-") {
            return Err(pdf_err("это не PDF: в начале файла нет «%PDF-»"));
        }
        let mut reader = Reader {
            data,
            xref: std::collections::HashMap::new(),
            trailer: Vec::new(),
            cache: std::collections::HashMap::new(),
            active: Vec::new(),
            streams: std::collections::HashMap::new(),
            startxref: 0,
        };
        let start = reader.find_startxref()?;
        reader.startxref = start;
        reader.load_xref_chain(start)?;
        if dict_get(&reader.trailer, "Encrypt").is_some() {
            return Err(pdf_err(
                "документ зашифрован: чтение зашифрованных PDF не поддерживается",
            ));
        }
        Ok(reader)
    }

    /// Последний `startxref` файла и стоящее за ним смещение.
    fn find_startxref(&self) -> RtResult<usize> {
        let tail_from = self.data.len().saturating_sub(2048);
        let tail = &self.data[tail_from..];
        let at = tail
            .windows(9)
            .rposition(|w| w == b"startxref")
            .ok_or_else(|| pdf_err("в конце файла нет «startxref»"))?;
        let mut lexer = Lexer::new(self.data, tail_from + at + 9);
        let offset = lexer
            .unsigned()
            .ok_or_else(|| pdf_err("после «startxref» нет смещения"))?;
        let offset = usize::try_from(offset).unwrap_or(usize::MAX);
        if offset >= self.data.len() {
            return Err(pdf_err(format!(
                "«startxref» указывает за пределы файла: {offset}"
            )));
        }
        Ok(offset)
    }

    /// Вся цепочка таблиц: сама таблица, её `/XRefStm` (гибридные файлы) и
    /// `/Prev` (инкрементальные обновления). Порядок обхода — от НОВОГО к
    /// старому, и запись побеждает первая: так и работает обновление.
    fn load_xref_chain(&mut self, start: usize) -> RtResult<()> {
        let mut seen = std::collections::HashSet::new();
        let mut pending = std::collections::VecDeque::new();
        pending.push_back(start);
        let mut sections = 0usize;
        while let Some(offset) = pending.pop_front() {
            if !seen.insert(offset) {
                continue;
            }
            sections += 1;
            if sections > MAX_XREF_SECTIONS {
                return Err(pdf_err(
                    "слишком длинная цепочка таблиц перекрёстных ссылок",
                ));
            }
            let section = self.load_xref_section(offset)?;
            for key in ["XRefStm", "Prev"] {
                if let Some(PdfValue::Integer(prev)) = dict_get(&section, key) {
                    if let Ok(prev) = usize::try_from(*prev) {
                        if prev < self.data.len() {
                            pending.push_back(prev);
                        }
                    }
                }
            }
            for (key, value) in section {
                if dict_get(&self.trailer, &key).is_none() {
                    self.trailer.push((key, value));
                }
            }
        }
        Ok(())
    }

    /// Одна таблица: классическая `xref` либо `xref`-поток. Отдаёт трейлер.
    fn load_xref_section(&mut self, offset: usize) -> RtResult<Vec<(String, PdfValue)>> {
        let mut lexer = Lexer::new(self.data, offset);
        lexer.skip_ws();
        if self.data[lexer.pos..].starts_with(b"xref") {
            lexer.pos += 4;
            return self.load_classic_xref(lexer);
        }
        self.load_xref_stream(offset)
    }

    fn load_classic_xref(&mut self, mut lexer: Lexer<'a>) -> RtResult<Vec<(String, PdfValue)>> {
        loop {
            lexer.skip_ws();
            if self.data[lexer.pos..].starts_with(b"trailer") {
                lexer.pos += 7;
                lexer.skip_ws();
                if lexer.peek() != Some(b'<') {
                    return Err(pdf_err("после «trailer» нет словаря"));
                }
                return lexer.parse_dict(0);
            }
            let first = lexer
                .unsigned()
                .ok_or_else(|| pdf_err("в таблице xref ожидался номер первого объекта"))?;
            let count = lexer
                .unsigned()
                .ok_or_else(|| pdf_err("в таблице xref ожидалось число записей"))?;
            if count > u64::try_from(self.data.len()).unwrap_or(u64::MAX) {
                return Err(pdf_err(
                    "в таблице xref объявлено больше записей, чем байт в файле",
                ));
            }
            for i in 0..count {
                let position = lexer
                    .unsigned()
                    .ok_or_else(|| pdf_err("запись xref без смещения"))?;
                let _generation = lexer
                    .unsigned()
                    .ok_or_else(|| pdf_err("запись xref без номера поколения"))?;
                lexer.skip_ws();
                let kind = lexer.token();
                // Сложение проверенное: `first` и `count` пришли из файла,
                // и их сумма обязана быть номером объекта, а не переполнением.
                let number = first
                    .checked_add(i)
                    .and_then(|number| u32::try_from(number).ok())
                    .ok_or_else(|| pdf_err("слишком большой номер объекта в таблице xref"))?;
                match kind {
                    b"n" => {
                        if let Ok(position) = usize::try_from(position) {
                            self.xref.entry(number).or_insert(XrefLoc::Offset(position));
                        }
                    }
                    b"f" => {
                        // Свободная запись: объекта нет. Занимать место в
                        // таблице ей всё равно нужно — иначе более старая
                        // секция вернула бы удалённый объект к жизни.
                        self.xref.entry(number).or_insert(XrefLoc::Offset(0));
                    }
                    other => {
                        return Err(pdf_err(format!(
                            "запись xref помечена «{}», а не «n» или «f»",
                            String::from_utf8_lossy(other)
                        )))
                    }
                }
            }
        }
    }

    fn load_xref_stream(&mut self, offset: usize) -> RtResult<Vec<(String, PdfValue)>> {
        let object = self.parse_object_at(offset)?;
        let PdfValue::Stream { dict, data } = object else {
            return Err(pdf_err(
                "«startxref» указывает не на таблицу xref и не на xref-поток",
            ));
        };
        let bytes = self.decode_stream(&dict, &data)?;
        let widths = match dict_get(&dict, "W") {
            Some(PdfValue::Array(items)) => items
                .iter()
                .map(|v| match v {
                    PdfValue::Integer(n) if *n >= 0 && *n <= 8 => Ok(*n as usize),
                    _ => Err(pdf_err("в /W xref-потока ожидались небольшие целые")),
                })
                .collect::<RtResult<Vec<_>>>()?,
            _ => return Err(pdf_err("в xref-потоке нет массива /W")),
        };
        if widths.len() < 3 {
            return Err(pdf_err("в /W xref-потока меньше трёх полей"));
        }
        let size = match dict_get(&dict, "Size") {
            Some(PdfValue::Integer(n)) if *n >= 0 => *n,
            _ => return Err(pdf_err("в xref-потоке нет /Size")),
        };
        let index: Vec<i64> = match dict_get(&dict, "Index") {
            Some(PdfValue::Array(items)) => items
                .iter()
                .map(|v| match v {
                    PdfValue::Integer(n) => Ok(*n),
                    _ => Err(pdf_err("в /Index xref-потока ожидались целые")),
                })
                .collect::<RtResult<Vec<_>>>()?,
            _ => vec![0, size],
        };
        let record = widths.iter().sum::<usize>();
        if record == 0 {
            return Err(pdf_err("в /W xref-потока все поля нулевой ширины"));
        }
        let mut at = 0usize;
        for pair in index.chunks(2) {
            let (&first, &count) = match pair {
                [first, count] => (first, count),
                _ => return Err(pdf_err("в /Index xref-потока нечётное число значений")),
            };
            for i in 0..count.max(0) {
                if at + record > bytes.len() {
                    return Err(pdf_err("xref-поток короче, чем объявлено в /Index"));
                }
                let mut field = [0u64; 3];
                let mut cursor = at;
                for (slot, width) in field.iter_mut().zip(widths.iter().copied()) {
                    let mut value = 0u64;
                    for _ in 0..width {
                        value = (value << 8) | u64::from(bytes[cursor]);
                        cursor += 1;
                    }
                    *slot = value;
                }
                at += record;
                // Нулевая ширина первого поля означает тип 1 (раздел 7.5.8.2).
                let kind = if widths[0] == 0 { 1 } else { field[0] };
                // Номер за пределами `u32` — запись просто пропускается, а
                // вот сумма за пределами `i64` означает мусорный `/Index`:
                // продолжать цикл после неё нечего, следующая же итерация
                // складывала бы `i64::MAX + 1`.
                let Some(number) = first.checked_add(i) else {
                    return Err(pdf_err(
                        "слишком большой номер объекта в /Index xref-потока",
                    ));
                };
                let Ok(number) = u32::try_from(number) else {
                    continue;
                };
                match kind {
                    1 => {
                        if let Ok(position) = usize::try_from(field[1]) {
                            self.xref.entry(number).or_insert(XrefLoc::Offset(position));
                        }
                    }
                    2 => {
                        if let (Ok(stream), Ok(index)) =
                            (u32::try_from(field[1]), usize::try_from(field[2]))
                        {
                            self.xref
                                .entry(number)
                                .or_insert(XrefLoc::InStream { stream, index });
                        }
                    }
                    // Тип 0 — свободный объект; всё остальное спецификация
                    // велит считать типом 0 (раздел 7.5.8.3).
                    _ => {
                        self.xref.entry(number).or_insert(XrefLoc::Offset(0));
                    }
                }
            }
        }
        Ok(dict)
    }

    /// Объект по номеру. Отсутствующий объект — это `null` (раздел 7.3.10),
    /// а не ошибка: так ведут себя все читатели, и так файл с испорченной
    /// таблицей всё же отдаёт то, что в нём уцелело.
    fn object(&mut self, number: u32) -> RtResult<PdfValue> {
        if let Some(value) = self.cache.get(&number) {
            return Ok(value.clone());
        }
        if self.active.contains(&number) {
            return Err(pdf_err(format!(
                "циклическая косвенная ссылка на объект {number}"
            )));
        }
        if self.active.len() > MAX_DEPTH {
            return Err(pdf_err("слишком глубокая цепочка косвенных ссылок"));
        }
        let Some(location) = self.xref.get(&number).copied() else {
            return Ok(PdfValue::Null);
        };
        self.active.push(number);
        let parsed = match location {
            XrefLoc::Offset(0) => Ok(PdfValue::Null),
            XrefLoc::Offset(offset) => self.parse_object_at(offset),
            XrefLoc::InStream { stream, index } => self.object_from_stream(stream, index),
        };
        self.active.pop();
        let value = parsed?;
        self.cache.insert(number, value.clone());
        Ok(value)
    }

    /// Значение, каким его видит потребитель: косвенная ссылка разыменована.
    fn resolve(&mut self, value: &PdfValue) -> RtResult<PdfValue> {
        match value {
            PdfValue::Ref(number) => self.object(*number),
            other => Ok(other.clone()),
        }
    }

    /// Разобрать `N G obj ... endobj` по смещению.
    fn parse_object_at(&mut self, offset: usize) -> RtResult<PdfValue> {
        if offset >= self.data.len() {
            return Err(pdf_err(format!(
                "объект за пределами файла: смещение {offset}"
            )));
        }
        let mut lexer = Lexer::new(self.data, offset);
        lexer
            .unsigned()
            .ok_or_else(|| pdf_err("объект не начинается с номера"))?;
        lexer
            .unsigned()
            .ok_or_else(|| pdf_err("у объекта нет номера поколения"))?;
        lexer.expect_keyword("obj")?;
        let value = lexer.parse_value(0)?;
        lexer.skip_ws();
        if !self.data[lexer.pos..].starts_with(b"stream") {
            return Ok(value);
        }
        let PdfValue::Dict(dict) = value else {
            return Err(pdf_err("перед «stream» должен стоять словарь"));
        };
        lexer.pos += 6;
        // После `stream` идёт CRLF или LF, но не одиночный CR (раздел 7.3.8.1).
        if self.data.get(lexer.pos) == Some(&b'\r') {
            lexer.pos += 1;
        }
        if self.data.get(lexer.pos) == Some(&b'\n') {
            lexer.pos += 1;
        }
        let start = lexer.pos;
        let declared = match dict_get(&dict, "Length") {
            Some(PdfValue::Integer(n)) if *n >= 0 => usize::try_from(*n).ok(),
            Some(PdfValue::Ref(number)) => match self.object(*number) {
                Ok(PdfValue::Integer(n)) if n >= 0 => usize::try_from(n).ok(),
                _ => None,
            },
            _ => None,
        };
        let end = match declared {
            Some(length) if self.ends_with_endstream(start, length) => start + length,
            // `/Length` соврал или его нет вовсе — ищем `endstream`. Платформа
            // такие файлы читает, и потерять из-за одного неверного числа
            // весь документ было бы хуже, чем довериться разделителю.
            _ => self.data[start..]
                .windows(9)
                .position(|w| w == b"endstream")
                .map(|at| start + at)
                .ok_or_else(|| pdf_err("поток без «endstream»"))?,
        };
        let mut data = self.data[start..end].to_vec();
        // Перевод строки перед `endstream` в поток не входит.
        if declared.is_none() || Some(end - start) != declared {
            if data.last() == Some(&b'\n') {
                data.pop();
            }
            if data.last() == Some(&b'\r') {
                data.pop();
            }
        }
        Ok(PdfValue::Stream { dict, data })
    }

    /// Стоит ли `endstream` там, где обещал `/Length`.
    fn ends_with_endstream(&self, start: usize, length: usize) -> bool {
        let Some(end) = start.checked_add(length) else {
            return false;
        };
        if end > self.data.len() {
            return false;
        }
        let mut lexer = Lexer::new(self.data, end);
        lexer.skip_ws();
        self.data[lexer.pos..].starts_with(b"endstream")
    }

    /// Объект из объектного потока `/Type /ObjStm`.
    fn object_from_stream(&mut self, stream: u32, index: usize) -> RtResult<PdfValue> {
        let objstm = self.object_stream(stream)?;
        let Some(&(_, offset)) = objstm.entries.get(index) else {
            return Err(pdf_err(format!(
                "в объектном потоке {stream} нет объекта с порядковым номером {index}"
            )));
        };
        if offset >= objstm.data.len() {
            return Err(pdf_err(format!(
                "объект {index} объектного потока {stream} лежит за его концом"
            )));
        }
        Lexer::new(&objstm.data, offset).parse_value(0)
    }

    fn object_stream(&mut self, number: u32) -> RtResult<std::rc::Rc<ObjStm>> {
        if let Some(found) = self.streams.get(&number) {
            return Ok(found.clone());
        }
        // Объектный поток лежит в файле напрямую: тип 2 внутри типа 2
        // спецификация запрещает, и защита от цикла тут именно эта.
        let location = self.xref.get(&number).copied();
        let Some(XrefLoc::Offset(offset)) = location else {
            return Err(pdf_err(format!(
                "объектный поток {number} не найден в таблице перекрёстных ссылок"
            )));
        };
        let PdfValue::Stream { dict, data } = self.parse_object_at(offset)? else {
            return Err(pdf_err(format!("объект {number} — не поток")));
        };
        let bytes = self.decode_stream(&dict, &data)?;
        let count = match dict_get(&dict, "N") {
            Some(PdfValue::Integer(n)) if *n >= 0 => usize::try_from(*n).unwrap_or(0),
            _ => return Err(pdf_err(format!("в объектном потоке {number} нет /N"))),
        };
        let first = match dict_get(&dict, "First") {
            Some(PdfValue::Integer(n)) if *n >= 0 => usize::try_from(*n).unwrap_or(usize::MAX),
            _ => return Err(pdf_err(format!("в объектном потоке {number} нет /First"))),
        };
        if first > bytes.len() {
            return Err(pdf_err(format!(
                "/First объектного потока {number} указывает за его конец"
            )));
        }
        let mut head = Lexer::new(&bytes, 0);
        let mut entries = Vec::with_capacity(count.min(4096));
        for _ in 0..count {
            let (Some(object), Some(offset)) = (head.unsigned(), head.unsigned()) else {
                return Err(pdf_err(format!(
                    "заголовок объектного потока {number} короче объявленного /N"
                )));
            };
            if head.pos > first {
                return Err(pdf_err(format!(
                    "заголовок объектного потока {number} заходит за /First"
                )));
            }
            let object = u32::try_from(object)
                .map_err(|_| pdf_err("слишком большой номер объекта в объектном потоке"))?;
            let offset = usize::try_from(offset).unwrap_or(usize::MAX);
            entries.push((object, first.saturating_add(offset)));
        }
        let objstm = std::rc::Rc::new(ObjStm {
            data: bytes,
            entries,
        });
        self.streams.insert(number, objstm.clone());
        Ok(objstm)
    }

    /// Снять с потока фильтры. Поддержан один — `FlateDecode`; всякий
    /// другой отвергается ПО ИМЕНИ, чтобы отказ был понятен.
    fn decode_stream(&mut self, dict: &[(String, PdfValue)], raw: &[u8]) -> RtResult<Vec<u8>> {
        let filters = match dict_get(dict, "Filter") {
            None | Some(PdfValue::Null) => Vec::new(),
            Some(PdfValue::Name(name)) => vec![name.clone()],
            Some(PdfValue::Array(items)) => {
                let mut out = Vec::with_capacity(items.len());
                for item in items {
                    match self.resolve(item)? {
                        PdfValue::Name(name) => out.push(name),
                        _ => return Err(pdf_err("в /Filter потока ожидались имена фильтров")),
                    }
                }
                out
            }
            Some(PdfValue::Ref(number)) => match self.object(*number)? {
                PdfValue::Name(name) => vec![name],
                PdfValue::Array(items) => {
                    let mut out = Vec::with_capacity(items.len());
                    for item in items {
                        match self.resolve(&item)? {
                            PdfValue::Name(name) => out.push(name),
                            _ => return Err(pdf_err("в /Filter потока ожидались имена фильтров")),
                        }
                    }
                    out
                }
                _ => return Err(pdf_err("/Filter потока — не имя и не массив имён")),
            },
            Some(_) => return Err(pdf_err("/Filter потока — не имя и не массив имён")),
        };
        let parms = match dict_get(dict, "DecodeParms").or_else(|| dict_get(dict, "DP")) {
            None => Vec::new(),
            Some(value) => match self.resolve(value)? {
                PdfValue::Null => Vec::new(),
                PdfValue::Dict(d) => vec![Some(d)],
                PdfValue::Array(items) => {
                    let mut out = Vec::with_capacity(items.len());
                    for item in items {
                        match self.resolve(&item)? {
                            PdfValue::Dict(d) => out.push(Some(d)),
                            _ => out.push(None),
                        }
                    }
                    out
                }
                _ => Vec::new(),
            },
        };
        let mut data = raw.to_vec();
        for (i, filter) in filters.iter().enumerate() {
            match filter.as_str() {
                "FlateDecode" | "Fl" => data = inflate_pdf_stream(&data)?,
                other => {
                    return Err(pdf_err(format!(
                        "фильтр потока «{other}» не поддерживается"
                    )))
                }
            }
            if let Some(Some(parm)) = parms.get(i) {
                let parm = parm.clone();
                data = self.apply_predictor(&parm, data)?;
            }
        }
        Ok(data)
    }

    /// Снять предиктор PNG (раздел 7.4.4.4). Предиктор TIFF (2) и всё
    /// неизвестное отвергаются с НОМЕРОМ в тексте ошибки.
    fn apply_predictor(
        &mut self,
        parms: &[(String, PdfValue)],
        data: Vec<u8>,
    ) -> RtResult<Vec<u8>> {
        let number = |key: &str, default: i64| -> RtResult<i64> {
            match dict_get(parms, key) {
                None | Some(PdfValue::Null) => Ok(default),
                Some(PdfValue::Integer(n)) => Ok(*n),
                Some(_) => Err(pdf_err(format!("/{key} предиктора — не целое"))),
            }
        };
        let predictor = number("Predictor", 1)?;
        if predictor <= 1 {
            return Ok(data);
        }
        if predictor < 10 {
            return Err(pdf_err(format!(
                "предиктор {predictor} не поддерживается: здесь снимаются только предикторы PNG (10..15)"
            )));
        }
        let colors = number("Colors", 1)?;
        let bits = number("BitsPerComponent", 8)?;
        let columns = number("Columns", 1)?;
        if colors <= 0 || bits <= 0 || columns <= 0 {
            return Err(pdf_err("параметры предиктора должны быть положительными"));
        }
        // Все три числа пришли из файла, поэтому цепочка проверена до
        // конца, вместе с округлением битов вверх до байтов: без этого
        // `+ 7` у `colors * bits` под `i64::MAX` переполняется само.
        let pixel_bits = colors
            .checked_mul(bits)
            .ok_or_else(|| pdf_err("слишком большая строка предиктора"))?;
        let row_bits = pixel_bits
            .checked_mul(columns)
            .ok_or_else(|| pdf_err("слишком большая строка предиктора"))?;
        let row_len = row_bits
            .checked_add(7)
            .and_then(|bits| usize::try_from(bits / 8).ok())
            .ok_or_else(|| pdf_err("слишком большая строка предиктора"))?;
        let step = pixel_bits
            .checked_add(7)
            .and_then(|bits| usize::try_from(bits / 8).ok())
            .ok_or_else(|| pdf_err("слишком большой шаг предиктора"))?
            .max(1);
        // Память выделяется по `row_len`, то есть по числу ИЗ ФАЙЛА, —
        // значит его нужно зажать самими данными. У правильного потока
        // каждая строка занимает в них `1 + row_len` байт, так что ни один
        // корректный файл этим не задет, а `data` после `inflate` уже
        // ограничена `MAX_STREAM_OUT`. Обрезанный до неполной строки поток
        // превращается из «строки, добитой нулями» в ошибку — так и надо.
        if data.is_empty() {
            return Ok(data);
        }
        if row_len > data.len() {
            return Err(pdf_err("строка предиктора длиннее самих данных"));
        }
        let mut out = Vec::with_capacity(data.len());
        let mut previous = vec![0u8; row_len];
        let mut at = 0usize;
        while at < data.len() {
            let kind = data[at];
            at += 1;
            let end = (at + row_len).min(data.len());
            let mut row = data[at..end].to_vec();
            row.resize(row_len, 0);
            at = end;
            for i in 0..row_len {
                let left = if i >= step { row[i - step] } else { 0 };
                let up = previous[i];
                let up_left = if i >= step { previous[i - step] } else { 0 };
                row[i] = match kind {
                    0 => row[i],
                    1 => row[i].wrapping_add(left),
                    2 => row[i].wrapping_add(up),
                    3 => row[i].wrapping_add(((u16::from(left) + u16::from(up)) / 2) as u8),
                    4 => row[i].wrapping_add(paeth(left, up, up_left)),
                    other => {
                        return Err(pdf_err(format!(
                            "неизвестный тип строки предиктора PNG: {other}"
                        )))
                    }
                };
            }
            out.extend_from_slice(&row);
            previous = row;
        }
        Ok(out)
    }

    /// Всё, что берётся из файла: дерево страниц `/Root` -> `/Pages` ->
    /// `/Kids`, дерево имён вложений и то, что понадобится инкрементальной
    /// записи.
    fn read_file(mut self) -> RtResult<PdfFile> {
        let root_value = match dict_get(&self.trailer, "Root").cloned() {
            Some(value) => value,
            None => return Err(pdf_err("в трейлере нет /Root")),
        };
        let catalog_number = match root_value {
            PdfValue::Ref(number) => Some(number),
            _ => None,
        };
        let PdfValue::Dict(root) = self.resolve(&root_value)? else {
            return Err(pdf_err("/Root указывает не на словарь каталога"));
        };
        let pages = match dict_get(&root, "Pages").cloned() {
            Some(value) => self.resolve(&value)?,
            None => return Err(pdf_err("в каталоге нет /Pages")),
        };
        let mut out = Vec::new();
        let mut seen = std::collections::HashSet::new();
        self.walk_pages(&pages, Inherited::default(), 0, &mut seen, &mut out)?;
        let attachments = self.read_attachments(&root)?;
        let names_dict = match dict_get(&root, "Names").cloned() {
            Some(value) => match self.resolve(&value)? {
                PdfValue::Dict(dict) => dict,
                _ => Vec::new(),
            },
            None => Vec::new(),
        };
        // Номер первого свободного объекта. `/Size` трейлера — это заявка
        // файла, таблица — то, что в нём есть на самом деле; берётся
        // максимум, потому что новый объект не должен наступить ни на одно
        // из двух даже во лгущем файле.
        let mut next_object = match dict_get(&self.trailer, "Size") {
            Some(PdfValue::Integer(size)) => u32::try_from(*size).unwrap_or(1),
            _ => 1,
        };
        for number in self.xref.keys() {
            next_object = next_object.max(number.saturating_add(1));
        }
        if let Some(number) = catalog_number {
            next_object = next_object.max(number.saturating_add(1));
        }
        Ok(PdfFile {
            pages: out,
            attachments,
            source: self.data.to_vec(),
            catalog_number,
            catalog: root,
            names_dict,
            startxref: self.startxref,
            next_object: next_object.max(1),
        })
    }

    /// Вложения: `/Root` -> `/Names` -> `/EmbeddedFiles` -> дерево имён.
    ///
    /// Одноимённые записи схлопываются, и побеждает ПОСЛЕДНЯЯ — измерено на
    /// файле с двумя записями `dup.txt`: платформа отдаёт одно вложение с
    /// содержимым второй.
    fn read_attachments(&mut self, root: &[(String, PdfValue)]) -> RtResult<Vec<PdfAttachment>> {
        let Some(names) = dict_get(root, "Names").cloned() else {
            return Ok(Vec::new());
        };
        let PdfValue::Dict(names) = self.resolve(&names)? else {
            return Ok(Vec::new());
        };
        let Some(embedded) = dict_get(&names, "EmbeddedFiles").cloned() else {
            return Ok(Vec::new());
        };
        let mut out: Vec<PdfAttachment> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        self.walk_name_tree(&embedded, 0, &mut seen, &mut out)?;
        let mut unique: Vec<PdfAttachment> = Vec::with_capacity(out.len());
        for item in out {
            match unique.iter_mut().find(|kept| kept.name == item.name) {
                Some(kept) => *kept = item,
                None => unique.push(item),
            }
        }
        Ok(unique)
    }

    /// Обход дерева имён (раздел 7.9.6): узел либо промежуточный с
    /// `/Kids`, либо лист с `/Names`. `/Limits` не читается вовсе — он
    /// нужен для ДВОИЧНОГО поиска по дереву, а нам нужны все записи
    /// подряд, и доверять ему в чужом файле незачем.
    ///
    /// Вход враждебный, поэтому ограничений три: множество посещённых
    /// номеров объектов, предел глубины и предел числа записей.
    fn walk_name_tree(
        &mut self,
        node: &PdfValue,
        depth: usize,
        seen: &mut std::collections::HashSet<u32>,
        out: &mut Vec<PdfAttachment>,
    ) -> RtResult<()> {
        if depth > MAX_DEPTH {
            return Err(pdf_err("слишком глубокое дерево имён вложений"));
        }
        let node = match node {
            PdfValue::Ref(number) => {
                if !seen.insert(*number) {
                    return Err(pdf_err(format!(
                        "цикл в дереве имён вложений: объект {number} встретился дважды"
                    )));
                }
                self.object(*number)?
            }
            other => other.clone(),
        };
        let PdfValue::Dict(node) = node else {
            return Err(pdf_err("узел дерева имён вложений — не словарь"));
        };
        if let Some(kids) = dict_get(&node, "Kids").cloned() {
            let PdfValue::Array(kids) = self.resolve(&kids)? else {
                return Err(pdf_err("/Kids дерева имён вложений — не массив"));
            };
            for kid in kids {
                self.walk_name_tree(&kid, depth + 1, seen, out)?;
            }
            return Ok(());
        }
        let Some(names) = dict_get(&node, "Names").cloned() else {
            // Лист без `/Names` и без `/Kids` — пустое дерево имён, а не
            // поломка: так выглядит документ, у которого вложения удалили.
            return Ok(());
        };
        let PdfValue::Array(names) = self.resolve(&names)? else {
            return Err(pdf_err("/Names дерева имён вложений — не массив"));
        };
        if names.len() % 2 != 0 {
            return Err(pdf_err(
                "в /Names дерева имён вложений нечётное число элементов",
            ));
        }
        for pair in names.chunks(2) {
            if out.len() >= MAX_ATTACHMENTS {
                return Err(pdf_err("в дереве имён вложений слишком много записей"));
            }
            // Ключ дерева не читается: имя вложения платформа берёт из
            // самой файловой спецификации (измерено — запись с ключом
            // «a-key» и `/F (a-file.txt)` отдала «a-file.txt»).
            if let Some(attachment) = self.filespec(&pair[1])? {
                out.push(attachment);
            }
        }
        Ok(())
    }

    /// Одна файловая спецификация (раздел 7.11.3). `None` — запись, которую
    /// платформа молча пропускает: без `/EF`, с висящей ссылкой в нём или
    /// вовсе без имени.
    fn filespec(&mut self, value: &PdfValue) -> RtResult<Option<PdfAttachment>> {
        let PdfValue::Dict(spec) = self.resolve(value)? else {
            return Ok(None);
        };
        // `/UF` — текстовая строка (UTF-16BE с меткой или UTF-8), `/F` —
        // строка файловой спецификации; при обеих побеждает `/UF`. Всё
        // измерено: файл с `/F (b-f.txt)` и `/UF` отдал имя из `/UF`.
        let name = match self.string_value(&spec, "UF")? {
            Some(bytes) => decode_text_string(&bytes),
            None => match self.string_value(&spec, "F")? {
                Some(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
                None => return Ok(None),
            },
        };
        let relation = match dict_get(&spec, "AFRelationship").cloned() {
            Some(value) => match self.resolve(&value)? {
                PdfValue::Name(name) => PdfRelation::from_pdf_name(&name),
                _ => PdfRelation::Unspecified,
            },
            None => PdfRelation::Unspecified,
        };
        let Some(ef) = dict_get(&spec, "EF").cloned() else {
            return Ok(None);
        };
        let PdfValue::Dict(ef) = self.resolve(&ef)? else {
            return Ok(None);
        };
        // В `/EF` спецификация разрешает те же ключи, что и в самой
        // файловой спецификации; берётся первый попавшийся поток.
        let mut stream = None;
        for key in ["F", "UF", "DOS", "Mac", "Unix"] {
            let Some(value) = dict_get(&ef, key).cloned() else {
                continue;
            };
            if let PdfValue::Stream { dict, data } = self.resolve(&value)? {
                stream = Some((dict, data));
                break;
            }
        }
        let Some((dict, data)) = stream else {
            return Ok(None);
        };
        let content_type = match dict_get(&dict, "Subtype") {
            Some(PdfValue::Name(name)) => name.clone(),
            _ => String::new(),
        };
        // Фильтр, которого мы не умеем (`/LZWDecode`), НЕ должен уносить
        // весь документ: измерено, что платформа такой файл читает и
        // вложение из него показывает. Отдаются сырые байты потока —
        // единственное, что у нас есть; потерять вместе с ними страницы
        // и остальные вложения было бы заметно хуже.
        let data = match self.decode_stream(&dict, &data) {
            Ok(decoded) => decoded,
            Err(_) => data,
        };
        Ok(Some(PdfAttachment {
            name,
            content_type,
            relation,
            data,
        }))
    }

    /// Строковое значение словаря, в том числе через косвенную ссылку.
    fn string_value(
        &mut self,
        dict: &[(String, PdfValue)],
        key: &str,
    ) -> RtResult<Option<Vec<u8>>> {
        let Some(value) = dict_get(dict, key).cloned() else {
            return Ok(None);
        };
        match self.resolve(&value)? {
            PdfValue::Str(bytes) => Ok(Some(bytes)),
            _ => Ok(None),
        }
    }

    fn walk_pages(
        &mut self,
        node: &PdfValue,
        inherited: Inherited,
        depth: usize,
        seen: &mut std::collections::HashSet<u32>,
        out: &mut Vec<PdfPageInfo>,
    ) -> RtResult<()> {
        if depth > MAX_DEPTH {
            return Err(pdf_err("слишком глубокое дерево страниц"));
        }
        let node = match node {
            PdfValue::Ref(number) => {
                if !seen.insert(*number) {
                    return Err(pdf_err(format!(
                        "цикл в дереве страниц: объект {number} встретился дважды"
                    )));
                }
                self.object(*number)?
            }
            other => other.clone(),
        };
        let PdfValue::Dict(node) = node else {
            return Err(pdf_err("узел дерева страниц — не словарь"));
        };
        let inherited = Inherited {
            media: self.rect(&node, "MediaBox")?.or(inherited.media),
            crop: self.rect(&node, "CropBox")?.or(inherited.crop),
        };
        let kids = match dict_get(&node, "Kids").cloned() {
            Some(value) => self.resolve(&value)?,
            None => PdfValue::Null,
        };
        if let PdfValue::Array(kids) = kids {
            for kid in kids {
                self.walk_pages(&kid, inherited, depth + 1, seen, out)?;
            }
            return Ok(());
        }
        if out.len() >= MAX_PAGES {
            return Err(pdf_err("в дереве страниц слишком много листьев"));
        }
        // Лист. `/Type /Page` не проверяется: файлы от небрежных писателей
        // его теряют, а решает всё равно отсутствие `/Kids`.
        //
        // `/TrimBox` берётся ТОЛЬКО со самой страницы: он, как и `/Rotate`,
        // не наследуется (измерено — страница под узлом с `/TrimBox` отдала
        // нулевые поля).
        let trim = self.rect(&node, "TrimBox")?;
        let rotate = rotate_of(&node);
        let visible = visible_rect(inherited);
        out.push(PdfPageInfo {
            width_mm: pt_to_mm(extent(visible).0),
            height_mm: pt_to_mm(extent(visible).1),
            rotate,
            margins: margins_of(visible, trim),
        });
        Ok(())
    }

    /// Прямоугольник из словаря: четыре числа, каждое может быть косвенным.
    fn rect(&mut self, dict: &[(String, PdfValue)], key: &str) -> RtResult<Option<[f64; 4]>> {
        let Some(value) = dict_get(dict, key).cloned() else {
            return Ok(None);
        };
        let value = self.resolve(&value)?;
        let PdfValue::Array(items) = value else {
            return Ok(None);
        };
        if items.len() != 4 {
            return Ok(None);
        }
        let mut out = [0.0f64; 4];
        for (slot, item) in out.iter_mut().zip(items.iter()) {
            *slot = match self.resolve(item)? {
                PdfValue::Integer(n) => n as f64,
                PdfValue::Real(x) if x.is_finite() => x,
                _ => return Ok(None),
            };
        }
        Ok(Some(out))
    }
}

/// Наследуемые от узлов `/Pages` атрибуты страницы. `/Rotate` здесь нет
/// намеренно: платформа его НЕ наследует (измерено).
#[derive(Debug, Clone, Copy, Default)]
struct Inherited {
    media: Option<[f64; 4]>,
    crop: Option<[f64; 4]>,
}

/// Видимая рамка страницы в пунктах: `/CropBox`, пересечённый с
/// `/MediaBox`, либо один `/MediaBox`. Рамки нет вовсе — US Letter
/// (измерено: страница без `/MediaBox` отдала 216 на 279 мм).
fn visible_rect(inherited: Inherited) -> [f64; 4] {
    const LETTER: [f64; 4] = [0.0, 0.0, 612.0, 792.0];
    let media = inherited.media.unwrap_or(LETTER);
    match inherited.crop {
        Some(crop) => [
            crop[0].max(media[0]),
            crop[1].max(media[1]),
            crop[2].min(media[2]),
            crop[3].min(media[3]),
        ],
        None => media,
    }
}

/// Ширина и высота рамки в пунктах. Рамка без площади — ноль на ноль ПО
/// ОБЕИМ сторонам сразу, а не по одной: измерено на переставленной,
/// нулевой и отрицательной рамках.
fn extent(rect: [f64; 4]) -> (f64, f64) {
    let width = rect[2] - rect[0];
    let height = rect[3] - rect[1];
    // Условие написано положительно, чтобы NaN (разность бесконечностей)
    // отсеивался вместе с нулевой и отрицательной площадью.
    if width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0 {
        (width, height)
    } else {
        (0.0, 0.0)
    }
}

/// Поля страницы в миллиметрах из `/TrimBox`.
///
/// ИЗМЕРЕНО на одиннадцати страницах, и правило вышло несимметричным:
///
/// * поле берётся ТОЛЬКО из `/TrimBox`; `/BleedBox` и `/ArtBox` не годятся
///   (страницы с одним из них отдали нули), а когда объявлены и `/TrimBox`,
///   и `/BleedBox`, побеждает `/TrimBox`;
/// * левое и ВЕРХНЕЕ поля — это АБСОЛЮТНЫЕ координаты левого нижнего угла
///   `/TrimBox`, а не отступы от `/MediaBox`: страница со смещённым началом
///   `[10 20 605.32 861.92]` и `/TrimBox [95.04 90.87 ...]` отдала 34 и 32,
///   а не 30 и 25;
/// * правое и НИЖНЕЕ — расстояния до дальних краёв ВИДИМОЙ рамки, то есть
///   `/CropBox`, если он есть: та же страница с `/CropBox` отдала 18 и 18;
/// * ось Y перевёрнута относительно PDF: «верхнее» поле отмеряется снизу
///   рамки, «нижнее» — сверху. Так платформа свои файлы и ПИШЕТ, поэтому
///   круг замыкается: `probe-margins.pdf`, записанный с полями 30/10/25/5,
///   читается обратно как 30/10/25/5;
/// * поля не поджимаются: `/TrimBox` шире `/MediaBox` даёт отрицательные
///   значения (-18, -37, -18, -20);
/// * `/TrimBox` НЕ наследуется от узлов `/Pages`.
fn margins_of(visible: [f64; 4], trim: Option<[f64; 4]>) -> [i64; 4] {
    let Some(trim) = trim else {
        return [0; 4];
    };
    [
        pt_to_mm(trim[0]),
        pt_to_mm(visible[2] - trim[2]),
        pt_to_mm(trim[1]),
        pt_to_mm(visible[3] - trim[3]),
    ]
}

/// Пункты в миллиметры с округлением к ближайшему — ровно так печатает
/// платформа (измерено: 100 пт -> 35, 200 пт -> 71, 100,62 -> 35,
/// 100,64 -> 36).
fn pt_to_mm(pt: f64) -> i64 {
    if !pt.is_finite() {
        return 0;
    }
    (pt * 25.4 / 72.0).round() as i64
}

/// `/Rotate` страницы, приведённый к кратному прямому углу.
///
/// Порядок действий измерен на десяти значениях: сначала остаток от
/// деления на 360, затем округление к ближайшему кратному 90, затем ещё
/// раз остаток (иначе 350 дало бы 360, а не 0).
fn rotate_of(page: &[(String, PdfValue)]) -> i64 {
    let raw = match dict_get(page, "Rotate") {
        Some(PdfValue::Integer(n)) => *n as f64,
        Some(PdfValue::Real(x)) if x.is_finite() => *x,
        _ => return 0,
    };
    let wrapped = raw % 360.0;
    let quarters = (wrapped / 90.0).round();
    if !quarters.is_finite() {
        return 0;
    }
    ((quarters as i64) * 90).rem_euclid(360)
}

fn paeth(left: u8, up: u8, up_left: u8) -> u8 {
    let p = i32::from(left) + i32::from(up) - i32::from(up_left);
    let (dl, du, dul) = (
        (p - i32::from(left)).abs(),
        (p - i32::from(up)).abs(),
        (p - i32::from(up_left)).abs(),
    );
    if dl <= du && dl <= dul {
        left
    } else if du <= dul {
        up
    } else {
        up_left
    }
}

/// Распаковать `FlateDecode`. Поток в PDF обёрнут по RFC 1950 (заголовок
/// zlib и adler32 в хвосте), но файлы от небрежных писателей встречаются и
/// с сырым deflate, поэтому обёртка распознаётся, а при неудаче делается
/// вторая попытка без неё.
fn inflate_pdf_stream(data: &[u8]) -> RtResult<Vec<u8>> {
    if data.len() >= 2 {
        let (cmf, flg) = (data[0], data[1]);
        let looks_zlib = cmf & 0x0F == 8 && (u16::from(cmf) * 256 + u16::from(flg)) % 31 == 0;
        if looks_zlib {
            if let Ok(out) =
                inflate_with_limit(flate2::read::ZlibDecoder::new(data), MAX_STREAM_OUT)
            {
                return Ok(out);
            }
        }
    }
    inflate_with_limit(flate2::read::DeflateDecoder::new(data), MAX_STREAM_OUT)
}

// ---------------------------------------------------------------------------
// Поверхность встроенного языка: `ДокументPDF`
// ---------------------------------------------------------------------------
//
// Всё, что здесь есть, снято с 8.3.27 скриптом
// `tests/conformance/measure/measure-pdf-read.bsl`; его вывод лежит рядом
// в `.platform.txt` и сверяется построчно.
//
// * `Новый ДокументPDF` — БЕЗ аргументов: и путь, и `ДвоичныеДанные` в
//   конструкторе платформа отвергает;
// * `Прочитать(ИмяФайла)` берёт только имя файла: `ДвоичныеДанные`
//   отвергнуты, вызов без аргументов — ошибка;
// * `Страницы` до первого чтения — `Неопределено`, и после НЕУДАЧНОГО
//   чтения снова `Неопределено`: документ возвращается в непрочитанное
//   состояние, а ошибку в нём даёт уже `Количество()`;
// * коллекция `КоллекцияСтраницPDF` умеет `Количество()`, `Получить(i)`,
//   `Индекс(Страница)`, `[i]` и `Для Каждого`; `Получить` вне диапазона
//   отдаёт `Неопределено` (и на 99, и на -1), а `[i]` — ошибку;
// * `КоличествоСтраниц` у документа НЕТ (ошибка);
// * страница только ЧИТАЕТСЯ: присваивание в `Ширина` — ошибка.
//
// Вложения сняты тем же способом (скрипт задачи —
// `tests/conformance/measure/measure-pdf-attachments.bsl`), и устроены они
// НЕ так, как страницы:
//
// * `Вложения` есть ВСЕГДА, в том числе до `Прочитать`: это пустая
//   `КоллекцияВложенийPDF`, а не `Неопределено`, и в неё уже можно
//   добавлять;
// * `Новый КоллекцияВложенийPDF` платформа строит, а `Новый ВложениеPDF` —
//   нет: вложение появляется только через `Добавить`;
// * коллекция умеет `Количество()`, `Получить(i)`, `[i]`, `Индекс(Влож)`,
//   `Найти(Имя)`, `Добавить(Имя, Данные[, ТипСодержимого[, ТипСвязи]])`,
//   `Удалить(i)`, `Очистить()` и `Для Каждого`; `Вставить` не пробовали, а
//   потому и не заявляем — здесь его нет до замера;
// * у вложения ровно четыре свойства — `ИмяФайла`, `ТипСодержимого`,
//   `Содержимое`, `ТипСвязи`, — и все ЧЕТЫРЕ ПИШУТСЯ (в отличие от
//   страницы, которая только читается);
// * `Записать(ИмяФайла[, Пароль])` у документа — процедура;
// * членов про ЭЛЕКТРОННУЮ ПОДПИСЬ у «ДокументPDF» на 8.3.27 НЕТ вовсе:
//   пробовано шесть написаний — чтения свойств `ЭлектронныеПодписи` и
//   `Подписи`, вызовы `ПолучитьЭлектронныеПодписи()`,
//   `ДобавитьЭлектроннуюПодпись()`, `ПроверитьЭлектроннуюПодпись()` и
//   `Подписать()`, — и все шесть кончаются исключением (текст ошибки
//   скрипт не печатает, только «нет» из ветки `Исключение`); четырёх типов
//   `ПодписьPDF`, `ЭлектроннаяПодписьPDF`, `PDFSignature` и
//   `КоллекцияПодписейPDF` платформа не знает. Поэтому здесь их тоже нет:
//   честная ошибка «нет такого метода» приходит сама, из общих таблиц, и
//   заводить ради неё пустую заглушку значило бы придумать платформе
//   поверхность, которой у неё нет.

/// Состояние `ДокументPDF`: разобранный файл либо ничего, если чтения ещё
/// не было или последнее чтение не удалось.
///
/// Вложения живут ОТДЕЛЬНО от разобранного файла, и это не украшение:
/// измерено, что `Вложения` у свежего документа — пустая коллекция, в
/// которую можно добавлять ещё до `Прочитать`. Коллекция и её элементы
/// держат тот же `Rc`, поэтому `Док.Вложения = Док.Вложения` — «Да»
/// (измерено), а `Прочитать` заменяет СОДЕРЖИМОЕ вектора, из-за чего уже
/// полученная коллекция видит новые вложения.
#[derive(Debug, Default)]
pub struct PdfDocState {
    file: Option<PdfFile>,
    attachments: Rc<RefCell<Vec<PdfAttachment>>>,
}

/// `Новый ДокументPDF` — пустой документ без источника.
pub fn new_pdf_document() -> BslValue {
    BslValue::new_object(DocumentObject {
        state: Rc::new(RefCell::new(PdfDocState::default())),
    })
}

/// `Новый КоллекцияВложенийPDF` — коллекция сама по себе, без документа.
///
/// Платформа такой конструктор ЗНАЕТ (измерено), хотя присоединить готовую
/// коллекцию к документу нечем: `Вложения` только читается. Значит, всё,
/// что с ней можно делать, — наполнять и разглядывать; ровно это здесь и
/// получается, потому что коллекция документа отличается от отдельной лишь
/// тем, кто ещё держит тот же `Rc`.
pub fn new_pdf_attachments() -> BslValue {
    BslValue::new_object(AttachmentsObject {
        items: Rc::new(RefCell::new(Vec::new())),
    })
}

/// `ДокументPDF` — держатель общего состояния: коллекция страниц и
/// страница — окна в тот же документ, а не снимки.
#[derive(Debug)]
pub struct DocumentObject {
    state: Rc<RefCell<PdfDocState>>,
}

/// `КоллекцияСтраницPDF` — окно в тот же документ.
#[derive(Debug)]
pub struct PagesObject {
    state: Rc<RefCell<PdfDocState>>,
}

/// `СтраницаPDF` — то же состояние плюс НОМЕР страницы с нуля.
#[derive(Debug)]
pub struct PageObject {
    state: Rc<RefCell<PdfDocState>>,
    index: usize,
}

/// `КоллекцияВложенийPDF` — вектор вложений, ОБЩИЙ с документом, если
/// коллекция получена из него; отдельным `Rc` она бывает и сама по себе.
#[derive(Debug)]
pub struct AttachmentsObject {
    items: Rc<RefCell<Vec<PdfAttachment>>>,
}

/// `ВложениеPDF` — тот же вектор плюс НОМЕР вложения с нуля.
#[derive(Debug)]
pub struct AttachmentObject {
    items: Rc<RefCell<Vec<PdfAttachment>>>,
    index: usize,
}

static DOCUMENT_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ДокументPDF",
    legacy_type_id: Some(TypeId::PdfDocument),
};

static PAGES_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "КоллекцияСтраницPDF",
    legacy_type_id: Some(TypeId::PdfPagesCollection),
};

static PAGE_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "СтраницаPDF",
    legacy_type_id: Some(TypeId::PdfPage),
};

static ATTACHMENTS_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "КоллекцияВложенийPDF",
    legacy_type_id: Some(TypeId::PdfAttachmentCollection),
};

static ATTACHMENT_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ВложениеPDF",
    legacy_type_id: Some(TypeId::PdfAttachment),
};

/// `ДокументPDF.Прочитать(ИмяФайла[, Пароль])`.
///
/// Пароль ПРИНИМАЕТСЯ И НЕ ИСПОЛЬЗУЕТСЯ, как у `Новый ЧтениеZipФайла`:
/// расшифровки здесь нет, и зашифрованный файл отвергается независимо от
/// того, назвали пароль или нет. Платформа с паролем такой файл читает
/// (измерено: `Прочитать(файл, "secret")` на RC4-40 отдало одну страницу),
/// и это единственное объявленное расхождение скрипта замеров чтения.
/// Симметричная граница на записи — в [`fn@write`].
///
/// Источником может быть ТОЛЬКО имя файла: `ДвоичныеДанные` платформа
/// отвергает (измерено). Принимает ли она ПОТОК, выяснить этой оснасткой
/// нельзя — на такой пробе платформа показывает модальное окно, то есть
/// немой таймаут, и в реестр открытых вопросов её не занести: одна строка
/// в `measure-all.bsl` унесла бы весь сеанс замеров. Поэтому здесь
/// принимается ровно то написание, которое измерено, — строка.
///
/// # Errors
///
/// [`RtError::Pdf`], если аргументов нет вовсе, если первым передано не имя
/// файла, если файл не читается с диска или не разбирается как PDF. При
/// любой ошибке документ возвращается в НЕПРОЧИТАННОЕ состояние — измерено,
/// что после неудачного чтения платформа снова отдаёт на `Страницы`
/// `Неопределено`, а ошибку даёт уже `Количество()` на нём. Вложения при
/// этом тоже забываются: измерено, что после неудачного чтения коллекция
/// пуста, а удачное чтение заменяет её содержимым нового файла.
pub fn read(document: &DocumentObject, args: &[BslValue]) -> RtResult<()> {
    let state = &document.state;
    // Сначала забываем прежнее, потом читаем: иначе неудача оставила бы
    // документ с чужими страницами. Вложения забываются тем же движением —
    // уже полученная коллекция при этом остаётся той же самой, меняется
    // только её содержимое.
    {
        let mut state = state.borrow_mut();
        state.file = None;
        state.attachments.borrow_mut().clear();
    }
    let (Some(source), true) = (args.first(), args.len() <= 2) else {
        return Err(pdf_err(
            "ДокументPDF.Прочитать ожидает имя файла и необязательный пароль",
        ));
    };
    let BslValue::Str(name) = source else {
        return Err(pdf_err(format!(
            "ДокументPDF.Прочитать ожидает имя файла строкой, получено «{}»",
            source.type_name()
        )));
    };
    let name = name.to_string();
    let bytes = std::fs::read(&name)
        .map_err(|e| pdf_err(format!("не удалось прочитать файл «{name}»: {e}")))?;
    let file = PdfFile::parse(&bytes)?;
    {
        let mut state = state.borrow_mut();
        state.attachments.borrow_mut().clone_from(&file.attachments);
        state.file = Some(file);
    }
    Ok(())
}

/// `ДокументPDF.Записать(ИмяФайла[, Пароль])` — процедура (измерено:
/// обращение к ней как к функции платформа отвергает).
///
/// Пишется ИНКРЕМЕНТАЛЬНОЕ ОБНОВЛЕНИЕ поверх прочитанных байт, см.
/// [`PdfFile::write_with_attachments`]; поэтому документ, который ничего не
/// читал, записать нечем. Платформа в этом случае тоже отвечает ошибкой,
/// хотя и оставляет на диске файл с одной пустой страницей A4 — этого
/// последнего мы не делаем сознательно: пустую страницу неоткуда взять, а
/// придумать её значило бы записать не тот документ, который просили.
///
/// # Errors
///
/// [`RtError::Pdf`], если аргументов нет или их больше двух, если имя файла
/// не строка, если документ ещё ничего не прочитал, если задан непустой
/// пароль (шифрования здесь нет) или если файл не записался на диск.
pub fn write(document: &DocumentObject, args: &[BslValue]) -> RtResult<()> {
    let state = &document.state;
    let (Some(target), true) = (args.first(), args.len() <= 2) else {
        return Err(pdf_err(
            "ДокументPDF.Записать ожидает имя файла и необязательный пароль",
        ));
    };
    let BslValue::Str(name) = target else {
        return Err(pdf_err(format!(
            "ДокументPDF.Записать ожидает имя файла строкой, получено «{}»",
            target.type_name()
        )));
    };
    // Пароль ПРИНИМАЕТСЯ, но работать с ним нечем: шифрования здесь нет ни
    // на чтении, ни на записи. Платформа на незашифрованном документе
    // отвечает на любой непустой пароль «Неверный пароль», то есть тоже
    // ошибкой.
    if let Some(password) = args.get(1) {
        let empty = match password {
            BslValue::Str(text) => text.len_utf16() == 0,
            BslValue::Undefined => true,
            _ => false,
        };
        if !empty {
            return Err(pdf_err(
                "ДокументPDF.Записать: шифрование PDF не поддерживается, пароль неприменим",
            ));
        }
    }
    let state = state.borrow();
    let file = state.file.as_ref().ok_or_else(|| {
        pdf_err("ДокументPDF.Записать: документ ничего не прочитал, записывать нечего")
    })?;
    let bytes = file.write_with_attachments(&state.attachments.borrow())?;
    let name = name.to_string();
    std::fs::write(&name, bytes)
        .map_err(|e| pdf_err(format!("не удалось записать файл «{name}»: {e}")))?;
    Ok(())
}

/// Свойство `ДокументPDF.Страницы` (`Pages`).
///
/// До первого чтения — и после НЕУДАЧНОГО чтения — свойство отдаёт
/// `Неопределено`, а не ошибку и не пустую коллекцию (измерено:
/// `ТипЗнч(Док.Страницы)` у свежего документа — «Не определено»). Ошибку
/// в этом состоянии даёт уже `Количество()`, потому что метода у
/// `Неопределено` нет.
///
/// # Errors
///
/// [`RtError::UnknownColumn`] на любом другом имени свойства: других
/// свойств у документа нет (измерено — `КоличествоСтраниц` платформа не
/// знает).
pub fn document_property(document: &DocumentObject, name: &str) -> RtResult<BslValue> {
    if name.eq_ignore_ascii_case("Вложения") || name.eq_ignore_ascii_case("Attachments") {
        // В отличие от `Страницы`, коллекция вложений есть и до чтения:
        // измерено, что у свежего документа это `КоллекцияВложенийPDF` с
        // нулём элементов, а не `Неопределено`.
        let items = document.state.borrow().attachments.clone();
        return Ok(BslValue::new_object(AttachmentsObject { items }));
    }
    if !name.eq_ignore_ascii_case("Страницы") && !name.eq_ignore_ascii_case("Pages") {
        return Err(RtError::UnknownColumn(name.to_string()));
    }
    if document.state.borrow().file.is_none() {
        return Ok(BslValue::Undefined);
    }
    Ok(BslValue::new_object(PagesObject {
        state: document.state.clone(),
    }))
}

/// Число вложений — общий путь `Количество()` и `Для Каждого`.
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель — не коллекция
/// вложений и не вложение.
pub fn attachment_count(attachments: &AttachmentsObject) -> RtResult<usize> {
    Ok(attachments.items.borrow().len())
}

/// `Вложения[Номер]` — вне диапазона ОШИБКА (измерено: `Вложения[99]`
/// платформа отвергает).
///
/// # Errors
///
/// [`RtError::IndexOutOfBounds`], если такого вложения нет.
pub fn attachment_at(attachments: &AttachmentsObject, index: usize) -> RtResult<BslValue> {
    let len = attachments.items.borrow().len();
    if index >= len {
        return Err(RtError::IndexOutOfBounds {
            index: index as i64,
            len,
        });
    }
    Ok(BslValue::new_object(AttachmentObject {
        items: attachments.items.clone(),
        index,
    }))
}

/// `Вложения.Получить(Номер)` — вне диапазона `Неопределено`, и на 99, и
/// на -1 (измерено), ровно как у страниц.
///
/// # Errors
///
/// [`RtError::TypeError`], если номер не число.
pub fn attachment_get(attachments: &AttachmentsObject, index: &BslValue) -> RtResult<BslValue> {
    let len = attachment_count(attachments)?;
    let BslValue::Number(number) = index else {
        return Err(RtError::TypeError {
            expected: "Число",
            op: "КоллекцияВложенийPDF.Получить",
        });
    };
    let Some(number) = number.to_i64_exact() else {
        return Ok(BslValue::Undefined);
    };
    match usize::try_from(number) {
        Ok(i) if i < len => attachment_at(attachments, i),
        _ => Ok(BslValue::Undefined),
    }
}

/// `Вложения.Индекс(Вложение)` — номер в этой же коллекции.
///
/// Строже, чем у страниц: чужое ЗНАЧЕНИЕ (число, массив) платформа
/// отвергает ошибкой типа, а не отдаёт -1 (измерено). Вложение из другой
/// коллекции — как раз тот случай, когда -1 законно.
///
/// # Errors
///
/// [`RtError::TypeError`], если аргумент не `ВложениеPDF`.
pub fn attachment_index_of(attachments: &AttachmentsObject, item: &BslValue) -> RtResult<BslValue> {
    let len = attachments.items.borrow().len();
    let found = match item
        .object_ref()
        .and_then(|object| object.downcast_ref::<AttachmentObject>())
    {
        Some(other) if Rc::ptr_eq(&attachments.items, &other.items) && other.index < len => {
            other.index as i64
        }
        Some(_) => -1,
        None => {
            return Err(RtError::TypeError {
                expected: "ВложениеPDF",
                op: "КоллекцияВложенийPDF.Индекс",
            })
        }
    };
    Ok(BslValue::Number(BslNumber::from_i64(found)))
}

/// Имя вложения из аргумента: строка как есть, число — своим
/// представлением, пустое имя — ошибка.
///
/// Так меряется платформа, и правило у `Найти` и `Добавить` одно:
/// `Найти(1)` отвечает `Неопределено` (то есть 1 стало именем «1» и не
/// нашлось), а `Найти("")` и `Добавить("", Данные)` — «Несоответствие
/// типов (параметр номер '1')».
fn attachment_name_arg(value: &BslValue, op: &'static str) -> RtResult<String> {
    let name = match value {
        BslValue::Str(text) => text.to_string(),
        // Целое число платформа принимает и превращает в имя (измерено:
        // после `ИмяФайла = 1` свойство отдаёт «1»). Дробное отвергается
        // здесь же: у него представление зависит от разделителя, и
        // придумывать его имени файла незачем.
        BslValue::Number(number) => match number.to_i64_exact() {
            Some(number) => number.to_string(),
            None => {
                return Err(RtError::TypeError {
                    expected: "Строка",
                    op,
                })
            }
        },
        _ => {
            return Err(RtError::TypeError {
                expected: "Строка",
                op,
            })
        }
    };
    if name.is_empty() {
        return Err(RtError::TypeError {
            expected: "непустое имя файла",
            op,
        });
    }
    Ok(name)
}

/// `Вложения.Найти(Имя)` — вложение с таким `ИмяФайла` либо
/// `Неопределено`. Аргумент ровно один (измерено).
///
/// # Errors
///
/// [`RtError::TypeError`], если имя пустое или не приводится к строке.
pub fn attachment_find(attachments: &AttachmentsObject, args: &[BslValue]) -> RtResult<BslValue> {
    let items = &attachments.items;
    let [name] = args else {
        return Err(pdf_err(
            "КоллекцияВложенийPDF.Найти ожидает ровно одно имя файла",
        ));
    };
    // Нестроковое и нечисловое значение платформа не бракует, а просто не
    // находит (измерено на `Найти(Новый Массив)` — «Не определено»).
    let name = match name {
        BslValue::Str(_) | BslValue::Number(_) => {
            attachment_name_arg(name, "КоллекцияВложенийPDF.Найти")?
        }
        _ => return Ok(BslValue::Undefined),
    };
    let found = items.borrow().iter().position(|item| item.name == name);
    match found {
        Some(index) => Ok(BslValue::new_object(AttachmentObject {
            items: items.clone(),
            index,
        })),
        None => Ok(BslValue::Undefined),
    }
}

/// `Вложения.Добавить(Имя, Данные[, ТипСодержимого[, ТипСвязи]])` —
/// процедура.
///
/// Одноимённое вложение СНИМАЕТСЯ, а новое дописывается В КОНЕЦ — то
/// есть коллекция ведёт себя как дерево имён, которым она и станет при
/// записи. Измерено: после `Добавить("а")`, `Добавить("б")` и повторного
/// `Добавить("а")` вложений двое, и порядок «б», «а», причём у «а» новые
/// тип содержимого и данные.
///
/// # Errors
///
/// [`RtError::Pdf`], если аргументов меньше двух или больше четырёх;
/// [`RtError::TypeError`], если имя пустое, если данные не
/// `ДвоичныеДанные`, если тип содержимого не строка или связь — не член
/// `ТипСвязиВложенияPDF`.
pub fn attachment_add(attachments: &AttachmentsObject, args: &[BslValue]) -> RtResult<()> {
    let items = &attachments.items;
    if args.len() < 2 || args.len() > 4 {
        return Err(pdf_err(
            "КоллекцияВложенийPDF.Добавить ожидает имя файла, данные и \
             необязательные тип содержимого и тип связи",
        ));
    }
    let name = attachment_name_arg(&args[0], "КоллекцияВложенийPDF.Добавить")?;
    let Some(data) = args[1].binary_data_bytes().map(<[u8]>::to_vec) else {
        return Err(RtError::TypeError {
            expected: "ДвоичныеДанные",
            op: "КоллекцияВложенийPDF.Добавить",
        });
    };
    let content_type = match args.get(2) {
        None | Some(BslValue::Undefined) => String::new(),
        Some(BslValue::Str(text)) => text.to_string(),
        Some(_) => {
            return Err(RtError::TypeError {
                expected: "Строка",
                op: "КоллекцияВложенийPDF.Добавить",
            })
        }
    };
    let relation = match args.get(3) {
        None => PdfRelation::Unspecified,
        Some(value) => relation_of(value, "КоллекцияВложенийPDF.Добавить")?,
    };
    let mut items = items.borrow_mut();
    let fresh = PdfAttachment {
        name,
        content_type,
        relation,
        data,
    };
    items.retain(|item| item.name != fresh.name);
    items.push(fresh);
    Ok(())
}

/// Член перечисления `ТипСвязиВложенияPDF` из значения языка.
fn relation_of(value: &BslValue, op: &'static str) -> RtResult<PdfRelation> {
    let BslValue::Enum(member) = value else {
        return Err(RtError::TypeError {
            expected: "ТипСвязиВложенияPDF",
            op,
        });
    };
    match member {
        EnumValue::PdfRelationSource => Ok(PdfRelation::Source),
        EnumValue::PdfRelationData => Ok(PdfRelation::Data),
        EnumValue::PdfRelationAlternative => Ok(PdfRelation::Alternative),
        EnumValue::PdfRelationSupplement => Ok(PdfRelation::Supplement),
        EnumValue::PdfRelationUnspecified => Ok(PdfRelation::Unspecified),
        _ => Err(RtError::TypeError {
            expected: "ТипСвязиВложенияPDF",
            op,
        }),
    }
}

/// Член перечисления по связи — обратное [`relation_of`].
fn relation_enum(relation: PdfRelation) -> EnumValue {
    match relation {
        PdfRelation::Source => EnumValue::PdfRelationSource,
        PdfRelation::Data => EnumValue::PdfRelationData,
        PdfRelation::Alternative => EnumValue::PdfRelationAlternative,
        PdfRelation::Supplement => EnumValue::PdfRelationSupplement,
        PdfRelation::Unspecified => EnumValue::PdfRelationUnspecified,
    }
}

/// `Вложения.Удалить(Номер)` либо `Удалить(Вложение)`.
///
/// Номер ВНЕ ДИАПАЗОНА не ошибка: измерено, что и `Удалить(99)`, и
/// `Удалить(-1)` удаляют ПОСЛЕДНЕЕ вложение, а не жалуются.
///
/// Что делает платформа с ПУСТОЙ коллекцией, этой оснасткой не выяснить:
/// `Удалить(0)` на пустой она встречает модальным окном, то есть немым
/// таймаутом, и `Попытка` его не ловит (проверено отдельной пробой —
/// вывод обрывается ровно перед вызовом). Строке в реестре открытых
/// вопросов там не место: она унесла бы весь сеанс замеров. Здесь пустая
/// коллекция остаётся пустой — это единственное продолжение правила
/// «номер вне диапазона удаляет последнее» на случай, когда последнего
/// нет.
///
/// # Errors
///
/// [`RtError::Pdf`], если аргумент не один;
/// [`RtError::TypeError`], если это не число и не `ВложениеPDF`.
pub fn attachment_delete(attachments: &AttachmentsObject, args: &[BslValue]) -> RtResult<()> {
    let items = &attachments.items;
    let [what] = args else {
        return Err(pdf_err(
            "КоллекцияВложенийPDF.Удалить ожидает ровно один аргумент",
        ));
    };
    let len = items.borrow().len();
    if len == 0 {
        return Ok(());
    }
    let index = match what {
        BslValue::Number(number) => match number.to_i64_exact() {
            Some(number) => match usize::try_from(number) {
                Ok(index) if index < len => index,
                _ => len - 1,
            },
            None => len - 1,
        },
        _ => match what
            .object_ref()
            .and_then(|object| object.downcast_ref::<AttachmentObject>())
        {
            Some(other) if Rc::ptr_eq(items, &other.items) && other.index < len => other.index,
            Some(_) => {
                return Err(RtError::TypeError {
                    expected: "ВложениеPDF этой же коллекции",
                    op: "КоллекцияВложенийPDF.Удалить",
                })
            }
            None => {
                return Err(RtError::TypeError {
                    expected: "Число или ВложениеPDF",
                    op: "КоллекцияВложенийPDF.Удалить",
                })
            }
        },
    };
    items.borrow_mut().remove(index);
    Ok(())
}

/// `Вложения.Очистить()` — аргументов не берёт (измерено).
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не коллекция вложений.
pub fn attachment_clear(attachments: &AttachmentsObject) -> RtResult<()> {
    attachments.items.borrow_mut().clear();
    Ok(())
}

/// Свойства `ВложениеPDF`. Все четыре имени и все четыре английских
/// синонима измерены: `FileName`, `MIMEType`, `Content` и
/// `RelationshipType`. Перебором проверено и то, чего у платформы НЕТ:
/// `ContentType`, `MediaType`, `Mime`, `Relationship`, `AFRelationship`,
/// `Relation`, `Имя`, `Описание`, `Размер` и `Данные`.
///
/// # Errors
///
/// [`RtError::Pdf`], если вложение уже удалено из коллекции;
/// [`RtError::UnknownColumn`] на неизвестном имени.
pub fn attachment_property(attachment: &AttachmentObject, name: &str) -> RtResult<BslValue> {
    let items = attachment.items.borrow();
    let item = items
        .get(attachment.index)
        .ok_or_else(|| pdf_err("вложение уже удалено из коллекции"))?;
    if name.eq_ignore_ascii_case("ИмяФайла") || name.eq_ignore_ascii_case("FileName") {
        return Ok(BslValue::Str(BslString::from_str(&item.name)));
    }
    if name.eq_ignore_ascii_case("ТипСодержимого") || name.eq_ignore_ascii_case("MIMEType")
    {
        return Ok(BslValue::Str(BslString::from_str(&item.content_type)));
    }
    if name.eq_ignore_ascii_case("Содержимое") || name.eq_ignore_ascii_case("Content") {
        return Ok(BslValue::binary_data_of(item.data.clone()));
    }
    if name.eq_ignore_ascii_case("ТипСвязи") || name.eq_ignore_ascii_case("RelationshipType")
    {
        return Ok(BslValue::Enum(relation_enum(item.relation)));
    }
    Err(RtError::UnknownColumn(name.to_string()))
}

/// Присваивание в свойство `ВложениеPDF`. Пишутся ВСЕ ЧЕТЫРЕ (измерено —
/// в отличие от страницы, которая только читается).
///
/// # Errors
///
/// [`RtError::Pdf`], если вложение уже удалено из коллекции;
/// [`RtError::TypeError`] на значении не того типа;
/// [`RtError::UnknownColumn`] на неизвестном имени.
pub fn set_attachment_property(
    attachment: &AttachmentObject,
    name: &str,
    value: &BslValue,
) -> RtResult<()> {
    let mut items = attachment.items.borrow_mut();
    let item = items
        .get_mut(attachment.index)
        .ok_or_else(|| pdf_err("вложение уже удалено из коллекции"))?;
    if name.eq_ignore_ascii_case("ИмяФайла") || name.eq_ignore_ascii_case("FileName") {
        // Число платформа принимает и превращает в строку (измерено:
        // после `ИмяФайла = 1` свойство отдаёт «1»).
        item.name = attachment_name_arg(value, "ВложениеPDF.ИмяФайла")?;
        return Ok(());
    }
    if name.eq_ignore_ascii_case("ТипСодержимого") || name.eq_ignore_ascii_case("MIMEType")
    {
        let BslValue::Str(text) = value else {
            return Err(RtError::TypeError {
                expected: "Строка",
                op: "ВложениеPDF.ТипСодержимого",
            });
        };
        item.content_type = text.to_string();
        return Ok(());
    }
    if name.eq_ignore_ascii_case("Содержимое") || name.eq_ignore_ascii_case("Content") {
        let Some(bytes) = value.binary_data_bytes() else {
            return Err(RtError::TypeError {
                expected: "ДвоичныеДанные",
                op: "ВложениеPDF.Содержимое",
            });
        };
        item.data = bytes.to_vec();
        return Ok(());
    }
    if name.eq_ignore_ascii_case("ТипСвязи") || name.eq_ignore_ascii_case("RelationshipType")
    {
        item.relation = relation_of(value, "ВложениеPDF.ТипСвязи")?;
        return Ok(());
    }
    Err(RtError::UnknownColumn(name.to_string()))
}

/// Число страниц — общий путь `Количество()` и `Для Каждого`.
///
/// # Errors
///
/// [`RtError::Pdf`], если документ ничего не прочитал.
pub fn page_count(pages: &PagesObject) -> RtResult<usize> {
    let state = pages.state.borrow();
    let file = state
        .file
        .as_ref()
        .ok_or_else(|| pdf_err("у ДокументPDF ещё нет страниц: сначала нужен Прочитать"))?;
    Ok(file.page_count())
}

/// `Страницы.Получить(Номер)` — вне диапазона отдаёт `Неопределено`, и на
/// 99, и на -1 (измерено). Этим он и отличается от `Страницы[Номер]`.
///
/// # Errors
///
/// [`RtError::Pdf`], если документ ничего не прочитал;
/// [`RtError::TypeError`], если номер не число.
pub fn page_get(pages: &PagesObject, index: &BslValue) -> RtResult<BslValue> {
    let count = page_count(pages)?;
    let BslValue::Number(number) = index else {
        return Err(RtError::TypeError {
            expected: "Число",
            op: "КоллекцияСтраницPDF.Получить",
        });
    };
    let Some(number) = number.to_i64_exact() else {
        return Ok(BslValue::Undefined);
    };
    match usize::try_from(number) {
        Ok(i) if i < count => page_at(pages, i),
        _ => Ok(BslValue::Undefined),
    }
}

/// `Страницы[Номер]` — вне диапазона ОШИБКА (измерено: `Страницы[-1]`
/// платформа отвергает, хотя `Получить(-1)` отдаёт `Неопределено`).
///
/// # Errors
///
/// [`RtError::Pdf`], если документ ничего не прочитал;
/// [`RtError::IndexOutOfBounds`], если такой страницы нет.
pub fn page_at(pages: &PagesObject, index: usize) -> RtResult<BslValue> {
    let count = page_count(pages)?;
    if index >= count {
        return Err(RtError::IndexOutOfBounds {
            index: index as i64,
            len: count,
        });
    }
    Ok(BslValue::new_object(PageObject {
        state: pages.state.clone(),
        index,
    }))
}

/// `Страницы.Индекс(Страница)` — номер страницы в этой же коллекции, и
/// `-1`, если страница чужая. Номер измерен: `Индекс(Страницы[1])` — 1.
///
/// # Errors
///
/// [`RtError::Pdf`], если документ ничего не прочитал.
pub fn page_index_of(pages: &PagesObject, page: &BslValue) -> RtResult<BslValue> {
    let count = page_count(pages)?;
    let found = match page
        .object_ref()
        .and_then(|object| object.downcast_ref::<PageObject>())
    {
        Some(other) if Rc::ptr_eq(&pages.state, &other.state) && other.index < count => {
            other.index as i64
        }
        _ => -1,
    };
    Ok(BslValue::Number(BslNumber::from_i64(found)))
}

/// Свойства `СтраницаPDF`. Все восемь имён и оба языка измерены.
///
/// # Errors
///
/// [`RtError::Pdf`], если документ успел забыть прочитанное;
/// [`RtError::UnknownColumn`] на неизвестном имени.
pub fn page_property(page_object: &PageObject, name: &str) -> RtResult<BslValue> {
    let number = |value: i64| Ok(BslValue::Number(BslNumber::from_i64(value)));
    let state = page_object.state.borrow();
    let page = state
        .file
        .as_ref()
        .and_then(|file| file.page(page_object.index))
        .ok_or_else(|| pdf_err("страница относится к документу, который уже перечитан"))?;
    if name.eq_ignore_ascii_case("Номер") || name.eq_ignore_ascii_case("Number") {
        // Номер СТРАНИЦЫ с единицы, в отличие от номера в коллекции
        // (измерено: `Страницы[0].Номер` — 1).
        return number(page_object.index as i64 + 1);
    }
    if name.eq_ignore_ascii_case("Ширина") || name.eq_ignore_ascii_case("Width") {
        return number(page.width_mm());
    }
    if name.eq_ignore_ascii_case("Высота") || name.eq_ignore_ascii_case("Height") {
        return number(page.height_mm());
    }
    if name.eq_ignore_ascii_case("Ориентация") || name.eq_ignore_ascii_case("Orientation")
    {
        // Ориентация — ЧИСЛО, а не член перечисления (измерено:
        // `ТипЗнч(Страницы[0].Ориентация)` — «Число»).
        return number(page.rotate());
    }
    // Поля страницы приходят из `/TrimBox`; правило — в `margins_of`.
    for (ru, en, which) in [
        ("ПолеСлева", "LeftMargin", PdfMargin::Left),
        ("ПолеСправа", "RightMargin", PdfMargin::Right),
        ("ПолеСверху", "TopMargin", PdfMargin::Top),
        ("ПолеСнизу", "BottomMargin", PdfMargin::Bottom),
    ] {
        if name.eq_ignore_ascii_case(ru) || name.eq_ignore_ascii_case(en) {
            return number(page.margin(which));
        }
    }
    Err(RtError::UnknownColumn(name.to_string()))
}

// --- объектный протокол -----------------------------------------------------

/// Адрес состояния как ключ тождества: обёртки строятся на каждое
/// обращение, а равенство у окон в документ — «то же состояние, то же
/// место» (измерено, см. ключи ниже).
fn state_addr<T>(state: &Rc<RefCell<T>>) -> usize {
    Rc::as_ptr(state) as usize
}

impl ObjectProtocol for DocumentObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &DOCUMENT_TYPE
    }

    fn get_property(&self, name: &str, _context: &mut CallContext<'_>) -> RtResult<BslValue> {
        document_property(self, name)
    }

    fn call_method(
        &self,
        name: &str,
        arguments: &[BslValue],
        _context: &mut CallContext<'_>,
    ) -> RtResult<BslValue> {
        if name.eq_ignore_ascii_case("Прочитать") || name.eq_ignore_ascii_case("Read") {
            read(self, arguments)?;
            Ok(BslValue::Undefined)
        } else if name.eq_ignore_ascii_case("Записать") || name.eq_ignore_ascii_case("Write")
        {
            write(self, arguments)?;
            Ok(BslValue::Undefined)
        } else {
            Err(RtError::UnknownMethod {
                method: name.to_string(),
                receiver: DOCUMENT_TYPE.name,
            })
        }
    }

    // `ЗначениеЗаполнено(Новый ДокументPDF)` — измеренная ошибка «Проверка
    // мутабельных значений на заполненность не поддерживается»; её отдаёт
    // реализация протокола по умолчанию.
}

impl ObjectProtocol for PagesObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &PAGES_TYPE
    }

    fn call_method(
        &self,
        name: &str,
        arguments: &[BslValue],
        _context: &mut CallContext<'_>,
    ) -> RtResult<BslValue> {
        if name.eq_ignore_ascii_case("Количество") || name.eq_ignore_ascii_case("Count") {
            page_count(self).map(|len| BslValue::number_from_i64(len as i64))
        } else if name.eq_ignore_ascii_case("Получить") || name.eq_ignore_ascii_case("Get")
        {
            match arguments {
                [index] => page_get(self, index),
                _ => Err(RtError::MethodNotApplicable {
                    method: "Получить",
                    receiver: PAGES_TYPE.name,
                }),
            }
        } else if name.eq_ignore_ascii_case("Индекс") || name.eq_ignore_ascii_case("IndexOf")
        {
            match arguments {
                [page] => page_index_of(self, page),
                _ => Err(RtError::MethodNotApplicable {
                    method: "Индекс",
                    receiver: PAGES_TYPE.name,
                }),
            }
        } else {
            Err(RtError::UnknownMethod {
                method: name.to_string(),
                receiver: PAGES_TYPE.name,
            })
        }
    }

    fn get_index(&self, index: &BslValue) -> RtResult<BslValue> {
        page_at(self, page_index_arg(index)?)
    }

    fn collection_len(&self) -> RtResult<usize> {
        page_count(self)
    }

    // Коллекция страниц судится ПО ДЛИНЕ (измерено): три страницы дали
    // «Да», пустое дерево — «Нет».
    fn is_filled(&self) -> RtResult<bool> {
        Ok(page_count(self)? > 0)
    }

    // Два отдельных чтения `Док.Страницы` равны (измерено) — окно в тот же
    // документ.
    fn identity_key(&self) -> Option<(usize, usize)> {
        Some((state_addr(&self.state), 0))
    }
}

/// Номер элемента из значения-индекса — та же семантика, что у `[]`
/// встроенных коллекций.
fn page_index_arg(index: &BslValue) -> RtResult<usize> {
    let BslValue::Number(number) = index else {
        return Err(RtError::BadIndex);
    };
    let index = number.to_i64_exact().ok_or(RtError::BadIndex)?;
    usize::try_from(index).map_err(|_| RtError::BadIndex)
}

impl ObjectProtocol for PageObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &PAGE_TYPE
    }

    fn get_property(&self, name: &str, _context: &mut CallContext<'_>) -> RtResult<BslValue> {
        page_property(self, name)
    }

    // Страница — ССЫЛКА на место в документе: `Страницы[0] = Страницы[0]`
    // — «Да», `Страницы[0] = Страницы[1]` — «Нет» (измерено).
    fn identity_key(&self) -> Option<(usize, usize)> {
        Some((state_addr(&self.state), self.index))
    }
}

impl ObjectProtocol for AttachmentsObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &ATTACHMENTS_TYPE
    }

    fn call_method(
        &self,
        name: &str,
        arguments: &[BslValue],
        _context: &mut CallContext<'_>,
    ) -> RtResult<BslValue> {
        if name.eq_ignore_ascii_case("Количество") || name.eq_ignore_ascii_case("Count") {
            attachment_count(self).map(|len| BslValue::number_from_i64(len as i64))
        } else if name.eq_ignore_ascii_case("Получить") || name.eq_ignore_ascii_case("Get")
        {
            match arguments {
                [index] => attachment_get(self, index),
                _ => Err(RtError::MethodNotApplicable {
                    method: "Получить",
                    receiver: ATTACHMENTS_TYPE.name,
                }),
            }
        } else if name.eq_ignore_ascii_case("Найти") || name.eq_ignore_ascii_case("Find") {
            attachment_find(self, arguments)
        } else if name.eq_ignore_ascii_case("Добавить") || name.eq_ignore_ascii_case("Add")
        {
            attachment_add(self, arguments)?;
            Ok(BslValue::Undefined)
        } else if name.eq_ignore_ascii_case("Удалить") || name.eq_ignore_ascii_case("Delete")
        {
            attachment_delete(self, arguments)?;
            Ok(BslValue::Undefined)
        } else if name.eq_ignore_ascii_case("Очистить") || name.eq_ignore_ascii_case("Clear")
        {
            if !arguments.is_empty() {
                return Err(RtError::MethodNotApplicable {
                    method: "Очистить",
                    receiver: ATTACHMENTS_TYPE.name,
                });
            }
            attachment_clear(self)?;
            Ok(BslValue::Undefined)
        } else if name.eq_ignore_ascii_case("Индекс") || name.eq_ignore_ascii_case("IndexOf")
        {
            match arguments {
                [item] => attachment_index_of(self, item),
                _ => Err(RtError::MethodNotApplicable {
                    method: "Индекс",
                    receiver: ATTACHMENTS_TYPE.name,
                }),
            }
        } else {
            Err(RtError::UnknownMethod {
                method: name.to_string(),
                receiver: ATTACHMENTS_TYPE.name,
            })
        }
    }

    fn get_index(&self, index: &BslValue) -> RtResult<BslValue> {
        attachment_at(self, page_index_arg(index)?)
    }

    fn collection_len(&self) -> RtResult<usize> {
        attachment_count(self)
    }

    // Коллекция вложений — тоже по длине: измерено, что у документа с
    // пятью вложениями `ЗначениеЗаполнено` даёт «Да».
    fn is_filled(&self) -> RtResult<bool> {
        Ok(attachment_count(self)? > 0)
    }

    // Коллекция и её элементы держат тот же `Rc`, поэтому `Док.Вложения =
    // Док.Вложения` — «Да» (измерено).
    fn identity_key(&self) -> Option<(usize, usize)> {
        Some((state_addr(&self.items), 0))
    }
}

impl ObjectProtocol for AttachmentObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &ATTACHMENT_TYPE
    }

    fn get_property(&self, name: &str, _context: &mut CallContext<'_>) -> RtResult<BslValue> {
        attachment_property(self, name)
    }

    fn set_property(
        &self,
        name: &str,
        value: BslValue,
        _context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        set_attachment_property(self, name, &value)
    }

    fn identity_key(&self) -> Option<(usize, usize)> {
        Some((state_addr(&self.items), self.index))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bsl_rt::pdf::{PaintMode, PdfDocument, PdfFont};
    use std::path::PathBuf;

    fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack
            .windows(needle.len())
            .position(|window| window == needle)
    }

    // --- чтение контейнера --------------------------------------------

    /// Собрать классический файл из готовых тел объектов: заголовок,
    /// объекты подряд, таблица `xref` и трейлер. Ровно то, что делает
    /// [`PdfDocument::write`], только без содержимого страниц — здесь
    /// проверяется КОНТЕЙНЕР.
    fn build_classic(objects: &[(u32, Vec<u8>)], trailer_extra: &str) -> Vec<u8> {
        let mut out = b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n".to_vec();
        let size = objects.iter().map(|(n, _)| *n).max().unwrap_or(0) + 1;
        let mut offsets = vec![0usize; size as usize];
        for (number, body) in objects {
            offsets[*number as usize] = out.len();
            out.extend_from_slice(format!("{number} 0 obj\n").as_bytes());
            out.extend_from_slice(body);
            out.extend_from_slice(b"\nendobj\n");
        }
        let xref = out.len();
        out.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
        for offset in offsets.iter().skip(1) {
            out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        out.extend_from_slice(
            format!("trailer\n<< /Size {size} /Root 1 0 R{trailer_extra} >>\nstartxref\n{xref}\n%%EOF\n")
                .as_bytes(),
        );
        out
    }

    fn empty_content() -> Vec<u8> {
        b"<< /Length 0 >>\nstream\n\nendstream".to_vec()
    }

    /// Однолистовой файл с заданным словарём страницы.
    fn one_page(page: &str) -> Vec<u8> {
        build_classic(
            &[
                (1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()),
                (2, b"<< /Type /Pages /Kids [ 3 0 R ] /Count 1 >>".to_vec()),
                (3, page.as_bytes().to_vec()),
                (4, empty_content()),
            ],
            "",
        )
    }

    /// Собственный вывод писателя обязан читаться собственным читателем:
    /// число страниц и их размеры сходятся с тем, что было задано.
    #[test]
    fn pdf_reader_reads_the_writer_output() {
        let mut doc = PdfDocument::new();
        let first = doc.add_page(595.32, 841.92).unwrap();
        doc.text(first, 40.0, 800.0, PdfFont::Courier, 12.0, "Накладная № 7")
            .unwrap();
        let second = doc.add_page(841.92, 595.32).unwrap();
        doc.rect(second, 10.0, 10.0, 100.0, 50.0, PaintMode::Stroke)
            .unwrap();
        let bytes = doc.write().unwrap();

        let file = PdfFile::parse(&bytes).expect("свой же вывод обязан читаться");
        assert_eq!(file.page_count(), 2);
        assert_eq!(file.page(0).unwrap().width_mm(), 210);
        assert_eq!(file.page(0).unwrap().height_mm(), 297);
        assert_eq!(file.page(1).unwrap().width_mm(), 297);
        assert_eq!(file.page(1).unwrap().height_mm(), 210);
        assert_eq!(file.page(0).unwrap().rotate(), 0);
    }

    /// ВСЕ закоммиченные файлы, снятые с платформы, обязаны читаться, и
    /// число страниц в каждом — совпасть с тем, ради чего файл снимали
    /// (происхождение всех — `tests/conformance/pdf/capture-platform-pdf*.bsl`).
    #[test]
    fn pdf_reader_reads_every_committed_platform_file() {
        let expected: &[(&str, usize)] = &[
            // Три файла задачи о вложениях: снятый с платформы и два
            // собранных НАШИМ писателем поверх чужой основы (см.
            // `make-open-bsl-attachments.bsl`).
            ("attach-platform.pdf", 1),
            ("attach-open-bsl.pdf", 1),
            ("attach-open-bsl-xrefstream.pdf", 2),
            ("platform-simple.pdf", 1),
            ("probe-align.pdf", 1),
            ("probe-big.pdf", 4),
            ("probe-border.pdf", 1),
            ("probe-color.pdf", 1),
            ("probe-colwidth.pdf", 1),
            ("probe-empty.pdf", 1),
            ("probe-fit.pdf", 1),
            ("probe-font.pdf", 1),
            ("probe-grid.pdf", 1),
            ("probe-landscape.pdf", 1),
            ("probe-line.pdf", 1),
            ("probe-margins.pdf", 1),
            ("probe-merge.pdf", 1),
            ("probe-number.pdf", 1),
            ("probe-numeric.pdf", 1),
            ("probe-pages.pdf", 2),
            ("probe-rowheight.pdf", 1),
            ("probe-wide.pdf", 2),
        ];
        let dir = PathBuf::from("../../tests/conformance/pdf");
        let mut checked = 0usize;
        for (name, pages) in expected {
            let bytes = std::fs::read(dir.join(name)).unwrap_or_else(|e| {
                panic!("файл {name} обязан лежать в дереве: {e}");
            });
            let file = PdfFile::parse(&bytes)
                .unwrap_or_else(|e| panic!("платформенный файл {name} обязан читаться: {e:?}"));
            assert_eq!(file.page_count(), *pages, "число страниц в {name}");
            for i in 0..file.page_count() {
                let page = file.page(i).unwrap();
                assert!(
                    page.width_mm() > 0 && page.height_mm() > 0,
                    "страница {i} файла {name} без размера"
                );
            }
            checked += 1;
        }
        // Ни один файл каталога не должен остаться неучтённым: новый
        // снимок с платформы обязан попасть в таблицу выше.
        let on_disk = std::fs::read_dir(&dir)
            .expect("каталог со снимками обязан существовать")
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().is_some_and(|x| x == "pdf"))
            .count();
        assert_eq!(checked, on_disk, "в каталоге появился неучтённый PDF");
    }

    /// Альбомный лист платформы: `probe-landscape.pdf` снят с документа,
    /// у которого `ОриентацияСтраницы = Ландшафт`.
    #[test]
    fn pdf_reader_sees_the_platform_landscape_page() {
        let bytes = std::fs::read("../../tests/conformance/pdf/probe-landscape.pdf").unwrap();
        let file = PdfFile::parse(&bytes).unwrap();
        let page = file.page(0).unwrap();
        assert_eq!((page.width_mm(), page.height_mm()), (297, 210));
    }

    /// Размеры отдаются в миллиметрах с округлением к ближайшему, начало
    /// рамки не важно (измерено, см. обзор модуля).
    #[test]
    fn pdf_reader_converts_points_to_millimetres() {
        let cases: &[(&str, i64, i64)] = &[
            ("[ 0 0 100 200 ]", 35, 71),
            ("[ 10 20 110 220 ]", 35, 71),
            ("[ 0 0 100.62 200 ]", 35, 71),
            ("[ 0 0 100.64 200 ]", 36, 71),
            ("[ 0 0 595.32 841.92 ]", 210, 297),
        ];
        for (rect, width, height) in cases {
            let pdf = one_page(&format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox {rect} /Contents 4 0 R >>"
            ));
            let file = PdfFile::parse(&pdf).unwrap();
            let page = file.page(0).unwrap();
            assert_eq!(
                (page.width_mm(), page.height_mm()),
                (*width, *height),
                "рамка {rect}"
            );
        }
    }

    /// Рамка без площади — ноль на ноль по обеим сторонам сразу, а страница
    /// без рамки вовсе — US Letter (измерено).
    #[test]
    fn pdf_reader_gives_zero_for_degenerate_boxes() {
        for rect in ["[ 595.32 0 0 841.92 ]", "[ 0 0 0 0 ]", "[ 0 0 -100 -200 ]"] {
            let pdf = one_page(&format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox {rect} /Contents 4 0 R >>"
            ));
            let file = PdfFile::parse(&pdf).unwrap();
            let page = file.page(0).unwrap();
            assert_eq!((page.width_mm(), page.height_mm()), (0, 0), "рамка {rect}");
        }
        let pdf = one_page("<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>");
        let file = PdfFile::parse(&pdf).unwrap();
        let page = file.page(0).unwrap();
        assert_eq!((page.width_mm(), page.height_mm()), (216, 279));
    }

    /// `/CropBox` задаёт видимую область и ПЕРЕСЕКАЕТСЯ с `/MediaBox`;
    /// обе рамки наследуются от узла `/Pages`, а `/Rotate` — нет
    /// (измерено).
    #[test]
    fn pdf_reader_inherits_boxes_but_not_rotation() {
        let pdf = build_classic(
            &[
                (1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()),
                (
                    2,
                    b"<< /Type /Pages /Kids [ 3 0 R 4 0 R 5 0 R ] /Count 3 \
                       /MediaBox [ 0 0 595.32 841.92 ] /CropBox [ 50 60 545.32 781.92 ] \
                       /Rotate 90 >>"
                        .to_vec(),
                ),
                (
                    3,
                    b"<< /Type /Page /Parent 2 0 R /Contents 6 0 R >>".to_vec(),
                ),
                (
                    4,
                    b"<< /Type /Page /Parent 2 0 R /MediaBox [ 0 0 595.32 841.92 ] \
                       /CropBox [ -100 -100 700 900 ] /Contents 6 0 R >>"
                        .to_vec(),
                ),
                (
                    5,
                    b"<< /Type /Page /Parent 2 0 R /MediaBox [ 0 0 595.32 841.92 ] \
                       /Rotate 180 /Contents 6 0 R >>"
                        .to_vec(),
                ),
                (6, empty_content()),
            ],
            "",
        );
        let file = PdfFile::parse(&pdf).unwrap();
        assert_eq!(file.page_count(), 3);
        // Унаследованный `/CropBox`.
        let first = file.page(0).unwrap();
        assert_eq!((first.width_mm(), first.height_mm()), (175, 255));
        // Наследуется, но `/Rotate` узла на страницу НЕ переходит.
        assert_eq!(first.rotate(), 0);
        // `/CropBox` шире `/MediaBox` — берётся пересечение.
        let second = file.page(1).unwrap();
        assert_eq!((second.width_mm(), second.height_mm()), (210, 297));
        // Собственный `/Rotate` страницы виден.
        assert_eq!(file.page(2).unwrap().rotate(), 180);
    }

    /// `/Rotate` приводится к кратному прямому углу — все десять значений,
    /// снятых с платформы.
    #[test]
    fn pdf_reader_normalises_rotation_like_the_platform() {
        let cases: &[(&str, i64)] = &[
            ("0", 0),
            ("90", 90),
            ("180", 180),
            ("270", 270),
            ("-90", 270),
            ("450", 90),
            ("44", 0),
            ("45", 90),
            ("46", 90),
            ("135", 180),
            ("315", 0),
            ("350", 0),
        ];
        for (raw, expected) in cases {
            let pdf = one_page(&format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [ 0 0 595.32 841.92 ] \
                 /Rotate {raw} /Contents 4 0 R >>"
            ));
            let file = PdfFile::parse(&pdf).unwrap();
            assert_eq!(file.page(0).unwrap().rotate(), *expected, "/Rotate {raw}");
        }
    }

    /// Инкрементальное обновление: вторая таблица `xref` с `/Prev`
    /// перекрывает объект 2 и добавляет объект 5.
    #[test]
    fn pdf_reader_follows_the_prev_chain() {
        let base = build_classic(
            &[
                (1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()),
                (
                    2,
                    b"<< /Type /Pages /Kids [ 3 0 R ] /Count 1 /MediaBox [ 0 0 595.32 841.92 ] >>"
                        .to_vec(),
                ),
                (
                    3,
                    b"<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>".to_vec(),
                ),
                (4, empty_content()),
            ],
            "",
        );
        let previous = find(&base, b"xref\n0 ").expect("в базовом файле есть таблица");
        let mut out = base.clone();
        let five = out.len();
        out.extend_from_slice(
            b"5 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [ 0 0 841.92 595.32 ] \
              /Contents 4 0 R >>\nendobj\n",
        );
        let two = out.len();
        out.extend_from_slice(
            b"2 0 obj\n<< /Type /Pages /Kids [ 3 0 R 5 0 R ] /Count 2 \
              /MediaBox [ 0 0 595.32 841.92 ] >>\nendobj\n",
        );
        let xref = out.len();
        out.extend_from_slice(
            format!(
                "xref\n0 1\n0000000000 65535 f \n2 1\n{two:010} 00000 n \n5 1\n{five:010} 00000 n \n\
                 trailer\n<< /Size 6 /Root 1 0 R /Prev {previous} >>\nstartxref\n{xref}\n%%EOF\n"
            )
            .as_bytes(),
        );

        let file = PdfFile::parse(&out).expect("инкрементальное обновление обязано читаться");
        assert_eq!(file.page_count(), 2);
        assert_eq!(file.page(0).unwrap().width_mm(), 210);
        assert_eq!(file.page(1).unwrap().width_mm(), 297);
        // Базовый файл в одиночку по-прежнему даёт одну страницу — значит
        // прочиталась именно цепочка, а не хвост.
        assert_eq!(PdfFile::parse(&base).unwrap().page_count(), 1);
    }

    /// Файл PDF 1.5: `xref`-поток с предиктором PNG «Up» и объектный поток,
    /// в котором лежат каталог, узел `/Pages` и обе страницы.
    ///
    /// Собран байтами прямо здесь по разделам 7.5.7 и 7.5.8 спецификации:
    /// ни qpdf, ни pikepdf на машине нет, а снимать такой файл с платформы
    /// нечем — её писатель кладёт классическую таблицу.
    #[test]
    fn pdf_reader_reads_xref_stream_with_object_stream() {
        let inner: &[(u32, &str)] = &[
            (1, "<< /Type /Catalog /Pages 2 0 R >>"),
            (
                2,
                "<< /Type /Pages /Kids [ 3 0 R 4 0 R ] /Count 2 /MediaBox [ 0 0 595.32 841.92 ] >>",
            ),
            (3, "<< /Type /Page /Parent 2 0 R /Contents 6 0 R >>"),
            (
                4,
                "<< /Type /Page /Parent 2 0 R /MediaBox [ 0 0 841.92 595.32 ] /Contents 6 0 R >>",
            ),
        ];
        let mut head = String::new();
        let mut body = String::new();
        for (number, text) in inner {
            head.push_str(&format!("{number} {} ", body.len()));
            body.push_str(text);
            body.push(' ');
        }
        let first = head.len();
        let objstm_plain = format!("{head}{body}").into_bytes();
        let objstm = zlib_compress(&objstm_plain);

        let mut out = b"%PDF-1.5\n%\xe2\xe3\xcf\xd3\n".to_vec();
        let at_five = out.len();
        out.extend_from_slice(
            format!(
                "5 0 obj\n<< /Type /ObjStm /N {} /First {first} /Filter /FlateDecode /Length {} >>\nstream\n",
                inner.len(),
                objstm.len()
            )
            .as_bytes(),
        );
        out.extend_from_slice(&objstm);
        out.extend_from_slice(b"\nendstream\nendobj\n");
        let at_six = out.len();
        out.extend_from_slice(b"6 0 obj\n");
        out.extend_from_slice(&empty_content());
        out.extend_from_slice(b"\nendobj\n");
        let at_xref = out.len();

        // Записи шириной [1 4 2]; строки предсказаны фильтром PNG «Up»
        // (тип 2), то есть каждая строка — разность с предыдущей.
        let mut rows: Vec<[u8; 7]> = Vec::new();
        let mut push = |kind: u8, second: u32, third: u16| {
            let mut row = [0u8; 7];
            row[0] = kind;
            row[1..5].copy_from_slice(&second.to_be_bytes());
            row[5..7].copy_from_slice(&third.to_be_bytes());
            rows.push(row);
        };
        push(0, 0, 65535);
        for index in 0..inner.len() {
            push(2, 5, index as u16);
        }
        push(1, at_five as u32, 0);
        push(1, at_six as u32, 0);
        push(1, at_xref as u32, 0);
        let mut predicted = Vec::new();
        let mut previous = [0u8; 7];
        for row in &rows {
            predicted.push(2u8);
            for i in 0..7 {
                predicted.push(row[i].wrapping_sub(previous[i]));
            }
            previous = *row;
        }
        let xref_stream = zlib_compress(&predicted);
        out.extend_from_slice(
            format!(
                "7 0 obj\n<< /Type /XRef /Size 8 /W [ 1 4 2 ] /Root 1 0 R /Filter /FlateDecode \
                 /DecodeParms << /Predictor 12 /Columns 7 >> /Length {} >>\nstream\n",
                xref_stream.len()
            )
            .as_bytes(),
        );
        out.extend_from_slice(&xref_stream);
        out.extend_from_slice(b"\nendstream\nendobj\n");
        out.extend_from_slice(format!("startxref\n{at_xref}\n%%EOF\n").as_bytes());

        let file = PdfFile::parse(&out).expect("xref-поток с ObjStm обязан читаться");
        assert_eq!(file.page_count(), 2);
        assert_eq!(file.page(0).unwrap().width_mm(), 210);
        assert_eq!(file.page(1).unwrap().width_mm(), 297);
    }

    /// Зашифрованный файл — ЧЕСТНЫЙ ОТКАЗ, а не попытка прочитать мусор:
    /// расшифровки здесь нет.
    #[test]
    fn pdf_reader_refuses_an_encrypted_file() {
        let pdf = build_classic(
            &[
                (1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()),
                (
                    2,
                    b"<< /Type /Pages /Kids [ 3 0 R ] /Count 1 /MediaBox [ 0 0 595.32 841.92 ] >>"
                        .to_vec(),
                ),
                (
                    3,
                    b"<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>".to_vec(),
                ),
                (4, empty_content()),
                (
                    5,
                    b"<< /Filter /Standard /V 1 /R 2 /O <00> /U <00> /P -1 >>".to_vec(),
                ),
            ],
            " /Encrypt 5 0 R",
        );
        let error = PdfFile::parse(&pdf).expect_err("зашифрованный файл читаться не должен");
        let RtError::Pdf(text) = error else {
            panic!("ожидалась ошибка PDF");
        };
        assert!(text.contains("зашифрован"), "текст ошибки: {text}");
    }

    /// Неподдержанный фильтр отвергается ПО ИМЕНИ — чтобы из текста было
    /// видно, чего не хватает.
    #[test]
    fn pdf_reader_names_the_unsupported_filter() {
        let inner = b"<< /Type /Catalog /Pages 2 0 R >>";
        let pdf = build_classic(
            &[
                (
                    1,
                    format!(
                        "<< /Type /ObjStm /N 1 /First 4 /Filter /LZWDecode /Length {} >>\nstream\n{}\nendstream",
                        inner.len() + 4,
                        format_args!("1 0 {}", String::from_utf8_lossy(inner))
                    )
                    .into_bytes(),
                ),
                (2, b"<< /Type /Pages /Kids [ ] /Count 0 >>".to_vec()),
            ],
            "",
        );
        // Каталог здесь — сам объект 1, то есть словарь потока; разбор
        // упрётся в фильтр только если добраться до потока, поэтому файл
        // собран так, чтобы `/Root` вёл именно в него.
        let error = PdfFile::parse(&pdf);
        // Каталог-поток — вырожденный случай; важно, что разбор кончается
        // ошибкой, а не паникой.
        assert!(error.is_err());

        // А вот прямая проверка фильтра: поток страницы с /LZWDecode.
        let mut reader = Reader::new(&pdf).unwrap();
        let dict = vec![(
            "Filter".to_string(),
            PdfValue::Name("LZWDecode".to_string()),
        )];
        let error = reader
            .decode_stream(&dict, b"whatever")
            .expect_err("LZW не поддержан");
        let RtError::Pdf(text) = error else {
            panic!("ожидалась ошибка PDF");
        };
        assert!(text.contains("LZWDecode"), "текст ошибки: {text}");
    }

    /// Битые входы: разбор обязан кончиться ошибкой, а не паникой и не
    /// зависанием.
    #[test]
    fn pdf_reader_rejects_broken_input_without_hanging() {
        let good = one_page(
            "<< /Type /Page /Parent 2 0 R /MediaBox [ 0 0 595.32 841.92 ] /Contents 4 0 R >>",
        );
        let cases: Vec<(&str, Vec<u8>)> = vec![
            ("пусто", Vec::new()),
            ("не PDF", b"just some bytes".to_vec()),
            (
                "заголовок без содержимого",
                b"%PDF-1.4\nnonsense\n".to_vec(),
            ),
            ("обрезан на половине", good[..good.len() / 2].to_vec()),
            ("обрезан на четверти", good[..good.len() / 4].to_vec()),
            ("startxref в никуда", {
                let mut bytes = good.clone();
                let at = find(&bytes, b"startxref\n").unwrap();
                bytes.truncate(at);
                bytes.extend_from_slice(b"startxref\n999999999\n%%EOF\n");
                bytes
            }),
            (
                "трейлер без /Root",
                build_classic(
                    &[(1, b"<< /Type /Pages /Kids [ ] /Count 0 >>".to_vec())],
                    "",
                )
                .split(|b| *b == b'\n')
                .map(|line| {
                    if line.starts_with(b"<< /Size") {
                        b"<< /Size 2 >>".to_vec()
                    } else {
                        line.to_vec()
                    }
                })
                .collect::<Vec<_>>()
                .join(&b'\n'),
            ),
            (
                "цикл в дереве страниц",
                build_classic(
                    &[
                        (1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()),
                        (2, b"<< /Type /Pages /Kids [ 3 0 R ] /Count 1 >>".to_vec()),
                        (3, b"<< /Type /Pages /Kids [ 2 0 R ] /Count 1 >>".to_vec()),
                    ],
                    "",
                ),
            ),
            (
                "циклическая косвенная ссылка",
                build_classic(
                    &[
                        (1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()),
                        (2, b"2 0 R".to_vec()),
                    ],
                    "",
                ),
            ),
            (
                "словарь без закрытия",
                build_classic(
                    &[
                        (1, b"<< /Type /Catalog /Pages 2 0 R".to_vec()),
                        (2, b"<< /Type /Pages /Kids [ ] /Count 0 >>".to_vec()),
                    ],
                    "",
                ),
            ),
        ];
        for (what, bytes) in cases {
            match PdfFile::parse(&bytes) {
                Err(RtError::Pdf(text)) => {
                    assert!(!text.is_empty(), "у ошибки «{what}» пустой текст");
                }
                Err(other) => panic!("«{what}»: ожидалась ошибка PDF, получено {other:?}"),
                Ok(file) => panic!("«{what}» разобрался в {} страниц", file.page_count()),
            }
        }
    }

    /// Мусор на входе не должен уводить разбор в панику ни на одном
    /// префиксе живого файла: грубая, но действенная проверка на то, что
    /// каждый обрыв обработан.
    #[test]
    fn pdf_reader_survives_every_prefix_of_a_good_file() {
        let good = one_page(
            "<< /Type /Page /Parent 2 0 R /MediaBox [ 0 0 595.32 841.92 ] /Contents 4 0 R >>",
        );
        for cut in 0..good.len() {
            // Результат не важен: важно, что вызов вернулся.
            let _ = PdfFile::parse(&good[..cut]);
        }
        // И то же самое с одним испорченным байтом в каждой позиции.
        for at in 0..good.len() {
            let mut bytes = good.clone();
            bytes[at] = b'#';
            let _ = PdfFile::parse(&bytes);
        }
    }

    /// Числа, которые проба кладёт в словари вместо правильных. `None` —
    /// вычисленное значение, то есть законный файл.
    #[derive(Debug, Default, Clone, Copy)]
    struct Boundary<'a> {
        /// `/N` объектного потока.
        n: Option<&'a str>,
        /// `/First` объектного потока.
        first: Option<&'a str>,
        /// `/Length` объектного потока.
        objstm_length: Option<&'a str>,
        /// `/Size` xref-потока.
        size: Option<&'a str>,
        /// Целиком ключ `/Index [ ... ]`: по умолчанию его нет вовсе и
        /// действует умолчание `[0 /Size]` (раздел 7.5.8.2).
        index: Option<&'a str>,
        /// Целиком массив `/W`.
        widths: Option<&'a str>,
        /// Целиком словарь `/DecodeParms` xref-потока.
        parms: Option<&'a str>,
        /// `/Length` xref-потока.
        xref_length: Option<&'a str>,
    }

    /// Собрать файл PDF 1.5 «объектный поток плюс xref-поток» байтами по
    /// разделам 7.5.7 и 7.5.8 спецификации, подставив в числовые ключи то,
    /// что просит проба. Без подстановок файл законный и читается — значит
    /// в пробе ломает именно подставленное число, а не сборка.
    fn build_boundary_pdf(boundary: Boundary<'_>) -> Vec<u8> {
        let pick = |over: Option<&str>, computed: String| -> String {
            over.map(str::to_string).unwrap_or(computed)
        };
        // Каталог и узел `/Pages` лежат внутри объектного потока 1,
        // xref-поток — объект 4.
        let inner: &[(u32, &str)] = &[
            (2, "<< /Type /Catalog /Pages 3 0 R >>"),
            (3, "<< /Type /Pages /Kids [ ] /Count 0 >>"),
        ];
        let mut head = String::new();
        let mut body = String::new();
        for (number, text) in inner {
            head.push_str(&format!("{number} {} ", body.len()));
            body.push_str(text);
            body.push(' ');
        }
        let first = head.len();
        let objstm = zlib_compress(format!("{head}{body}").as_bytes());

        let mut out = b"%PDF-1.5\n%\xe2\xe3\xcf\xd3\n".to_vec();
        let at_objstm = out.len();
        out.extend_from_slice(
            format!(
                "1 0 obj\n<< /Type /ObjStm /N {} /First {} /Filter /FlateDecode /Length {} >>\nstream\n",
                pick(boundary.n, inner.len().to_string()),
                pick(boundary.first, first.to_string()),
                pick(boundary.objstm_length, objstm.len().to_string()),
            )
            .as_bytes(),
        );
        out.extend_from_slice(&objstm);
        out.extend_from_slice(b"\nendstream\nendobj\n");
        let at_xref = out.len();

        // Записи шириной [1 4 2], строки предсказаны фильтром PNG «Up».
        let mut rows: Vec<[u8; 7]> = Vec::new();
        let mut push = |kind: u8, second: u32, third: u16| {
            let mut row = [0u8; 7];
            row[0] = kind;
            row[1..5].copy_from_slice(&second.to_be_bytes());
            row[5..7].copy_from_slice(&third.to_be_bytes());
            rows.push(row);
        };
        push(0, 0, 65535);
        push(1, at_objstm as u32, 0);
        push(2, 1, 0);
        push(2, 1, 1);
        push(1, at_xref as u32, 0);
        let mut predicted = Vec::new();
        let mut previous = [0u8; 7];
        for row in &rows {
            predicted.push(2u8);
            for i in 0..7 {
                predicted.push(row[i].wrapping_sub(previous[i]));
            }
            previous = *row;
        }
        let xref_stream = zlib_compress(&predicted);
        out.extend_from_slice(
            format!(
                "4 0 obj\n<< /Type /XRef /Size {} {} /W {} /Root 2 0 R /Filter /FlateDecode \
                 /DecodeParms {} /Length {} >>\nstream\n",
                pick(boundary.size, rows.len().to_string()),
                boundary.index.unwrap_or(""),
                pick(boundary.widths, "[ 1 4 2 ]".to_string()),
                pick(boundary.parms, "<< /Predictor 12 /Columns 7 >>".to_string()),
                pick(boundary.xref_length, xref_stream.len().to_string()),
            )
            .as_bytes(),
        );
        out.extend_from_slice(&xref_stream);
        out.extend_from_slice(b"\nendstream\nendobj\n");
        out.extend_from_slice(format!("startxref\n{at_xref}\n%%EOF\n").as_bytes());
        out
    }

    /// Граничные числа ПРЯМО ИЗ ФАЙЛА в каждом числовом ключе разбора:
    /// разбор обязан вернуться с `RtError`, а не переполнить арифметику и
    /// не выделить память по объявленному в файле размеру. Проверки
    /// префиксов и однобайтовых порч этого не ловят — двадцатизначное
    /// число из хорошего файла так не получить, поэтому словари здесь
    /// собираются байтами.
    ///
    /// Случаи, помеченные как обязанные упасть, — ровно те, где иначе
    /// сложение уходит за `i64` или строка предиктора выделяется по
    /// объявленным гигабайтам; там `Ok` означал бы, что защита снята.
    #[test]
    fn pdf_reader_rejects_boundary_numbers_in_dictionaries() {
        assert_eq!(
            PdfFile::parse(&build_boundary_pdf(Boundary::default()))
                .expect("базовый файл пробы обязан читаться")
                .page_count(),
            0
        );

        let cases: &[(&str, Boundary, bool)] = &[
            (
                "первый номер /Index у i64::MAX",
                Boundary {
                    index: Some("/Index [ 9223372036854775807 2 ]"),
                    ..Boundary::default()
                },
                true,
            ),
            (
                "первый номер /Index на единицу ниже i64::MAX",
                Boundary {
                    index: Some("/Index [ 9223372036854775806 2 ]"),
                    ..Boundary::default()
                },
                true,
            ),
            (
                "первый номер /Index за u32",
                Boundary {
                    index: Some("/Index [ 4294967296 2 ]"),
                    ..Boundary::default()
                },
                true,
            ),
            (
                "первый номер /Index на 10^11",
                Boundary {
                    index: Some("/Index [ 100000000000 1 ]"),
                    ..Boundary::default()
                },
                true,
            ),
            (
                "число записей /Index у i64::MAX",
                Boundary {
                    index: Some("/Index [ 0 9223372036854775807 ]"),
                    ..Boundary::default()
                },
                true,
            ),
            (
                "ширина поля /W у i64::MAX",
                Boundary {
                    widths: Some("[ 9223372036854775807 4 2 ]"),
                    ..Boundary::default()
                },
                true,
            ),
            (
                "лишнее поле /W на 10^11",
                Boundary {
                    widths: Some("[ 1 4 2 100000000000 ]"),
                    ..Boundary::default()
                },
                true,
            ),
            (
                "/Size у i64::MAX",
                Boundary {
                    size: Some("9223372036854775807"),
                    ..Boundary::default()
                },
                true,
            ),
            (
                "/Size на 10^11",
                Boundary {
                    size: Some("100000000000"),
                    ..Boundary::default()
                },
                true,
            ),
            (
                "/N объектного потока у i64::MAX",
                Boundary {
                    n: Some("9223372036854775807"),
                    ..Boundary::default()
                },
                true,
            ),
            (
                "/N объектного потока за u32",
                Boundary {
                    n: Some("4294967296"),
                    ..Boundary::default()
                },
                true,
            ),
            (
                "/First объектного потока у i64::MAX",
                Boundary {
                    first: Some("9223372036854775807"),
                    ..Boundary::default()
                },
                true,
            ),
            (
                "/First объектного потока на 10^11",
                Boundary {
                    first: Some("100000000000"),
                    ..Boundary::default()
                },
                true,
            ),
            (
                "/Colors предиктора у i64::MAX",
                Boundary {
                    parms: Some(
                        "<< /Predictor 12 /Colors 9223372036854775807 /BitsPerComponent 1 /Columns 1 >>",
                    ),
                    ..Boundary::default()
                },
                true,
            ),
            (
                "/Columns предиктора на 10^11",
                Boundary {
                    parms: Some("<< /Predictor 12 /Columns 100000000000 >>"),
                    ..Boundary::default()
                },
                true,
            ),
            (
                "/Columns предиктора у i64::MAX",
                Boundary {
                    parms: Some("<< /Predictor 12 /Columns 9223372036854775807 >>"),
                    ..Boundary::default()
                },
                true,
            ),
            (
                "/BitsPerComponent предиктора у i64::MAX",
                Boundary {
                    parms: Some("<< /Predictor 12 /BitsPerComponent 9223372036854775807 /Columns 7 >>"),
                    ..Boundary::default()
                },
                true,
            ),
            (
                "/BitsPerComponent предиктора за u32",
                Boundary {
                    parms: Some("<< /Predictor 12 /BitsPerComponent 4294967296 /Columns 7 >>"),
                    ..Boundary::default()
                },
                true,
            ),
            (
                "/Colors предиктора за u32",
                Boundary {
                    parms: Some("<< /Predictor 12 /Colors 4294967296 /Columns 7 >>"),
                    ..Boundary::default()
                },
                true,
            ),
            // Номер предиктора больше десяти — это по-прежнему строки PNG
            // (раздел 7.4.4.4), поэтому такой файл имеет право прочитаться.
            (
                "/Predictor у i64::MAX",
                Boundary {
                    parms: Some("<< /Predictor 9223372036854775807 /Columns 7 >>"),
                    ..Boundary::default()
                },
                false,
            ),
            // Соврамший `/Length` спасает поиск `endstream`, и это
            // измеренное поведение платформы, а не недосмотр.
            (
                "/Length объектного потока у i64::MAX",
                Boundary {
                    objstm_length: Some("9223372036854775807"),
                    ..Boundary::default()
                },
                false,
            ),
            (
                "/Length xref-потока на 10^11",
                Boundary {
                    xref_length: Some("100000000000"),
                    ..Boundary::default()
                },
                false,
            ),
        ];

        for (what, boundary, must_fail) in cases {
            match PdfFile::parse(&build_boundary_pdf(*boundary)) {
                Err(RtError::Pdf(text)) => {
                    assert!(!text.is_empty(), "у ошибки «{what}» пустой текст");
                }
                Err(other) => panic!("«{what}»: ожидалась ошибка PDF, получено {other:?}"),
                Ok(file) => assert!(
                    !must_fail,
                    "«{what}» разобрался в {} страниц",
                    file.page_count()
                ),
            }
        }
    }

    /// Пустое дерево страниц — законный файл с нулём страниц (измерено).
    #[test]
    fn pdf_reader_accepts_an_empty_page_tree() {
        let pdf = build_classic(
            &[
                (1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()),
                (2, b"<< /Type /Pages /Kids [ ] /Count 0 >>".to_vec()),
            ],
            "",
        );
        let file = PdfFile::parse(&pdf).expect("пустое дерево — законный файл");
        assert_eq!(file.page_count(), 0);
    }

    /// `/Length` косвенной ссылкой — так пишет сама платформа
    /// (`platform-simple.pdf`), и так же обязан читаться собранный здесь
    /// файл; соврамший `/Length` не должен рушить разбор.
    #[test]
    fn pdf_reader_takes_length_from_an_indirect_object_and_survives_a_wrong_one() {
        let pdf = build_classic(
            &[
                (1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()),
                (
                    2,
                    b"<< /Type /Pages /Kids [ 3 0 R ] /Count 1 /MediaBox [ 0 0 595.32 841.92 ] >>"
                        .to_vec(),
                ),
                (
                    3,
                    b"<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>".to_vec(),
                ),
                (4, b"<< /Length 5 0 R >>\nstream\nq Q\nendstream".to_vec()),
                (5, b"3".to_vec()),
            ],
            "",
        );
        assert_eq!(PdfFile::parse(&pdf).unwrap().page_count(), 1);

        // Подмена РАВНОЙ ДЛИНЫ: «/Length 5 0 R» и «/Length 99999» — по
        // тринадцать байтов, иначе поехали бы смещения в таблице xref и
        // проба мерила бы не то.
        let mut lying = pdf.clone();
        let at = find(&lying, b"/Length 5 0 R").expect("поток с косвенной длиной");
        lying[at..at + 13].copy_from_slice(b"/Length 99999");
        assert_eq!(PdfFile::parse(&lying).unwrap().page_count(), 1);
    }

    /// Синтаксис значений: имена с `#`, строки со скобками и escape'ами,
    /// шестнадцатеричные строки, логические значения и `null`.
    #[test]
    fn pdf_reader_parses_the_value_syntax() {
        let source =
            "[ /A#20B (в (скобках) \\) и \\101) <48656C6C6F> <4a4> true false null 1 2 R -3.5 ]"
                .as_bytes();
        let value = Lexer::new(source, 0).parse_value(0).unwrap();
        let PdfValue::Array(items) = value else {
            panic!("ожидался массив");
        };
        assert_eq!(items[0], PdfValue::Name("A B".to_string()));
        assert_eq!(
            items[1],
            PdfValue::Str("в (скобках) ) и A".as_bytes().to_vec())
        );
        assert_eq!(items[2], PdfValue::Str(b"Hello".to_vec()));
        // Нечётный хвост шестнадцатеричной строки дополняется нулём.
        assert_eq!(items[3], PdfValue::Str(vec![0x4A, 0x40]));
        assert_eq!(items[4], PdfValue::Bool(true));
        assert_eq!(items[5], PdfValue::Bool(false));
        assert_eq!(items[6], PdfValue::Null);
        assert_eq!(items[7], PdfValue::Ref(1));
        assert_eq!(items[8], PdfValue::Real(-3.5));
        assert_eq!(items.len(), 9);
    }

    /// КРУГ ЗАМЫКАЕТСЯ на файле самой платформы: `probe-margins.pdf` она
    /// записала из документа с полями 30, 10, 25 и 5 мм
    /// (`capture-platform-pdf-layout.bsl`, строки 241..244), и читатель
    /// обязан вернуть ровно их.
    #[test]
    fn pdf_reader_reads_back_the_platform_margins() {
        let bytes = std::fs::read("../../tests/conformance/pdf/probe-margins.pdf").unwrap();
        let file = PdfFile::parse(&bytes).unwrap();
        let page = file.page(0).unwrap();
        assert_eq!(page.margin(PdfMargin::Left), 30);
        assert_eq!(page.margin(PdfMargin::Right), 10);
        assert_eq!(page.margin(PdfMargin::Top), 25);
        assert_eq!(page.margin(PdfMargin::Bottom), 5);
        // А умолчание полей табличного документа — 10 мм со всех сторон.
        let empty = std::fs::read("../../tests/conformance/pdf/probe-empty.pdf").unwrap();
        let empty = PdfFile::parse(&empty).unwrap();
        let page = empty.page(0).unwrap();
        assert_eq!(
            [
                page.margin(PdfMargin::Left),
                page.margin(PdfMargin::Right),
                page.margin(PdfMargin::Top),
                page.margin(PdfMargin::Bottom),
            ],
            [10, 10, 10, 10]
        );
    }

    /// Правило полей целиком, все одиннадцать снятых страниц.
    #[test]
    fn pdf_reader_takes_margins_from_the_trim_box_only() {
        let a4 = "/MediaBox [ 0 0 595.32 841.92 ]";
        let shift = "/MediaBox [ 10 20 605.32 861.92 ]";
        let cases: &[(&str, [i64; 4])] = &[
            // Обе рамки объявлены и совпадают — как пишет платформа.
            (
                "/TrimBox [ 85.04 70.87 566.97 827.75 ] /BleedBox [ 85.04 70.87 566.97 827.75 ]",
                [30, 10, 25, 5],
            ),
            ("/TrimBox [ 85.04 70.87 566.97 827.75 ]", [30, 10, 25, 5]),
            // Один `/BleedBox` полей не даёт, как и один `/ArtBox`.
            ("/BleedBox [ 85.04 70.87 566.97 827.75 ]", [0, 0, 0, 0]),
            ("/ArtBox [ 85.04 70.87 566.97 827.75 ]", [0, 0, 0, 0]),
            // Рамки разные — побеждает `/TrimBox`.
            (
                "/TrimBox [ 56.7 56.7 538.62 785.22 ] /BleedBox [ 14.17 14.17 581.15 827.75 ]",
                [20, 20, 20, 20],
            ),
            // `/CropBox` двигает ДАЛЬНИЕ края, но не ближние.
            (
                "/CropBox [ 50 60 545.32 781.92 ] /TrimBox [ 85.04 70.87 495.32 731.92 ]",
                [30, 18, 25, 18],
            ),
            // Поля не поджимаются к нулю.
            ("/TrimBox [ -50 -50 700 900 ]", [-18, -37, -18, -20]),
            ("/TrimBox [ 0 0 595.32 841.92 ]", [0, 0, 0, 0]),
        ];
        for (extra, expected) in cases {
            let pdf = one_page(&format!(
                "<< /Type /Page /Parent 2 0 R {a4} {extra} /Contents 4 0 R >>"
            ));
            let file = PdfFile::parse(&pdf).unwrap();
            let page = file.page(0).unwrap();
            let got = [
                page.margin(PdfMargin::Left),
                page.margin(PdfMargin::Right),
                page.margin(PdfMargin::Top),
                page.margin(PdfMargin::Bottom),
            ];
            assert_eq!(got, *expected, "страница с {extra}");
        }

        // Смещённое начало `/MediaBox`: левое и верхнее поля — АБСОЛЮТНЫЕ
        // координаты угла `/TrimBox`, а не отступы от рамки.
        let shifted: &[(&str, [i64; 4])] = &[
            ("/TrimBox [ 95.04 90.87 576.97 847.75 ]", [34, 10, 32, 5]),
            (
                "/CropBox [ 60 80 555.32 801.92 ] /TrimBox [ 95.04 90.87 505.32 751.92 ]",
                [34, 18, 32, 18],
            ),
        ];
        for (extra, expected) in shifted {
            let pdf = one_page(&format!(
                "<< /Type /Page /Parent 2 0 R {shift} {extra} /Contents 4 0 R >>"
            ));
            let file = PdfFile::parse(&pdf).unwrap();
            let page = file.page(0).unwrap();
            let got = [
                page.margin(PdfMargin::Left),
                page.margin(PdfMargin::Right),
                page.margin(PdfMargin::Top),
                page.margin(PdfMargin::Bottom),
            ];
            assert_eq!(got, *expected, "смещённая страница с {extra}");
        }

        // `/TrimBox` НЕ наследуется от узла `/Pages` — как и `/Rotate`.
        let pdf = build_classic(
            &[
                (1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()),
                (
                    2,
                    b"<< /Type /Pages /Kids [ 3 0 R ] /Count 1 /MediaBox [ 0 0 595.32 841.92 ] \
                       /TrimBox [ 85.04 70.87 566.97 827.75 ] >>"
                        .to_vec(),
                ),
                (
                    3,
                    b"<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>".to_vec(),
                ),
                (4, empty_content()),
            ],
            "",
        );
        let file = PdfFile::parse(&pdf).unwrap();
        assert_eq!(file.page(0).unwrap().margin(PdfMargin::Left), 0);
    }

    /// Все пять фильтров строк PNG, снимаемые предиктором, — и отказ с
    /// НОМЕРОМ на предикторе TIFF и на неизвестном типе строки.
    #[test]
    fn pdf_reader_undoes_every_png_row_filter() {
        // Три строки по четыре байта, шаг предсказания — один байт.
        let rows: [[u8; 4]; 3] = [[10, 20, 30, 40], [11, 22, 33, 44], [200, 100, 50, 25]];
        for kind in 0u8..=4 {
            let mut encoded = Vec::new();
            let mut previous = [0u8; 4];
            for row in &rows {
                encoded.push(kind);
                for i in 0..4 {
                    let left = if i >= 1 { row[i - 1] } else { 0 };
                    let up = previous[i];
                    let up_left = if i >= 1 { previous[i - 1] } else { 0 };
                    let predictor = match kind {
                        0 => 0,
                        1 => left,
                        2 => up,
                        3 => ((u16::from(left) + u16::from(up)) / 2) as u8,
                        _ => paeth(left, up, up_left),
                    };
                    encoded.push(row[i].wrapping_sub(predictor));
                }
                previous = *row;
            }
            let empty = one_page("<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>");
            let mut reader = Reader::new(&empty).unwrap();
            let parms = vec![
                ("Predictor".to_string(), PdfValue::Integer(12)),
                ("Columns".to_string(), PdfValue::Integer(4)),
            ];
            let decoded = reader.apply_predictor(&parms, encoded).unwrap();
            let expected: Vec<u8> = rows.iter().flatten().copied().collect();
            assert_eq!(decoded, expected, "фильтр строки {kind}");
        }

        let empty = one_page("<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>");
        let mut reader = Reader::new(&empty).unwrap();
        let tiff = vec![("Predictor".to_string(), PdfValue::Integer(2))];
        let error = reader
            .apply_predictor(&tiff, vec![0; 4])
            .expect_err("предиктор TIFF не поддержан");
        let RtError::Pdf(text) = error else {
            panic!("ожидалась ошибка PDF");
        };
        assert!(
            text.contains('2'),
            "номер предиктора обязан быть в тексте: {text}"
        );

        let png = vec![
            ("Predictor".to_string(), PdfValue::Integer(12)),
            ("Columns".to_string(), PdfValue::Integer(4)),
        ];
        let error = reader
            .apply_predictor(&png, vec![9, 0, 0, 0, 0])
            .expect_err("тип строки 9 у PNG не определён");
        let RtError::Pdf(text) = error else {
            panic!("ожидалась ошибка PDF");
        };
        assert!(
            text.contains('9'),
            "номер типа строки обязан быть в тексте: {text}"
        );
    }

    // -----------------------------------------------------------------
    // Вложения
    // -----------------------------------------------------------------

    /// Однолистовой файл с деревом имён вложений: `spec` — тело
    /// `/EmbeddedFiles`, `objects` — всё остальное, что ему нужно.
    fn with_attachments(tree: &str, objects: &[(u32, Vec<u8>)]) -> Vec<u8> {
        let mut all = vec![
            (
                1,
                format!("<< /Type /Catalog /Pages 2 0 R /Names << /EmbeddedFiles {tree} >> >>")
                    .into_bytes(),
            ),
            (
                2,
                b"<< /Type /Pages /Kids [ 3 0 R ] /Count 1 /MediaBox [ 0 0 595.32 841.92 ] >>"
                    .to_vec(),
            ),
            (
                3,
                b"<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>".to_vec(),
            ),
            (4, empty_content()),
        ];
        all.extend_from_slice(objects);
        all.sort_by_key(|(number, _)| *number);
        build_classic(&all, "")
    }

    /// Пара объектов «поток встроенного файла + файловая спецификация»
    /// под номерами `number` и `number + 1`.
    fn filespec_pair(number: u32, head: &str, data: &[u8]) -> Vec<(u32, Vec<u8>)> {
        let mut stream = format!(
            "<< /Type /EmbeddedFile /Subtype /text#2Fplain /Length {} >>\nstream\n",
            data.len()
        )
        .into_bytes();
        stream.extend_from_slice(data);
        stream.extend_from_slice(b"\nendstream");
        vec![
            (number, stream),
            (
                number + 1,
                format!("<< /Type /Filespec {head} /EF << /F {number} 0 R >> >>").into_bytes(),
            ),
        ]
    }

    /// Файл с xref-ПОТОКОМ вместо классической таблицы: объекты пишутся
    /// подряд, а таблица — поток `/XRef` шириной `[1 4 2]` без предиктора.
    fn build_xref_stream(objects: &[(u32, Vec<u8>)]) -> Vec<u8> {
        let mut out = b"%PDF-1.5\n%\xe2\xe3\xcf\xd3\n".to_vec();
        let size = objects.iter().map(|(n, _)| *n).max().unwrap_or(0) + 2;
        let mut offsets = vec![0usize; size as usize];
        for (number, body) in objects {
            offsets[*number as usize] = out.len();
            out.extend_from_slice(format!("{number} 0 obj\n").as_bytes());
            out.extend_from_slice(body);
            out.extend_from_slice(b"\nendobj\n");
        }
        let at_xref = out.len();
        let table_number = size - 1;
        offsets[table_number as usize] = at_xref;
        let mut rows = Vec::new();
        for (index, offset) in offsets.iter().enumerate() {
            if index == 0 {
                rows.push(0u8);
                rows.extend_from_slice(&0u32.to_be_bytes());
                rows.extend_from_slice(&65535u16.to_be_bytes());
                continue;
            }
            rows.push(1u8);
            rows.extend_from_slice(&(*offset as u32).to_be_bytes());
            rows.extend_from_slice(&0u16.to_be_bytes());
        }
        let packed = zlib_compress(&rows);
        out.extend_from_slice(
            format!(
                "{table_number} 0 obj\n<< /Type /XRef /Size {size} /W [ 1 4 2 ] /Root 1 0 R \
                 /Filter /FlateDecode /Length {} >>\nstream\n",
                packed.len()
            )
            .as_bytes(),
        );
        out.extend_from_slice(&packed);
        out.extend_from_slice(b"\nendstream\nendobj\n");
        out.extend_from_slice(format!("startxref\n{at_xref}\n%%EOF\n").as_bytes());
        out
    }

    fn names_tree(entries: &[(&str, u32)]) -> String {
        let mut out = String::from("<< /Names [");
        for (name, number) in entries {
            out.push_str(&format!(" ({name}) {number} 0 R"));
        }
        out.push_str(" ] >>");
        out
    }

    /// Круг «добавить — записать — прочитать своим же читателем» на ОБОИХ
    /// видах таблицы перекрёстных ссылок: инкрементальное обновление
    /// дописывается и поверх классической `xref`, и поверх xref-потока, а
    /// страницы исходного файла при этом остаются на месте.
    #[test]
    fn pdf_attachments_round_trip_over_both_xref_kinds() {
        let mut objects = filespec_pair(10, "/F (первое.txt)", "было".as_bytes());
        let tree = names_tree(&[("первое.txt", 11)]);
        let classic = with_attachments(&tree, &objects);

        objects.sort_by_key(|(number, _)| *number);
        let mut stream_objects = vec![
            (
                1,
                format!("<< /Type /Catalog /Pages 2 0 R /Names << /EmbeddedFiles {tree} >> >>")
                    .into_bytes(),
            ),
            (
                2,
                b"<< /Type /Pages /Kids [ 3 0 R ] /Count 1 /MediaBox [ 0 0 595.32 841.92 ] >>"
                    .to_vec(),
            ),
            (
                3,
                b"<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>".to_vec(),
            ),
            (4, empty_content()),
        ];
        stream_objects.extend(objects);
        stream_objects.sort_by_key(|(number, _)| *number);
        let with_stream = build_xref_stream(&stream_objects);

        for (kind, base) in [("классическая xref", classic), ("xref-поток", with_stream)]
        {
            let file = PdfFile::parse(&base).unwrap_or_else(|e| panic!("{kind}: {e:?}"));
            assert_eq!(file.page_count(), 1, "{kind}: страница");
            assert_eq!(file.attachments().len(), 1, "{kind}: вложение из основы");

            let mut attachments = file.attachments().to_vec();
            attachments.push(PdfAttachment::new(
                "второе.bin".to_string(),
                "application/octet-stream".to_string(),
                PdfRelation::Data,
                vec![0, 1, 2, 250, 251, 252],
            ));
            let updated = file
                .write_with_attachments(&attachments)
                .unwrap_or_else(|e| panic!("{kind}: запись {e:?}"));
            // Исходные байты остаются началом файла: обновление ТОЛЬКО
            // дописывает.
            assert!(updated.starts_with(&base), "{kind}: основа переписана");

            let back =
                PdfFile::parse(&updated).unwrap_or_else(|e| panic!("{kind}: перечтение {e:?}"));
            assert_eq!(back.page_count(), 1, "{kind}: страница после записи");
            let names: Vec<&str> = back.attachments().iter().map(|a| a.name()).collect();
            // Порядок — по имени: так пишется дерево имён.
            assert_eq!(names, ["второе.bin", "первое.txt"], "{kind}: имена");
            assert_eq!(
                back.attachments()[0].data(),
                &[0, 1, 2, 250, 251, 252],
                "{kind}"
            );
            assert_eq!(
                back.attachments()[0].relation(),
                PdfRelation::Data,
                "{kind}"
            );
            assert_eq!(
                back.attachments()[0].content_type(),
                "application/octet-stream",
                "{kind}"
            );
            assert_eq!(back.attachments()[1].data(), "было".as_bytes(), "{kind}");

            // И ещё круг: обновление поверх обновления.
            let mut again = back.attachments().to_vec();
            again.retain(|item| item.name() != "первое.txt");
            let twice = back
                .write_with_attachments(&again)
                .expect("второе обновление");
            let last = PdfFile::parse(&twice).expect("перечтение второго обновления");
            assert_eq!(last.attachments().len(), 1, "{kind}: после удаления");
            assert_eq!(last.attachments()[0].name(), "второе.bin", "{kind}");
            assert_eq!(last.page_count(), 1, "{kind}: страница цела");
        }
    }

    /// Дерево имён с промежуточными узлами `/Kids` обходится целиком, а
    /// `/Limits` при этом не читается вовсе — нам нужны все записи.
    #[test]
    fn pdf_attachments_walk_a_name_tree_with_kids() {
        let mut objects = Vec::new();
        objects.extend(filespec_pair(10, "/F (a.txt)", b"A"));
        objects.extend(filespec_pair(12, "/F (b.txt)", b"B"));
        objects.extend(filespec_pair(14, "/F (c.txt)", b"C"));
        objects.push((
            30,
            b"<< /Limits [ (a.txt) (b.txt) ] /Names [ (a.txt) 11 0 R (b.txt) 13 0 R ] >>".to_vec(),
        ));
        objects.push((
            31,
            b"<< /Limits [ (c.txt) (c.txt) ] /Names [ (c.txt) 15 0 R ] >>".to_vec(),
        ));
        objects.push((32, b"<< /Kids [ 30 0 R 31 0 R ] >>".to_vec()));
        let bytes = with_attachments("32 0 R", &objects);

        let file = PdfFile::parse(&bytes).expect("дерево с /Kids обязано читаться");
        let names: Vec<&str> = file.attachments().iter().map(|a| a.name()).collect();
        assert_eq!(names, ["a.txt", "b.txt", "c.txt"]);
        assert_eq!(file.attachments()[2].data(), b"C");
    }

    /// Битое дерево имён — `RtError` с внятным текстом, а не паника и не
    /// молчаливый пропуск. Вход враждебный: цикл, лишняя глубина,
    /// нечётный `/Names`, не тот тип узла.
    #[test]
    fn pdf_attachments_reject_a_broken_name_tree() {
        let cycle = with_attachments("32 0 R", &[(32, b"<< /Kids [ 32 0 R ] >>".to_vec())]);
        let not_a_dict = with_attachments("32 0 R", &[(32, b"[ 1 2 3 ]".to_vec())]);
        let odd = with_attachments(
            "32 0 R",
            &[(32, "<< /Names [ (одно) ] >>".as_bytes().to_vec())],
        );
        let kids_not_array = with_attachments("32 0 R", &[(32, b"<< /Kids 7 >>".to_vec())]);
        let names_not_array = with_attachments("32 0 R", &[(32, b"<< /Names 7 >>".to_vec())]);

        // Глубина: цепочка `/Kids` длиннее предела.
        let mut deep: Vec<(u32, Vec<u8>)> = Vec::new();
        let depth = MAX_DEPTH as u32 + 5;
        for i in 0..depth {
            deep.push((
                100 + i,
                format!("<< /Kids [ {} 0 R ] >>", 100 + i + 1).into_bytes(),
            ));
        }
        deep.push((100 + depth, b"<< /Names [ ] >>".to_vec()));
        let too_deep = with_attachments("100 0 R", &deep);

        for (what, bytes) in [
            ("цикл", cycle),
            ("узел не словарь", not_a_dict),
            ("нечётный /Names", odd),
            ("/Kids не массив", kids_not_array),
            ("/Names не массив", names_not_array),
            ("слишком глубоко", too_deep),
        ] {
            let err = PdfFile::parse(&bytes)
                .expect_err(&format!("{what}: разбор обязан кончиться ошибкой"));
            let RtError::Pdf(text) = err else {
                panic!("{what}: ожидался RtError::Pdf, получено {err:?}");
            };
            assert!(!text.is_empty(), "{what}: пустой текст ошибки");
        }
    }

    /// Записи без данных платформа молча пропускает — и мы тоже: без
    /// `/EF`, с висящей ссылкой в нём и без имени вовсе.
    #[test]
    fn pdf_attachments_skip_entries_without_data_or_name() {
        let mut objects = Vec::new();
        objects.push((11, b"<< /Type /Filespec /F (noef.txt) >>".to_vec()));
        objects.push((
            13,
            b"<< /Type /Filespec /F (dangling.txt) /EF << /F 99 0 R >> >>".to_vec(),
        ));
        objects.extend(filespec_pair(
            14,
            "/Type /Filespec",
            "безымянное".as_bytes(),
        ));
        objects.extend(filespec_pair(16, "/F (живое.txt)", "живое".as_bytes()));
        let tree = names_tree(&[("noef", 11), ("dangling", 13), ("noname", 15), ("ok", 17)]);
        let bytes = with_attachments(&tree, &objects);

        let file = PdfFile::parse(&bytes).expect("файл с битыми записями обязан читаться");
        let names: Vec<&str> = file.attachments().iter().map(|a| a.name()).collect();
        assert_eq!(names, ["живое.txt"]);
    }

    /// Имя берётся из `/UF`, а без него из `/F`; байты со знаком порядка
    /// FE FF читаются как UTF-16BE, остальные — как UTF-8. Одноимённые
    /// записи схлопываются, и побеждает ПОСЛЕДНЯЯ.
    #[test]
    fn pdf_attachments_decode_names_and_collapse_duplicates() {
        let utf16: String = "юникод.txt"
            .encode_utf16()
            .map(|unit| {
                let [hi, lo] = unit.to_be_bytes();
                format!("\\{hi:03o}\\{lo:03o}")
            })
            .collect();
        let mut objects = Vec::new();
        objects.extend(filespec_pair(10, "/F (только-f.txt)", b"1"));
        objects.extend(filespec_pair(12, "/F (не-это.txt) /UF (это.txt)", b"2"));
        objects.extend(filespec_pair(14, &format!("/UF (\\376\\377{utf16})"), b"3"));
        objects.extend(filespec_pair(16, "/F (дубль.txt)", "первое".as_bytes()));
        objects.extend(filespec_pair(18, "/F (дубль.txt)", "второе".as_bytes()));
        let tree = names_tree(&[("a", 11), ("b", 13), ("c", 15), ("d", 17), ("e", 19)]);
        let bytes = with_attachments(&tree, &objects);

        let file = PdfFile::parse(&bytes).expect("файл обязан читаться");
        let names: Vec<&str> = file.attachments().iter().map(|a| a.name()).collect();
        assert_eq!(
            names,
            ["только-f.txt", "это.txt", "юникод.txt", "дубль.txt"]
        );
        assert_eq!(file.attachments()[3].data(), "второе".as_bytes());
    }

    /// Поток встроенного файла с фильтром, которого мы не умеем, не
    /// уносит документ: вложение остаётся, содержимое — сырые байты.
    #[test]
    fn pdf_attachments_survive_an_unsupported_filter() {
        let objects = vec![
            (
                10,
                b"<< /Type /EmbeddedFile /Filter /LZWDecode /Length 3 >>\nstream\n\x80\x0b\x60\nendstream"
                    .to_vec(),
            ),
            (
                11,
                b"<< /Type /Filespec /F (lzw.txt) /EF << /F 10 0 R >> >>".to_vec(),
            ),
        ];
        let bytes = with_attachments(&names_tree(&[("lzw", 11)]), &objects);

        let file = PdfFile::parse(&bytes).expect("неизвестный фильтр не должен ронять документ");
        assert_eq!(file.page_count(), 1);
        assert_eq!(file.attachments().len(), 1);
        assert_eq!(file.attachments()[0].data(), b"\x80\x0b\x60");
    }

    /// Связь читается из `/AFRelationship`, а неизвестное имя — это
    /// `НеУстановлено` (измерено на `/AFRelationship /Nonsense`).
    #[test]
    fn pdf_attachments_read_the_relationship() {
        let mut objects = Vec::new();
        objects.extend(filespec_pair(
            10,
            "/F (s.txt) /AFRelationship /Source",
            b"1",
        ));
        objects.extend(filespec_pair(12, "/F (d.txt) /AFRelationship /Data", b"2"));
        objects.extend(filespec_pair(
            14,
            "/F (a.txt) /AFRelationship /Alternative",
            b"3",
        ));
        objects.extend(filespec_pair(
            16,
            "/F (u.txt) /AFRelationship /Supplement",
            b"4",
        ));
        objects.extend(filespec_pair(
            18,
            "/F (n.txt) /AFRelationship /Nonsense",
            b"5",
        ));
        objects.extend(filespec_pair(20, "/F (empty.txt)", b"6"));
        let tree = names_tree(&[
            ("a", 11),
            ("b", 13),
            ("c", 15),
            ("d", 17),
            ("e", 19),
            ("f", 21),
        ]);
        let bytes = with_attachments(&tree, &objects);

        let file = PdfFile::parse(&bytes).expect("файл обязан читаться");
        let relations: Vec<PdfRelation> = file.attachments().iter().map(|a| a.relation()).collect();
        assert_eq!(
            relations,
            [
                PdfRelation::Source,
                PdfRelation::Data,
                PdfRelation::Alternative,
                PdfRelation::Supplement,
                PdfRelation::Unspecified,
                PdfRelation::Unspecified,
            ]
        );
    }

    /// Имена в записанном дереве экранируются РОВНО тремя знаками и
    /// уходят сырыми байтами: `/F` в UTF-8, `/UF` в UTF-16BE. Иначе
    /// платформа, которая не снимает восьмеричные экраны, прочитала бы
    /// вместо имени его запись.
    #[test]
    fn pdf_attachments_are_written_with_raw_name_bytes() {
        let base = with_attachments("32 0 R", &[(32, b"<< /Names [ ] >>".to_vec())]);
        let file = PdfFile::parse(&base).expect("основа обязана читаться");
        let written = file
            .write_with_attachments(&[PdfAttachment::new(
                "имя (со скобкой).txt".to_string(),
                "text/plain".to_string(),
                PdfRelation::Unspecified,
                b"data".to_vec(),
            )])
            .expect("запись");

        assert!(
            find(&written, "имя \\(со скобкой\\).txt".as_bytes()).is_some(),
            "имя обязано быть записано сырыми байтами UTF-8 со скобочным экранированием"
        );
        assert!(
            find(&written, &[0xFE, 0xFF, 0x04, 0x38, 0x04, 0x3C, 0x04, 0x4F]).is_some(),
            "/UF обязан быть UTF-16BE с меткой порядка байтов"
        );
        assert!(
            find(&written, b"\\376\\377").is_none(),
            "восьмеричных экранов в имени быть не должно"
        );
        let back = PdfFile::parse(&written).expect("перечтение");
        assert_eq!(back.attachments()[0].name(), "имя (со скобкой).txt");
    }

    /// Шестнадцатеричная строка в фикстуре `pdf-attachments.bsl` — это
    /// РОВНО байты снятого с платформы `attach-platform.pdf`.
    ///
    /// Копия нужна оснастке (конформанс-раннер запускает фикстуру из
    /// каталога крейта, платформенный — из корня репозитория, и одному
    /// относительному пути с обоими не сойтись), но копия, которая молча
    /// разъехалась с оригиналом, — это фикстура, проверяющая не то.
    #[test]
    fn pdf_attachments_fixture_hex_matches_the_captured_file() {
        let fixture =
            std::fs::read_to_string("../../tests/conformance/fixtures/pdf-attachments.bsl")
                .expect("фикстура вложений обязана лежать в дереве");
        let mut hex = String::new();
        for line in fixture.lines() {
            if !line.starts_with("ШестнПлатформа") {
                continue;
            }
            let mut parts = line.split('"');
            parts.next();
            for (index, part) in parts.enumerate() {
                if index % 2 == 0 {
                    hex.push_str(part);
                }
            }
        }
        assert!(!hex.is_empty(), "в фикстуре не нашлось строки с байтами");
        let bytes: Vec<u8> = hex
            .as_bytes()
            .chunks(2)
            .map(|pair| {
                u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16)
                    .expect("в строке фикстуры обязаны быть только шестнадцатеричные цифры")
            })
            .collect();
        let captured = std::fs::read("../../tests/conformance/pdf/attach-platform.pdf")
            .expect("снимок платформы обязан лежать в дереве");
        assert_eq!(
            bytes, captured,
            "байты в фикстуре разошлись со снимком attach-platform.pdf"
        );
    }
}
