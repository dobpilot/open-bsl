//! Чтение контейнера PDF: xref, объекты, потоки, вложения.

use super::*;

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
pub(crate) const MAX_DEPTH: usize = 64;

/// Предел на длину цепочки таблиц перекрёстных ссылок (`/Prev` и
/// `/XRefStm`). Инкрементальных обновлений в живом файле единицы.
pub(crate) const MAX_XREF_SECTIONS: usize = 1024;

/// Предел на размер РАСПАКОВАННОГО потока. Без него подделанный поток
/// раздувает память: `FlateDecode` умеет отдавать гигабайты из килобайт.
pub(crate) const MAX_STREAM_OUT: usize = 256 << 20;

/// Предел на число страниц в дереве. Дерево обходится с множеством
/// посещённых номеров, поэтому предел нужен только для страниц, вписанных
/// в `/Kids` НЕ ссылкой, а словарём на месте.
pub(crate) const MAX_PAGES: usize = 1 << 20;

/// Страница разобранного файла: только то, что отдаёт платформа.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PdfPageInfo {
    pub(crate) width_mm: i64,
    pub(crate) height_mm: i64,
    pub(crate) rotate: i64,
    /// Поля в миллиметрах в порядке «слева, справа, сверху, снизу» —
    /// ровно том, в каком их читает [`PdfPageInfo::margin`].
    pub(crate) margins: [i64; 4],
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
pub(crate) const MAX_ATTACHMENTS: usize = 1 << 16;

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
    pub(crate) fn from_pdf_name(name: &str) -> PdfRelation {
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
    pub(crate) name: String,
    pub(crate) content_type: String,
    pub(crate) relation: PdfRelation,
    pub(crate) data: Vec<u8>,
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
    pub(crate) pages: Vec<PdfPageInfo>,
    pub(crate) attachments: Vec<PdfAttachment>,
    pub(crate) source: Vec<u8>,
    /// Номер объекта каталога. `None`, если `/Root` в трейлере — словарь на
    /// месте, а не ссылка: такой каталог инкрементальным обновлением не
    /// переписать.
    pub(crate) catalog_number: Option<u32>,
    pub(crate) catalog: Vec<(String, PdfValue)>,
    /// Разрешённый словарь каталога `/Names` без `/EmbeddedFiles`: при
    /// записи он переносится целиком, чтобы не потерять чужие деревья имён
    /// (`/Dests`, `/JavaScript`).
    pub(crate) names_dict: Vec<(String, PdfValue)>,
    /// Смещение таблицы перекрёстных ссылок, с которой начался разбор, —
    /// оно же `/Prev` нового обновления.
    pub(crate) startxref: usize,
    /// Номер, с которого можно заводить новые объекты.
    pub(crate) next_object: u32,
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

pub(crate) fn pdf_err(text: impl Into<String>) -> RtError {
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
pub(crate) fn decode_text_string(bytes: &[u8]) -> String {
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
pub(crate) fn write_raw_str(out: &mut Vec<u8>, bytes: &[u8]) {
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
pub(crate) fn take_object_number(next: &mut u32) -> RtResult<u32> {
    let taken = *next;
    *next = next
        .checked_add(1)
        .ok_or_else(|| pdf_err("в файле кончились номера объектов"))?;
    Ok(taken)
}

/// Имя вложения в UTF-16BE с меткой порядка байтов — то, что идёт в `/UF`.
pub(crate) fn utf16be_with_bom(text: &str) -> Vec<u8> {
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
pub(crate) fn is_ws(b: u8) -> bool {
    matches!(b, 0x00 | 0x09 | 0x0A | 0x0C | 0x0D | 0x20)
}

/// Разделители PDF (таблица 2): на них кончается любая лексема.
pub(crate) fn is_delim(b: u8) -> bool {
    matches!(
        b,
        b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
    )
}

pub(crate) fn is_regular(b: u8) -> bool {
    !is_ws(b) && !is_delim(b)
}

/// Значение по ключу словаря. Ключи PDF регистрозависимы.
pub(crate) fn dict_get<'a>(dict: &'a [(String, PdfValue)], key: &str) -> Option<&'a PdfValue> {
    dict.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

/// Лексер и разборщик значений. Потоки он не разбирает — за ними нужен
/// `/Length`, а тот бывает косвенной ссылкой, то есть уже [`Reader`].
pub(crate) struct Lexer<'a> {
    pub(crate) data: &'a [u8],
    pub(crate) pos: usize,
}

impl<'a> Lexer<'a> {
    pub(crate) fn new(data: &'a [u8], pos: usize) -> Self {
        Lexer { data, pos }
    }

    pub(crate) fn peek(&self) -> Option<u8> {
        self.data.get(self.pos).copied()
    }

    /// Пробелы и комментарии `%` до конца строки.
    pub(crate) fn skip_ws(&mut self) {
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
    pub(crate) fn token(&mut self) -> &'a [u8] {
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

    pub(crate) fn expect_keyword(&mut self, word: &str) -> RtResult<()> {
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
    pub(crate) fn unsigned(&mut self) -> Option<u64> {
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

    pub(crate) fn parse_value(&mut self, depth: usize) -> RtResult<PdfValue> {
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

    pub(crate) fn parse_dict(&mut self, depth: usize) -> RtResult<Vec<(String, PdfValue)>> {
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
                    )));
                }
            }
        }
    }

    /// Имя `/Name`. Escape-последовательности `#xx` раскрываются, поэтому
    /// `/A#20B` и `/A B` — разные имена, а `/Ко#Д` — ошибка.
    pub(crate) fn parse_name(&mut self) -> RtResult<PdfValue> {
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
                        ));
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
    pub(crate) fn parse_literal_string(&mut self) -> RtResult<PdfValue> {
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
    pub(crate) fn parse_hex_string(&mut self) -> RtResult<PdfValue> {
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
    pub(crate) fn parse_number_or_ref(&mut self) -> RtResult<PdfValue> {
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

pub(crate) fn hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Где лежит объект: прямо в файле или внутри объектного потока.
#[derive(Debug, Clone, Copy)]
pub(crate) enum XrefLoc {
    /// Смещение от начала файла.
    Offset(usize),
    /// Номер объектного потока и порядковый номер объекта в нём.
    InStream { stream: u32, index: usize },
}

/// Разобранный объектный поток `/Type /ObjStm`: распакованные байты и
/// таблица «номер объекта — смещение внутри».
pub(crate) struct ObjStm {
    pub(crate) data: Vec<u8>,
    pub(crate) entries: Vec<(u32, usize)>,
}

pub(crate) struct Reader<'a> {
    pub(crate) data: &'a [u8],
    pub(crate) xref: std::collections::HashMap<u32, XrefLoc>,
    pub(crate) trailer: Vec<(String, PdfValue)>,
    pub(crate) cache: std::collections::HashMap<u32, PdfValue>,
    /// Стек номеров разбираемых сейчас объектов — защита от `1 0 obj 1 0 R`.
    pub(crate) active: Vec<u32>,
    pub(crate) streams: std::collections::HashMap<u32, std::rc::Rc<ObjStm>>,
    /// Смещение таблицы, с которой начался разбор: `/Prev` будущего
    /// инкрементального обновления.
    pub(crate) startxref: usize,
}

impl<'a> Reader<'a> {
    pub(crate) fn new(data: &'a [u8]) -> RtResult<Reader<'a>> {
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
    pub(crate) fn find_startxref(&self) -> RtResult<usize> {
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
    pub(crate) fn load_xref_chain(&mut self, start: usize) -> RtResult<()> {
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
                if let Some(PdfValue::Integer(prev)) = dict_get(&section, key)
                    && let Ok(prev) = usize::try_from(*prev)
                    && prev < self.data.len()
                {
                    pending.push_back(prev);
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
    pub(crate) fn load_xref_section(&mut self, offset: usize) -> RtResult<Vec<(String, PdfValue)>> {
        let mut lexer = Lexer::new(self.data, offset);
        lexer.skip_ws();
        if self.data[lexer.pos..].starts_with(b"xref") {
            lexer.pos += 4;
            return self.load_classic_xref(lexer);
        }
        self.load_xref_stream(offset)
    }

    pub(crate) fn load_classic_xref(
        &mut self,
        mut lexer: Lexer<'a>,
    ) -> RtResult<Vec<(String, PdfValue)>> {
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
                        )));
                    }
                }
            }
        }
    }

    pub(crate) fn load_xref_stream(&mut self, offset: usize) -> RtResult<Vec<(String, PdfValue)>> {
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
    pub(crate) fn object(&mut self, number: u32) -> RtResult<PdfValue> {
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
    pub(crate) fn resolve(&mut self, value: &PdfValue) -> RtResult<PdfValue> {
        match value {
            PdfValue::Ref(number) => self.object(*number),
            other => Ok(other.clone()),
        }
    }

    /// Разобрать `N G obj ... endobj` по смещению.
    pub(crate) fn parse_object_at(&mut self, offset: usize) -> RtResult<PdfValue> {
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
    pub(crate) fn ends_with_endstream(&self, start: usize, length: usize) -> bool {
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
    pub(crate) fn object_from_stream(&mut self, stream: u32, index: usize) -> RtResult<PdfValue> {
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

    pub(crate) fn object_stream(&mut self, number: u32) -> RtResult<std::rc::Rc<ObjStm>> {
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
    pub(crate) fn decode_stream(
        &mut self,
        dict: &[(String, PdfValue)],
        raw: &[u8],
    ) -> RtResult<Vec<u8>> {
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
                    )));
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
    pub(crate) fn apply_predictor(
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
                        )));
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
    pub(crate) fn read_file(mut self) -> RtResult<PdfFile> {
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
    pub(crate) fn read_attachments(
        &mut self,
        root: &[(String, PdfValue)],
    ) -> RtResult<Vec<PdfAttachment>> {
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
    pub(crate) fn walk_name_tree(
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
    pub(crate) fn filespec(&mut self, value: &PdfValue) -> RtResult<Option<PdfAttachment>> {
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
    pub(crate) fn string_value(
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

    pub(crate) fn walk_pages(
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
    pub(crate) fn rect(
        &mut self,
        dict: &[(String, PdfValue)],
        key: &str,
    ) -> RtResult<Option<[f64; 4]>> {
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
pub(crate) struct Inherited {
    pub(crate) media: Option<[f64; 4]>,
    pub(crate) crop: Option<[f64; 4]>,
}

/// Видимая рамка страницы в пунктах: `/CropBox`, пересечённый с
/// `/MediaBox`, либо один `/MediaBox`. Рамки нет вовсе — US Letter
/// (измерено: страница без `/MediaBox` отдала 216 на 279 мм).
pub(crate) fn visible_rect(inherited: Inherited) -> [f64; 4] {
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
pub(crate) fn extent(rect: [f64; 4]) -> (f64, f64) {
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
pub(crate) fn margins_of(visible: [f64; 4], trim: Option<[f64; 4]>) -> [i64; 4] {
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
pub(crate) fn pt_to_mm(pt: f64) -> i64 {
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
pub(crate) fn rotate_of(page: &[(String, PdfValue)]) -> i64 {
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

pub(crate) fn paeth(left: u8, up: u8, up_left: u8) -> u8 {
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
pub(crate) fn inflate_pdf_stream(data: &[u8]) -> RtResult<Vec<u8>> {
    if data.len() >= 2 {
        let (cmf, flg) = (data[0], data[1]);
        let looks_zlib = cmf & 0x0F == 8 && (u16::from(cmf) * 256 + u16::from(flg)) % 31 == 0;
        if looks_zlib
            && let Ok(out) =
                inflate_with_limit(flate2::read::ZlibDecoder::new(data), MAX_STREAM_OUT)
        {
            return Ok(out);
        }
    }
    inflate_with_limit(flate2::read::DeflateDecoder::new(data), MAX_STREAM_OUT)
}
