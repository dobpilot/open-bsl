//! Разбор MXL: скобочная сериализация и сборка модели.

use super::*;

// --- разбор MXL -----------------------------------------------------------

/// Узел скобочной сериализации: либо группа, либо строка в кавычках, либо
/// «атом» — число, GUID или голое слово.
#[derive(Debug)]
pub(crate) enum Node {
    Group(Vec<Node>),
    Text(String),
    Atom(String),
}

impl Node {
    pub(crate) fn group(&self) -> RtResult<&[Node]> {
        match self {
            Node::Group(g) => Ok(g),
            _ => Err(bad("ожидалась группа")),
        }
    }

    pub(crate) fn text(&self) -> String {
        match self {
            Node::Text(s) => s.clone(),
            Node::Atom(s) => s.clone(),
            Node::Group(_) => String::new(),
        }
    }

    pub(crate) fn number(&self) -> RtResult<i64> {
        match self {
            Node::Atom(s) => s
                .parse()
                .map_err(|_| bad(format!("ожидалось число, а не «{s}»"))),
            _ => Err(bad("ожидалось число")),
        }
    }
}

/// Разбор текста в дерево. Кавычка внутри строки удвоена — единственное
/// экранирование формата, поэтому и разбор у него ровно один.
pub(crate) fn parse_nodes(s: &str) -> RtResult<Node> {
    let mut chars = s.chars().peekable();
    fn group(chars: &mut std::iter::Peekable<std::str::Chars>) -> RtResult<Node> {
        let mut items = Vec::new();
        let mut atom = String::new();
        let flush = |atom: &mut String, items: &mut Vec<Node>| {
            let t = atom.trim();
            if !t.is_empty() {
                items.push(Node::Atom(t.to_string()));
            }
            atom.clear();
        };
        while let Some(c) = chars.next() {
            match c {
                '{' => {
                    flush(&mut atom, &mut items);
                    items.push(group(chars)?);
                }
                '}' => {
                    flush(&mut atom, &mut items);
                    return Ok(Node::Group(items));
                }
                ',' => flush(&mut atom, &mut items),
                '"' => {
                    flush(&mut atom, &mut items);
                    let mut text = String::new();
                    loop {
                        match chars.next() {
                            Some('"') => {
                                if chars.peek() == Some(&'"') {
                                    text.push('"');
                                    chars.next();
                                } else {
                                    break;
                                }
                            }
                            Some(ch) => text.push(ch),
                            None => return Err(bad("незакрытая строка")),
                        }
                    }
                    items.push(Node::Text(text));
                }
                c if c.is_whitespace() => atom.push(' '),
                c => atom.push(c),
            }
        }
        Err(bad("незакрытая группа"))
    }
    match chars.next() {
        Some('{') => group(&mut chars),
        _ => Err(bad("файл не начинается с группы")),
    }
}

/// Курсор по плоскому списку узлов: разделы формата идут подряд и без
/// разметки, поэтому читаются позиционно — ровно в том же порядке, в каком
/// их пишет [`mxl_body`]. Эти две функции обязаны меняться вместе.
pub(crate) struct Cursor<'a> {
    pub(crate) items: &'a [Node],
    pub(crate) pos: usize,
}

impl<'a> Cursor<'a> {
    pub(crate) fn next(&mut self) -> RtResult<&'a Node> {
        let n = self
            .items
            .get(self.pos)
            .ok_or_else(|| bad("файл кончился раньше времени"))?;
        self.pos += 1;
        Ok(n)
    }

    /// Ошибка называет ПОЗИЦИЮ и то, что там лежит: разделы читаются
    /// подряд, и без этого непонятно, на каком именно разбор сбился.
    pub(crate) fn number(&mut self) -> RtResult<i64> {
        let at = self.pos;
        self.next()?.number().map_err(|_| {
            bad(format!(
                "ожидалось число в позиции {at}, а там {:?}",
                self.items.get(at)
            ))
        })
    }

    pub(crate) fn skip(&mut self, n: usize) -> RtResult<()> {
        for _ in 0..n {
            self.next()?;
        }
        Ok(())
    }
}

/// Прочитать документ из байтов .mxl.
///
/// # Errors
///
/// Возвращает ошибку, если нет заголовка `MOXCEL`, текст не разбирается или
/// разделы идут не в том порядке, в каком их пишет платформа.
pub fn from_mxl_bytes(bytes: &[u8]) -> RtResult<SpreadDocData> {
    // Сверяется ПОДПИСЬ, а не весь заголовок: последний его байт — версия
    // формата, и она бывает разной. Файл настоящего отчёта из 1С пришёл с
    // версией 11, наши эталоны — с 12; тело при этом устроено одинаково.
    // Пишем мы всегда 12, как более новую.
    if !bytes.starts_with(MXL_SIGNATURE) {
        return Err(bad("не файл MXL: нет подписи MOXCEL"));
    }
    if bytes.len() < MXL_HEADER.len() {
        return Err(bad("файл MXL оборван на заголовке"));
    }
    let body = &bytes[MXL_HEADER.len()..];
    let body = body.strip_prefix(BOM).unwrap_or(body);
    let text = std::str::from_utf8(body).map_err(|_| bad("тело MXL не в UTF-8"))?;
    // Ровно обратное тому, что делает запись: `ВК`+`ПС` свернуть в `ПС`.
    // Иначе перевод строки ВНУТРИ текста ячейки вернулся бы удвоенным.
    let text = text.replace("\r\n", "\n");
    let root = parse_nodes(&text)?;
    let items = root.group()?;
    let mut cur = Cursor { items, pos: 0 };

    let mut doc = SpreadDocData::new();
    // Версия, языки, размеры по умолчанию, палитра линий и шесть служебных
    // групп — их содержимое мы не моделируем, но пропустить надо ровно
    // столько, сколько пишем сами.
    cur.skip(3)?; // 8, 1, 12
    cur.skip(1)?; // языки
    cur.skip(1)?; // {128,72}
    // Палитра линий: `{<кол-во>,1,<запись>,0,1,<запись>,0,...}` — записи
    // идут тройками, потому что каждая обёрнута парой служебных чисел.
    let line_palette = {
        let g = cur.next()?.group()?;
        let count = g.first().map_or(Ok(0), Node::number)? as usize;
        let mut out = Vec::with_capacity(count);
        for i in 0..count {
            if let Some(Node::Group(record)) = g.get(2 + i * 3) {
                out.push(Line {
                    style: match record.get(3).map_or(Ok(1), Node::number)? {
                        2 => LineStyle::Dotted,
                        3 => LineStyle::Double,
                        _ => LineStyle::Solid,
                    },
                    width: record.get(4).map_or(Ok(1), Node::number)?,
                });
            }
        }
        out
    };
    cur.skip(1)?; // счётчик следом за палитрой
    cur.skip(6)?; // шесть {0,0}
    cur.skip(2)?; // 0, 2

    // --- строки и ячейки ---------------------------------------------
    let row_count = cur.number()?;
    let mut heights: Vec<(u32, i64)> = Vec::new();
    let mut cells: Vec<(u32, u32, CellData)> = Vec::new();
    let mut row_formats: Vec<(u32, usize)> = Vec::new();
    let mut cell_formats: Vec<(u32, u32, usize)> = Vec::new();
    for _ in 0..row_count {
        let index = cur.number()? as u32;
        let row_format = cur.number()? as usize;
        if row_format > 0 {
            row_formats.push((index, row_format));
        }
        let count = cur.number()?;
        if count == 0 {
            heights.push((index, 0)); // строка есть, ячеек нет
            continue;
        }
        let mut column = cur.number()? as u32;
        for i in 0..count {
            let cell_group = cur.next()?.group()?;
            let mut cell = CellData::default();
            let mask = cell_group
                .first()
                .ok_or_else(|| bad("пустая ячейка"))?
                .number()?;
            // Поля идут по возрастанию бита, поэтому читаются курсором.
            let mut field = 2;
            if mask & 2 != 0 {
                // `{"S","<представление>"}` — тег типа и то, как значение
                // печатается. Самого значения в файле нет (измерено).
                if let Some(Node::Group(pair)) = cell_group.get(field) {
                    cell.value = Some(pair_text(pair_get(pair, 1)));
                }
                field += 1;
            }
            if mask & 4 != 0 {
                if let Some(Node::Group(pair)) = cell_group.get(field) {
                    cell.detail = Some(pair_text(pair_get(pair, 1)));
                }
                field += 1;
            }
            if mask & 8 != 0 {
                cell.detail_param = cell_group.get(field).map(Node::text).unwrap_or_default();
                field += 1;
            }
            let cell_format = cell_group.get(1).map_or(Ok(0), Node::number)?;
            if mask & 16 != 0 {
                // Текстовый блок: `{1,1,{<язык>,<текст>}}`.
                if let Some(Node::Group(block)) = cell_group.get(field)
                    && let Some(Node::Group(pair)) = block.get(2)
                {
                    let lang = pair_text(pair_get(pair, 0));
                    let value = pair_text(pair_get(pair, 1));
                    if lang.is_empty() {
                        cell.parameter = value;
                    } else {
                        cell.text = value;
                    }
                }
            }
            if cell_format > 0 {
                cell_formats.push((index, column, cell_format as usize));
            }
            cells.push((index, column, cell));
            if i + 1 < count {
                column = cur.number()? as u32;
            }
        }
    }

    // --- колонки и высота ---------------------------------------------
    let columns = cur.next()?.group()?;
    let width = columns.first().map_or(Ok(0), Node::number)? as u32;
    // `{<число колонок>,0,GUID,<сколько с форматом>,<колонка>,<формат>,...}`
    let mut column_formats: Vec<(u32, usize)> = Vec::new();
    if columns.len() > 4 {
        let count = columns[3].number()? as usize;
        for i in 0..count {
            let (a, b) = (4 + i * 2, 5 + i * 2);
            if b < columns.len() {
                column_formats.push((columns[a].number()? as u32, columns[b].number()? as usize));
            }
        }
    }
    let height = cur.number()? as u32;
    // Дополнительные НАБОРЫ КОЛОНОК: счётчик, сами наборы и привязка строк
    // к ним парами «строка, набор». Своей модели у них нет — строки лежат в
    // общей сетке, — но пропустить их надо точно, иначе разъедется всё
    // дальше. В простом документе оба счётчика нулевые, и раздел выглядит
    // как два нуля; настоящий отчёт из 1С показал, что это не константа.
    let set_count = cur.number()? as usize;
    let mut raw_sets: Vec<RawColumnSet> = Vec::with_capacity(set_count);
    for _ in 0..set_count {
        let g = cur.next()?.group()?;
        let count = g.first().map_or(Ok(0), Node::number)?.max(0) as u32;
        let id = g.get(2).map(Node::text).unwrap_or_default();
        // Хвост дескриптора устроен как у основного набора: сколько колонок
        // с форматом, затем пары «колонка, формат».
        let mut formats = Vec::new();
        if g.len() > 4 {
            let count = g[3].number()?.max(0) as usize;
            for i in 0..count {
                let (a, b) = (4 + i * 2, 5 + i * 2);
                if b < g.len() {
                    formats.push((g[a].number()?.max(0) as u32, g[b].number()?.max(0) as usize));
                }
            }
        }
        raw_sets.push((id, count, formats));
    }
    let binding_count = cur.number()? as usize;
    let mut bindings = Vec::with_capacity(binding_count);
    for _ in 0..binding_count {
        let row = cur.number()?.max(0) as u32;
        let set = cur.number()?.max(0) as u32;
        bindings.push((row, set));
    }

    // Дальше ДВАЖДЫ число рисунков и сами рисунки, затем секция групп
    // строк, затем ещё три нуля.
    // Первое поле — сколько НОМЕРОВ выдано, второе — сколько рисунков
    // осталось. Различить их удалось только на документе, где рисунок
    // добавили и удалили: там `1,0`.
    cur.skip(1)?;
    let drawing_count = cur.number()? as usize;
    let mut drawing_cells_raw = Vec::with_capacity(drawing_count);
    for _ in 0..drawing_count {
        let g = cur.next()?.group()?;
        // `{{0,<формат>},2,<кол1>,<стр1>,<смещ>,<смещ>,<кол2>,<стр2>,...}`
        let num = |i: usize| -> RtResult<i64> { g.get(i).map_or(Ok(0), Node::number) };
        drawing_cells_raw.push((
            num(2)? as u32,
            num(3)? as u32,
            num(4)?,
            num(5)?,
            num(6)? as u32,
            num(7)? as u32,
            num(8)?,
            num(9)?,
        ));
    }
    let group_count = cur.number()? as usize;
    let mut groups = Vec::with_capacity(group_count);
    for _ in 0..group_count {
        let g = cur.next()?.group()?;
        let name = match g.get(3) {
            Some(Node::Group(loc)) => match loc.get(2) {
                Some(Node::Group(pair)) => pair_text(pair_get(pair, 1)),
                _ => String::new(),
            },
            _ => String::new(),
        };
        groups.push(RowGroup {
            r1: g.first().map_or(Ok(0), Node::number)?.max(0) as u32,
            r2: g.get(1).map_or(Ok(0), Node::number)?.max(0) as u32,
            level: g.get(2).map_or(Ok(0), Node::number)?.max(0) as u32,
            name,
            collapsed: g.get(4).map_or(Ok(0), Node::number)? != 0,
        });
        cur.skip(1)?; // замыкающий -1
    }
    cur.skip(3)?;

    for (r, c, cell) in cells {
        if let Some(detail) = &cell.detail {
            doc.set_cell_detail(r, c, detail);
        }
        if !cell.detail_param.is_empty() {
            doc.set_cell_detail_param(r, c, &cell.detail_param);
        }
        if let Some(presentation) = &cell.value {
            doc.set_cell_value(r, c, presentation);
        } else if !cell.text.is_empty() {
            doc.set_cell_text(r, c, &cell.text);
        } else if !cell.parameter.is_empty() {
            doc.set_cell_parameter(r, c, &cell.parameter);
        } else {
            doc.rows.entry(r).or_default().cells.entry(c).or_default();
        }
    }
    for (r, _) in heights {
        doc.rows.entry(r).or_default();
    }

    // --- три списка объединений ----------------------------------------
    for record in cur.next()?.group()?.iter().skip(1) {
        let g = record.group()?;
        if g.len() >= 5 {
            let (c1, r1, c2, r2, flag) = (
                g[0].number()? as u32,
                g[1].number()? as u32,
                g[2].number()? as u32,
                g[3].number()? as u32,
                g[4].number()?,
            );
            doc.merges.push(Merge {
                r1,
                c1,
                r2,
                c2,
                unmerge: flag == 2,
            });
        }
    }
    for record in cur.next()?.group()?.iter().skip(1) {
        let g = record.group()?;
        if g.len() >= 4 {
            doc.row_merges
                .push((g[1].number()? as u32, g[3].number()? as u32));
        }
    }
    for record in cur.next()?.group()?.iter().skip(1) {
        let g = record.group()?;
        if g.len() >= 3 {
            doc.col_merges
                .push((g[0].number()? as u32, g[2].number()? as u32));
        }
    }

    // --- именованные области --------------------------------------------
    let names = cur.next()?.group()?;
    let mut i = 1;
    while i + 1 < names.len() {
        let name = names[i].text();
        if let Node::Group(body) = &names[i + 1] {
            // `{1,{<область>},0}` — ячейки, `{2,<рисунок>,0}` — рисунок;
            // рисунки мы не моделируем и пропускаем.
            if body.first().map_or(Ok(0), Node::number)? == 1
                && let Some(Node::Group(area)) = body.get(1)
                && area.len() >= 5
            {
                let kind = match area[0].number()? {
                    1 => AreaKind::Rows,
                    2 => AreaKind::Columns,
                    _ => AreaKind::Rect,
                };
                let (c1, r1, c2, r2) = (
                    area[1].number()?,
                    area[2].number()?,
                    area[3].number()?,
                    area[4].number()?,
                );
                doc.names.insert(
                    name,
                    NamedArea {
                        kind,
                        r1: r1.max(0) as u32,
                        c1: c1.max(0) as u32,
                        r2: r2.max(0) as u32,
                        c2: c2.max(0) as u32,
                    },
                );
            }
        }
        i += 2;
    }

    // --- хвост: параметры страницы, область печати и палитры -----------
    cur.skip(1)?; // ""
    let page = cur.next()?.group()?;
    // `{0,<кол-во>,<id>,{"N",v},...}` — пары «идентификатор — значение»,
    // отсортированные по идентификатору.
    if let Some(Node::Group(list)) = page.first() {
        let mut i = 2;
        while i + 1 < list.len() {
            let id = list[i].number().unwrap_or(0);
            if let Node::Group(val) = &list[i + 1] {
                let value = val.get(1).map_or(Ok(0), Node::number)?;
                // Поля страницы лежат в сотых долях миллиметра, и порядок
                // идентификаторов свой: сверху, слева, снизу, справа.
                let mm = |v: i64| v as f64 / 100.0;
                match id {
                    1 => doc.landscape = value == 2,
                    2 => doc.print_scale = Some(value),
                    6 => doc.margins.top = mm(value),
                    7 => doc.margins.left = mm(value),
                    8 => doc.margins.bottom = mm(value),
                    9 => doc.margins.right = mm(value),
                    _ => {}
                }
            }
            i += 2;
        }
    }
    let print_area = cur.next()?.group()?;
    if print_area.first().map_or(Ok(0), Node::number)? != 0 && print_area.len() >= 5 {
        doc.print_area = Some((
            print_area[2].number()?.max(0) as u32,
            print_area[1].number()?.max(0) as u32,
            print_area[4].number()?.max(0) as u32,
            print_area[3].number()?.max(0) as u32,
        ));
    }
    cur.skip(10)?;

    let format_count = cur.number()? as usize;
    let mut palette: Vec<Vec<(u64, i64)>> = Vec::with_capacity(format_count);
    for _ in 0..format_count {
        palette.push(parse_format(cur.next()?.group()?)?);
    }
    let font_count = cur.number()? as usize;
    let mut fonts_test: Vec<Font> = Vec::with_capacity(font_count);
    for _ in 0..font_count {
        let g = cur.next()?.group()?;
        fonts_test.push(parse_font(g)?);
    }
    // Форматные строки («ЧДЦ=2» и подобные), затем описания типов и GUID —
    // все три списка переменной длины. В простом документе все три пусты, и
    // раздел выглядит как три нуля подряд; настоящий отчёт показал, что это
    // не константа.
    // Форматные строки («ЧДЦ=2» и подобные): `{1,<кол-во языков>,
    // {<язык>,<строка>},…}`. У пустой записи — ноль языков. Берём
    // русскую строку, а при её отсутствии — первую попавшуюся: как и
    // шаблон, формат хранится в `CellData::format_spec` для ВМ.
    let num_format_count = cur.number()? as usize;
    let mut num_formats: Vec<Option<String>> = Vec::with_capacity(num_format_count);
    for _ in 0..num_format_count {
        let g = cur.next()?.group()?;
        let lang_count = g.get(1).map_or(Ok(0), Node::number)?.max(0) as usize;
        let mut spec = None;
        for i in 0..lang_count {
            if let Some(Node::Group(pair)) = g.get(2 + i) {
                let lang = pair_text(pair_get(pair, 0));
                let value = pair_text(pair_get(pair, 1));
                if lang == "ru" || spec.is_none() {
                    spec = Some(value);
                }
            }
        }
        num_formats.push(spec);
    }
    let type_count = cur.number()? as usize;
    cur.skip(type_count)?;
    let guid = cur.number()? as usize;
    cur.skip(guid)?;
    let color_count = cur.number()? as usize;
    let mut colors: Vec<Option<Color>> = Vec::with_capacity(color_count);
    for _ in 0..color_count {
        let g = cur.next()?.group()?;
        // `{3,0,{<цвет>}}` — заданный цвет, `{3,3,{...}}` — цвет стиля,
        // который мы не моделируем.
        let own = g.get(1).map_or(Ok(3), Node::number)? == 0;
        let value = match g.get(2) {
            Some(Node::Group(v)) => v.first().map_or(Ok(0), Node::number)?,
            _ => 0,
        };
        colors.push(own.then_some(Color {
            r: (value & 0xFF) as u8,
            g: ((value >> 8) & 0xFF) as u8,
            b: ((value >> 16) & 0xFF) as u8,
        }));
    }
    // Палитра рисунков: счётчик и столько же групп с base64-данными.
    // В простом документе счётчик нулевой — тогда это и есть первый из
    // трёх нулей, которые писались раньше через `skip(3)`; поэтому чтение
    // счётчика здесь, а не отдельного `skip`, сохраняет порядок.
    let picture_count = cur.number()? as usize;
    cur.skip(picture_count)?;
    cur.skip(2)?; // 0,0
    cur.skip(1)?; // ""
    cur.skip(1)?; // 0
    if let Ok(Node::Group(settings)) = cur.next() {
        doc.show_grid = settings.get(4).map_or(Ok(1), Node::number)? != 0;
        doc.fix_left = settings.get(11).map_or(Ok(0), Node::number)?;
        doc.fix_top = settings.get(12).map_or(Ok(0), Node::number)?;
    }

    // --- применение форматов --------------------------------------------
    let value = |index: usize, bit: u32| -> Option<i64> {
        palette
            .get(index.checked_sub(1)?)?
            .iter()
            .find(|(b, _)| *b == u64::from(bit))
            .map(|(_, v)| *v)
    };
    for (r, fmt) in row_formats {
        if let Some(v) = value(fmt, bits::ROW_HEIGHT) {
            doc.set_row_height(r, v / ROW_HEIGHT_SCALE);
        }
    }
    for (c, fmt) in column_formats {
        if let Some(v) = value(fmt, bits::COL_WIDTH) {
            doc.set_col_width(c, v / COL_WIDTH_SCALE);
        }
    }
    for (r, c, fmt) in cell_formats {
        if let Some(v) = value(fmt, bits::H_ALIGN)
            && let Some(a) = HAlign::from_code(v)
        {
            doc.set_cell_h_align(r, c, a);
        }
        if let Some(v) = value(fmt, bits::V_ALIGN)
            && let Some(a) = VAlign::from_code(v)
        {
            doc.set_cell_v_align(r, c, a);
        }
        if value(fmt, bits::TEXT_PLACEMENT) == Some(3) {
            doc.set_cell_wrap(r, c, true);
        }
        if let Some(v) = value(fmt, bits::NUMBER_FORMAT) {
            doc.set_cell_numeric(r, c);
            if let Some(Some(spec)) = num_formats.get(v.max(0) as usize) {
                doc.set_cell_format_spec(r, c, spec);
            }
        }
        if let Some(v) = value(fmt, bits::FONT)
            && let Some(f) = fonts_test.get(v.max(0) as usize)
        {
            doc.set_cell_font(r, c, f.clone());
        }
        if let Some(v) = value(fmt, bits::TEXT_COLOR)
            && let Some(Some(color)) = colors.get(v.max(0) as usize)
        {
            doc.set_cell_text_color(r, c, *color);
        }
        for (bit, side) in [
            (bits::BORDER_LEFT, 0),
            (bits::BORDER_TOP, 1),
            (bits::BORDER_RIGHT, 2),
            (bits::BORDER_BOTTOM, 3),
        ] {
            if let Some(v) = value(fmt, bit)
                && let Some(&line) = line_palette.get(v.max(0) as usize)
            {
                doc.set_cell_border(
                    r,
                    c,
                    (side == 0).then_some(line),
                    (side == 1).then_some(line),
                    (side == 2).then_some(line),
                    (side == 3).then_some(line),
                );
            }
        }
        if let Some(v) = value(fmt, bits::BACK_COLOR)
            && let Some(Some(color)) = colors.get(v.max(0) as usize)
        {
            doc.set_cell_back_color(r, c, *color);
        }
    }

    // Объявленные размеры НЕ обязаны покрывать содержимое: в отчёте с
    // дополнительными наборами колонок основной набор объявлен из одной
    // колонки, а ячейки заняли пять. Своей модели у наборов нет, поэтому
    // берём большее из объявленного и занятого — иначе половина документа
    // просто исчезла бы.
    // Ширины колонок наборов лежат в той же палитре форматов, что и всё
    // остальное, поэтому наборы собираются только теперь.
    doc.column_sets = raw_sets
        .into_iter()
        .map(|(id, count, formats)| ColumnSet {
            id,
            count,
            widths: formats
                .into_iter()
                .filter_map(|(c, fmt)| Some((c, value(fmt, bits::COL_WIDTH)?)))
                .collect(),
        })
        .collect();
    doc.row_sets = bindings;

    doc.main_columns = width;
    doc.height = doc.height.max(height);
    doc.width = doc.width.max(width);
    doc.row_groups = groups;
    // Геометрия рисунков восстанавливается ПОСЛЕДНЕЙ: она считается по
    // фактическим ширинам колонок и высотам строк, а те приходят из палитры
    // форматов в хвосте файла.
    for (c1, r1, oc1, or1, c2, r2, oc2, or2) in drawing_cells_raw {
        let left = doc.col_start(c1) + oc1;
        let top = doc.row_start(r1) + or1;
        let right = doc.col_start(c2) + oc2;
        let bottom = doc.row_start(r2) + or2;
        let number_of = doc.drawings.len() + 1;
        doc.drawings.push(Drawing {
            left: left as f64 / MM_TO_QP,
            top: top as f64 / MM_TO_QP,
            width: (right - left) as f64 / MM_TO_QP,
            height: (bottom - top) as f64 / MM_TO_QP,
            name: format!("D{number_of}"),
        });
    }
    Ok(doc)
}

/// Формат — маска и значения по возрастанию установленных бит.
///
/// Маска ШИРЕ тридцати двух бит: настоящий отчёт из 1С принёс `8589954623`,
/// это тридцать четыре значащих бита. Обрезав её до `u32`, разбор считал бы
/// не то число значений и разъезжался на всём, что идёт следом.
pub(crate) fn parse_format(g: &[Node]) -> RtResult<Vec<(u64, i64)>> {
    let mask = g.first().map_or(Ok(0), Node::number)? as u64;
    let mut out = Vec::new();
    let mut i = 1;
    for bit in 0..64 {
        let b = 1u64 << bit;
        if mask & b != 0 {
            if let Some(n) = g.get(i) {
                out.push((b, n.number()?));
            }
            i += 1;
        }
    }
    Ok(out)
}

pub(crate) fn parse_font(g: &[Node]) -> RtResult<Font> {
    // Второе поле — вид записи: `0` — абсолютный шрифт с полным набором
    // полей, `2` — ссылка на другой шрифт с модификациями. У ссылки мало
    // полей и вместо кегля — группа-указатель, поэтому читаем только то,
    // что безопасно для обоих видов.
    let absolute = g.get(1).is_none_or(|n| n.number().unwrap_or(0) == 0);
    Ok(Font {
        size: if absolute {
            g.get(3).map_or(Ok(100), Node::number)? / 10
        } else {
            10
        },
        bold: g.get(7).map_or(Ok(400), Node::number)? >= 700,
        italic: g.get(8).map_or(Ok(0), Node::number)? != 0,
        underline: g.get(9).map_or(Ok(0), Node::number)? != 0,
        strikeout: g.get(10).map_or(Ok(0), Node::number)? != 0,
        face: g.get(16).map_or(String::new(), Node::text),
    })
}

pub(crate) fn pair_get(g: &[Node], i: usize) -> Option<&Node> {
    g.get(i)
}

pub(crate) fn pair_text(n: Option<&Node>) -> String {
    n.map_or(String::new(), Node::text)
}
