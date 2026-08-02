//! Побайтная сверка записи MXL с эталонами, снятыми с платформы 8.3.27.
//!
//! Эталоны в `tests/conformance/mxl/*.mxl` — это вывод самой 1С, а не наш
//! собственный: их породил `tests/conformance/mxl/generate.bsl`, прогнанный
//! через `tests/conformance/measure/1c/run-on-1c.sh`. Поэтому расхождение
//! здесь означает ошибку в НАШЕЙ записи, и «поправить эталон» — недопустимый
//! способ починки.
//!
//! Часть эталонов пока не воспроизводится (границы, значения ячеек, группы
//! строк, ландшафт, рисунки) — они лежат рядом как задел и в этот тест не
//! включены.

use bsl_rt::{
    to_mxl_bytes, Color, Font, HAlign, Line, LineStyle, Merge, NamedArea, SpreadDocData, VAlign,
};

fn oracle(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/conformance/mxl")
        .join(format!("{name}.mxl"));
    std::fs::read(&path).unwrap_or_else(|e| panic!("нет эталона {}: {e}", path.display()))
}

/// Сравнение с показом ПЕРВОГО расхождения в текстовом виде: побайтный дифф
/// на 600 байтах нечитаем, а разница почти всегда в одном месте.
fn check(name: &str, doc: &SpreadDocData) {
    let ours = to_mxl_bytes(doc);
    let theirs = oracle(name);
    if ours == theirs {
        return;
    }
    let text = |b: &[u8]| {
        String::from_utf8_lossy(&b[16..])
            .replace('\r', "")
            .to_string()
    };
    let (a, b) = (text(&ours), text(&theirs));
    let first_diff = a
        .lines()
        .zip(b.lines())
        .position(|(x, y)| x != y)
        .unwrap_or(a.lines().count().min(b.lines().count()));
    panic!(
        "{name}: расхождение со снятым с платформы эталоном, строка {}\n  наше:  {:?}\n  1С:    {:?}\n--- наше целиком ---\n{a}\n--- платформа ---\n{b}",
        first_diff + 1,
        a.lines().nth(first_diff),
        b.lines().nth(first_diff),
    );
}

fn base() -> SpreadDocData {
    let mut doc = SpreadDocData::new();
    doc.set_cell_text(0, 0, "A");
    doc
}

/// Выгрузка НАШИХ файлов для встречной проверки на платформе.
///
/// Побайтное совпадение доказано только на снятых эталонах, а писать
/// приходится и то, чего среди них нет: длинные документы, разрежённые,
/// с несколькими объединениями. Единственный способ убедиться, что такой
/// файл вообще валиден, — дать его прочитать самой 1С:
///
/// ```text
/// cargo test -p bsl-rt --test mxl_oracle -- --ignored выгрузить
/// ./tests/conformance/measure/1c/run-on-1c.sh tests/conformance/mxl/read-back.bsl
/// ```
///
/// Тест отложен намеренно: он ничего не проверяет сам, он готовит вход для
/// платформы.
#[test]
#[ignore = "готовит файлы для встречной проверки на платформе"]
fn dump_for_the_platform_crosscheck() {
    let dir = std::path::Path::new("/tmp/open-bsl-ours");
    std::fs::create_dir_all(dir).expect("не создался каталог выгрузки");

    let mut samples: Vec<(&str, SpreadDocData)> = Vec::new();

    // Длинный документ: проверяет сцепку заголовков строк на масштабе.
    let mut many = SpreadDocData::new();
    for r in 0..50 {
        for c in 0..3 {
            many.set_cell_text(r, c, &format!("R{}C{}", r + 1, c + 1));
        }
    }
    samples.push(("many-rows", many));

    // Разрежённый: между занятыми строками десятки пустых.
    let mut sparse = SpreadDocData::new();
    sparse.set_cell_text(0, 0, "первая");
    sparse.set_cell_text(9, 4, "десятая");
    sparse.set_cell_text(19, 9, "двадцатая");
    samples.push(("sparse", sparse));

    // Несколько объединений, в том числе вертикальное.
    let mut merges_doc = SpreadDocData::new();
    merges_doc.set_cell_text(0, 0, "шапка");
    merges_doc.merge(Merge::new(0, 0, 0, 3));
    merges_doc.set_cell_text(2, 0, "бок");
    merges_doc.merge(Merge::new(2, 0, 4, 0));
    merges_doc.set_cell_text(2, 2, "блок");
    merges_doc.merge(Merge::new(2, 2, 3, 3));
    samples.push(("merges", merges_doc));

    // Всё оформление сразу.
    let mut styled = SpreadDocData::new();
    styled.set_cell_text(0, 0, "заголовок");
    styled.set_cell_h_align(0, 0, HAlign::Center);
    styled.set_cell_v_align(0, 0, VAlign::Center);
    styled.set_cell_wrap(0, 0, true);
    styled.set_row_height(0, 24);
    styled.set_col_width(0, 30);
    styled.set_col_width(1, 12);
    styled.set_cell_parameter(1, 0, "Товар");
    styled.set_cell_text(1, 1, "штука");
    styled.fix_top = 1;
    styled.print_scale = 90.into();
    samples.push(("styled", styled));

    // Текст, ломающий разбор: кавычки, запятые, скобки, перевод строки.
    let mut text = SpreadDocData::new();
    text.set_cell_text(0, 0, "с \"кавычкой\" и, запятой");
    text.set_cell_text(1, 0, "две\nстроки");
    text.set_cell_text(2, 0, "{фигурные} [квадратные]");
    text.set_cell_text(3, 0, "ёжик ⌘ 日本語");
    text.set_cell_text(4, 0, &"длинный ".repeat(40));
    samples.push(("text-edges", text));

    // Палитры шрифтов и цветов: их проверка на эталонах есть, а вот
    // канонической ли выходит их НУМЕРАЦИЯ в живом документе — показывает
    // только встречный прогон.
    let mut palettes = SpreadDocData::new();
    palettes.set_cell_text(0, 0, "жирный красный");
    palettes.set_cell_font(0, 0, Font::new("Arial", 12).bold());
    palettes.set_cell_back_color(0, 0, Color::new(255, 0, 0));
    palettes.set_cell_text(1, 0, "курсив синий");
    palettes.set_cell_font(1, 0, Font::new("Times New Roman", 9).italic());
    palettes.set_cell_text_color(1, 0, Color::new(0, 0, 255));
    palettes.set_cell_text(2, 0, "он же жирный красный");
    palettes.set_cell_font(2, 0, Font::new("Arial", 12).bold());
    palettes.set_cell_back_color(2, 0, Color::new(255, 0, 0));
    palettes.set_cell_h_align(2, 0, HAlign::Right);
    samples.push(("palettes", palettes));

    // Границы: проверяется не дескриптор линии (он сверен эталонами), а
    // НУМЕРАЦИЯ в палитре линий, когда их в документе несколько и они
    // переиспользуются.
    let mut borders = SpreadDocData::new();
    let solid = Line::new(LineStyle::Solid);
    let double = Line::new(LineStyle::Double);
    let thick = Line::new(LineStyle::Solid).thick();
    borders.set_cell_text(0, 0, "шапка");
    borders.set_cell_border(0, 0, Some(double), Some(double), Some(double), Some(double));
    borders.set_cell_text(1, 0, "строка");
    borders.set_cell_border(1, 0, Some(solid), None, Some(solid), Some(thick));
    borders.set_cell_text(2, 0, "итог");
    borders.set_cell_border(2, 0, Some(double), None, None, Some(solid));
    samples.push(("borders", borders));

    // Оформление для проверки XLSX-стилей: те же семь ячеек, что platform
    // писала в своём файле, включая повтор — он обязан переиспользовать
    // стиль, а не завести восьмой.
    let mut styles = SpreadDocData::new();
    styles.set_cell_text(0, 0, "обычная");
    styles.set_cell_text(1, 0, "жирная");
    styles.set_cell_font(1, 0, Font::new("Arial", 14).bold());
    styles.set_cell_text(2, 0, "курсив");
    styles.set_cell_font(2, 0, Font::new("Times New Roman", 9).italic());
    styles.set_cell_text(3, 0, "цветная");
    styles.set_cell_back_color(3, 0, Color::new(255, 255, 0));
    styles.set_cell_text_color(3, 0, Color::new(255, 0, 0));
    styles.set_cell_text(4, 0, "в рамке");
    let line = Line::new(LineStyle::Solid);
    styles.set_cell_border(4, 0, Some(line), Some(line), Some(line), Some(line));
    styles.set_cell_text(5, 0, "по центру с переносом");
    styles.set_cell_h_align(5, 0, HAlign::Center);
    styles.set_cell_v_align(5, 0, VAlign::Center);
    styles.set_cell_wrap(5, 0, true);
    styles.set_cell_text(6, 0, "снова жирная");
    styles.set_cell_font(6, 0, Font::new("Arial", 14).bold());
    std::fs::write(dir.join("стили.xlsx"), bsl_rt::to_xlsx_bytes(&styles))
        .expect("не записался xlsx со стилями");

    for (name, doc) in &samples {
        let path = dir.join(format!("{name}.mxl"));
        std::fs::write(&path, to_mxl_bytes(doc)).expect("не записался образец");
        println!("выгружено: {}", path.display());
    }
}

#[test]
fn empty_document() {
    check("00-empty", &SpreadDocData::new());
}

#[test]
fn single_cell() {
    check("01-base", &base());
}

#[test]
fn second_row() {
    let mut doc = base();
    doc.set_cell_text(1, 0, "B");
    check("02-row2", &doc);
}

#[test]
fn second_column() {
    let mut doc = base();
    doc.set_cell_text(0, 1, "B");
    check("03-col2", &doc);
}

#[test]
fn sparse_document() {
    let mut doc = SpreadDocData::new();
    doc.set_cell_text(2, 4, "X");
    check("04-far", &doc);
}

#[test]
fn quotes_and_braces_in_text() {
    let mut doc = SpreadDocData::new();
    doc.set_cell_text(0, 0, "ка\"вычка, запятая {скобка}");
    check("05-escape", &doc);
}

#[test]
fn newline_inside_cell() {
    let mut doc = SpreadDocData::new();
    doc.set_cell_text(0, 0, "раз\nдва");
    check("06-multiline", &doc);
}

#[test]
fn column_width() {
    let mut doc = base();
    doc.set_col_width(0, 25);
    check("10-width", &doc);
}

#[test]
fn row_height() {
    let mut doc = base();
    doc.set_row_height(0, 30);
    check("11-height", &doc);
}

#[test]
fn merged_cells() {
    let mut doc = base();
    doc.merge(Merge::new(0, 0, 0, 2));
    check("12-merge", &doc);
}

/// Объединение одной ячейки в файл всё равно попадает.
#[test]
fn merge_of_one_cell() {
    let mut doc = SpreadDocData::new();
    doc.set_cell_text(0, 0, "A");
    doc.merge(Merge::new(0, 0, 0, 0));
    check("m01-single", &doc);
}

/// Записи СОРТИРУЮТСЯ, а не пишутся в порядке вызовов: здесь объединения
/// заданы снизу вверх, а в файле лежат сверху вниз.
#[test]
fn merges_are_sorted() {
    let mut doc = SpreadDocData::new();
    doc.merge(Merge::new(2, 0, 2, 1));
    doc.merge(Merge::new(0, 0, 0, 1));
    doc.merge(Merge::new(1, 0, 1, 1));
    check("m02-order", &doc);
}

/// Координаты в записи АБСОЛЮТНЫЕ, и колонка идёт первой. На объединении,
/// начинающемся не в начале координат, это единственное, что видно.
#[test]
fn merge_beyond_content() {
    let mut doc = SpreadDocData::new();
    doc.merge(Merge::new(4, 4, 4, 6));
    check("m03-beyond", &doc);
}

#[test]
fn two_by_two_block() {
    let mut doc = SpreadDocData::new();
    // Платформа присваивала `Текст` всей области 2x2 сразу, то есть завела
    // все четыре ячейки; объединение потом накрытые убрало, а строку
    // оставило пустой.
    for r in 1..=2 {
        for c in 1..=2 {
            doc.set_cell_text(r, c, "блок");
        }
    }
    doc.merge(Merge::new(1, 1, 2, 2));
    check("m10-block", &doc);
}

#[test]
fn two_adjacent_merges() {
    let mut doc = SpreadDocData::new();
    doc.merge(Merge::new(0, 0, 0, 1));
    doc.merge(Merge::new(0, 2, 0, 3));
    check("m11-adjacent", &doc);
}

#[test]
fn repeated_merge_adds_no_record() {
    let mut doc = SpreadDocData::new();
    doc.merge(Merge::new(0, 0, 0, 2));
    doc.merge(Merge::new(0, 0, 0, 2));
    check("m12-twice", &doc);
}

/// `Разъединить` убирает запись целиком, а ширину таблицы не сокращает.
#[test]
fn unmerge_drops_the_record() {
    let mut doc = SpreadDocData::new();
    doc.set_cell_text(0, 0, "A");
    doc.merge(Merge::new(0, 0, 0, 2));
    doc.unmerge(0, 0, 0, 2);
    check("m05-unmerge", &doc);
}

/// Объединение целых строк и целых колонок живёт в ДРУГИХ списках.
#[test]
fn whole_row_merge() {
    let mut doc = SpreadDocData::new();
    doc.set_cell_text(0, 0, "A");
    doc.merge_rows(1, 2);
    check("m07-rows", &doc);
}

#[test]
fn whole_column_merge() {
    let mut doc = SpreadDocData::new();
    doc.set_cell_text(0, 0, "A");
    doc.merge_columns(1, 3);
    check("m06-columns", &doc);
}

/// Разрыв объединения колонок в отдельной строке — запись с флагом 2.
#[test]
fn merge_cancelled_in_one_row() {
    let mut doc = SpreadDocData::new();
    doc.set_cell_text(0, 0, "A");
    doc.set_cell_text(2, 0, "C");
    doc.merge_columns(0, 2);
    doc.unmerge_cells(1, 0, 1, 2);
    check("m13-unmerge-row", &doc);
}

/// Объединение переезжает вместе с областью при `Вывести`.
#[test]
fn merge_survives_output() {
    let mut source = SpreadDocData::new();
    source.set_cell_text(0, 0, "шапка");
    source.merge(Merge::new(0, 0, 0, 2));
    let mut doc = SpreadDocData::new();
    doc.append(&source);
    doc.append(&source);
    check("m14-output", &doc);
}

#[test]
fn cell_parameter() {
    let mut doc = SpreadDocData::new();
    doc.set_cell_parameter(0, 0, "ПарамЯчейки");
    check("21-param", &doc);
}

#[test]
fn grid_hidden() {
    let mut doc = base();
    doc.show_grid = false;
    check("17-nogrid", &doc);
}

#[test]
fn fixed_top() {
    let mut doc = base();
    doc.fix_top = 1;
    check("16-fix", &doc);
}

/// Фиксация пишется ЧИСЛОМ, а не флагом, и слева она в другом поле.
#[test]
fn fixation_is_a_count_and_has_a_left_field() {
    let mut doc = base();
    doc.fix_top = 3;
    check("fix-top3", &doc);

    let mut doc = base();
    doc.fix_left = 2;
    check("fix-left", &doc);
}

/// Единицы ширины и высоты: восьмые доли знака и четверти пункта.
#[test]
fn column_width_units() {
    for (value, name) in [(1, "w1"), (2, "w2"), (10, "w10"), (100, "w100")] {
        let mut doc = base();
        doc.set_col_width(0, value);
        check(name, &doc);
    }
}

#[test]
fn row_height_units() {
    for (value, name) in [(1, "h1"), (5, "h5"), (100, "h100")] {
        let mut doc = base();
        doc.set_row_height(0, value);
        check(name, &doc);
    }
}

#[test]
fn two_columns_of_different_width() {
    let mut doc = base();
    doc.set_col_width(0, 10);
    doc.set_col_width(1, 20);
    doc.set_cell_text(0, 1, "B");
    check("two-widths", &doc);
}

/// Одинаковое оформление занимает ОДИН элемент палитры на обе колонки.
#[test]
fn equal_width_reuses_the_format() {
    let mut doc = base();
    doc.set_col_width(0, 10);
    doc.set_col_width(1, 10);
    doc.set_cell_text(0, 1, "B");
    check("same-width", &doc);
}

#[test]
fn horizontal_alignment() {
    for (align, name) in [
        (HAlign::Auto, "halign-Авто"),
        (HAlign::Left, "halign-Лево"),
        (HAlign::Center, "halign-Центр"),
        (HAlign::Right, "halign-Право"),
    ] {
        let mut doc = base();
        doc.set_cell_h_align(0, 0, align);
        check(name, &doc);
    }
}

#[test]
fn vertical_alignment() {
    for (align, name) in [
        (VAlign::Top, "valign-Верх"),
        (VAlign::Center, "valign-Центр"),
        (VAlign::Bottom, "valign-Низ"),
    ] {
        let mut doc = base();
        doc.set_cell_v_align(0, 0, align);
        check(name, &doc);
    }
}

#[test]
fn word_wrap() {
    let mut doc = base();
    doc.set_cell_wrap(0, 0, true);
    check("wrap", &doc);
}

/// Формат строки занимает палитру раньше формата ячейки — от этого зависят
/// НОМЕРА, а значит и байты.
#[test]
fn row_format_precedes_cell_format() {
    let mut doc = base();
    doc.set_cell_h_align(0, 0, HAlign::Center);
    doc.set_row_height(0, 30);
    check("combo-align-height", &doc);
}

/// Строка, у которой задана только высота, пишется заголовком из трёх полей.
#[test]
fn row_without_cells() {
    let mut doc = SpreadDocData::new();
    doc.set_row_height(1, 40);
    doc.set_cell_text(0, 0, "A");
    check("empty-row-height", &doc);
}

/// Ячейка с оформлением, но без текста, — без текстового блока вовсе.
#[test]
fn cell_without_text() {
    let mut doc = SpreadDocData::new();
    doc.set_cell_back_color(0, 0, Color::new(255, 255, 0));
    check("format-no-text", &doc);
}

/// Цвет упакован как `B<<16|G<<8|R`: красный это 255, синий — 16711680.
#[test]
fn background_color() {
    let mut doc = base();
    doc.set_cell_back_color(0, 0, Color::new(255, 0, 0));
    check("color1", &doc);

    let mut doc = base();
    doc.set_cell_back_color(0, 0, Color::new(0, 0, 255));
    check("color2", &doc);
}

#[test]
fn text_color() {
    let mut doc = base();
    doc.set_cell_text_color(0, 0, Color::new(255, 0, 0));
    check("color3", &doc);
}

#[test]
fn yellow_background() {
    let mut doc = base();
    doc.set_cell_back_color(0, 0, Color::new(255, 255, 0));
    check("14-bgcolor", &doc);
}

/// Два цвета в одной ячейке — один элемент палитры форматов с двумя битами.
#[test]
fn text_and_background_color_together() {
    let mut doc = base();
    doc.set_cell_back_color(0, 0, Color::new(255, 0, 0));
    doc.set_cell_text_color(0, 0, Color::new(0, 0, 255));
    check("two-colors", &doc);
}

/// Одинаковый цвет у двух ячеек занимает палитру один раз.
#[test]
fn equal_color_is_reused() {
    let mut doc = SpreadDocData::new();
    doc.set_cell_text(0, 0, "A");
    doc.set_cell_text(1, 0, "B");
    doc.set_cell_back_color(0, 0, Color::new(255, 0, 0));
    doc.set_cell_back_color(1, 0, Color::new(255, 0, 0));
    check("same-color", &doc);
}

/// Палитра занимается ПО СТРОКАМ: формат строки, потом форматы её ячеек, и
/// лишь после всех строк — форматы колонок. Здесь у первой строки высоты
/// нет, поэтому номер 1 достаётся ячейке, а высота второй получает 2.
#[test]
fn palette_is_filled_row_by_row() {
    let mut doc = SpreadDocData::new();
    doc.set_cell_text(0, 0, "A");
    doc.set_cell_h_align(0, 0, HAlign::Center);
    doc.set_cell_text(1, 0, "B");
    doc.set_row_height(1, 30);
    check("palette-order", &doc);
}

#[test]
fn vertical_merge() {
    let mut doc = SpreadDocData::new();
    doc.merge(Merge::new(0, 0, 2, 0));
    doc.set_cell_text(0, 0, "верт");
    check("merge-vertical", &doc);
}

/// `Параметр` занимает то же место, что и текст: заданный после текста, он
/// его ЗАМЕЩАЕТ. Обратно платформа читает такую ячейку как текст, а
/// `.Параметр` отдаёт пустым даже из своего собственного файла — измерено.
#[test]
fn parameter_replaces_text() {
    let mut doc = SpreadDocData::new();
    doc.set_cell_text(0, 0, "Текст");
    doc.set_cell_parameter(0, 0, "Пар");
    check("text-then-param", &doc);
}

#[test]
fn parameter_in_empty_document() {
    let mut doc = SpreadDocData::new();
    doc.set_cell_parameter(0, 0, "ПрямоСейчас");
    check("param-alone", &doc);
}

/// Именованные области. Задаются присваиванием `Область(...).Имя`, лежат в
/// четвёртом списке подряд и отсортированы ПО ИМЕНИ, а не по порядку
/// присваивания.
fn named_sample() -> SpreadDocData {
    let mut doc = SpreadDocData::new();
    doc.set_cell_text(0, 0, "шапка");
    doc.set_cell_text(1, 0, "строка1");
    doc.set_cell_text(1, 1, "цена1");
    doc.set_cell_text(2, 0, "строка2");
    doc.set_cell_text(3, 0, "итого");
    doc
}

#[test]
fn named_area() {
    let mut doc = named_sample();
    doc.set_area_name("Заголовок", NamedArea::rect(0, 0, 0, 0));
    check("n01-one", &doc);
}

#[test]
fn two_named_areas_go_alphabetically() {
    let mut doc = named_sample();
    doc.set_area_name("Шапка", NamedArea::rect(0, 0, 0, 0));
    doc.set_area_name("Строки", NamedArea::rect(1, 0, 2, 1));
    check("n02-two", &doc);
}

#[test]
fn three_named_areas() {
    let mut doc = named_sample();
    doc.set_area_name("А", NamedArea::rect(0, 0, 0, 0));
    doc.set_area_name("Б", NamedArea::rect(1, 0, 1, 1));
    doc.set_area_name("В", NamedArea::rect(3, 0, 3, 0));
    check("n03-three", &doc);
}

/// У области-строк колонки заменены на -1, у области-колонок — строки.
#[test]
fn named_row_area() {
    let mut doc = named_sample();
    doc.set_area_name("ЦелыеСтроки", NamedArea::rows(1, 2));
    check("n04-rows", &doc);
}

#[test]
fn named_column_area() {
    let mut doc = named_sample();
    doc.set_area_name("ЦелыеКолонки", NamedArea::columns(0, 1));
    check("n05-columns", &doc);
}

/// Пара «высота|ширина» для пересечения — то самое, чем режут ценники.
#[test]
fn area_intersection() {
    let mut doc = named_sample();
    doc.set_area_name("ВысотаБлока", NamedArea::rows(1, 2));
    doc.set_area_name("ШиринаБлока", NamedArea::columns(0, 0));
    check("n06-cross", &doc);
}

#[test]
fn renaming_keeps_one_record() {
    let mut doc = named_sample();
    doc.set_area_name("Старое", NamedArea::rect(0, 0, 0, 0));
    doc.clear_area_name("Старое");
    doc.set_area_name("Новое", NamedArea::rect(0, 0, 0, 0));
    check("n07-rename", &doc);
}

/// Пустое имя снимает имя, и раздел исчезает целиком.
#[test]
fn clearing_the_name() {
    let mut doc = named_sample();
    doc.set_area_name("Снять", NamedArea::rect(0, 0, 0, 0));
    doc.clear_area_name("Снять");
    check("n08-clear", &doc);
}

/// Повторное имя игнорируется: за ним остаётся ПЕРВАЯ область.
#[test]
fn duplicate_name_is_ignored() {
    let mut doc = named_sample();
    doc.set_area_name("Дубль", NamedArea::rect(0, 0, 0, 0));
    doc.set_area_name("Дубль", NamedArea::rect(1, 0, 1, 0));
    check("n09-dup", &doc);
}

/// Шрифты: кегль в файле умножен на десять, насыщенность — 400 или 700,
/// начертания — отдельными флагами.
#[test]
fn fonts_test() {
    for (font, name) in [
        (Font::new("Arial", 10), "font1"),
        (Font::new("Arial", 20), "font2"),
        (Font::new("Times New Roman", 10), "font3"),
        (Font::new("Arial", 10).italic(), "font4"),
        (Font::new("Arial", 10).underline(), "font5"),
        (Font::new("Arial", 10).strikeout(), "font6"),
    ] {
        let mut doc = base();
        doc.set_cell_font(0, 0, font);
        check(name, &doc);
    }
}

#[test]
fn bold() {
    let mut doc = base();
    doc.set_cell_font(0, 0, Font::new("Arial", 10).bold());
    check("09-bold", &doc);
}

#[test]
fn two_different_fonts() {
    let mut doc = base();
    doc.set_cell_text(1, 0, "B");
    doc.set_cell_font(0, 0, Font::new("Arial", 10).bold());
    doc.set_cell_font(1, 0, Font::new("Arial", 20));
    check("two-fonts", &doc);
}

#[test]
fn equal_font_is_reused() {
    let mut doc = base();
    doc.set_cell_text(1, 0, "B");
    doc.set_cell_font(0, 0, Font::new("Arial", 10).bold());
    doc.set_cell_font(1, 0, Font::new("Arial", 10).bold());
    check("same-font", &doc);
}

/// Шрифт и выравнивание — ОДИН элемент палитры с маской 257.
#[test]
fn font_and_alignment_in_one_format() {
    let mut doc = base();
    doc.set_cell_font(0, 0, Font::new("Arial", 10).bold());
    doc.set_cell_h_align(0, 0, HAlign::Center);
    check("combo-font-align", &doc);
}

#[test]
fn print_scale() {
    let mut doc = base();
    doc.print_scale = Some(80);
    check("print-scale", &doc);
}

#[test]
fn print_area_test() {
    let mut doc = base();
    doc.print_area = Some((0, 0, 0, 0));
    check("print-area", &doc);
}

/// Разбор ВСЕГО корпуса эталонов: 108 файлов, снятых с платформы, включая
/// те, чьи разделы мы ещё не моделируем. Читатель обязан либо разобрать
/// файл, либо честно вернуть ошибку, но не паниковать и не зависать.
#[test]
fn every_oracle_parses() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance/mxl");
    let mut parsed = 0;
    let mut refused = Vec::new();
    for entry in std::fs::read_dir(&dir).expect("нет каталога эталонов") {
        let path = entry.expect("не читается запись каталога").path();
        if path.extension().and_then(|e| e.to_str()) != Some("mxl") {
            continue;
        }
        let bytes = std::fs::read(&path).expect("не читается эталон");
        match bsl_rt::from_mxl_bytes(&bytes) {
            Ok(_) => parsed += 1,
            Err(e) => refused.push(format!(
                "{}: {e}",
                path.file_name().unwrap().to_string_lossy()
            )),
        }
    }
    // Отказов больше не осталось: весь снятый с платформы корпус обязан
    // разбираться. Любая ошибка здесь — дыра в разборе.
    let others: Vec<&String> = refused
        .iter()
        .filter(|s| !s.contains("есть рисунки") && !s.contains("ячейки со значением"))
        .collect();
    assert!(
        others.is_empty(),
        "не разобрались по неожиданной причине: {others:#?}"
    );
    assert!(
        parsed >= 110,
        "разобрано подозрительно мало: {parsed} при {} отказах",
        refused.len()
    );
}

/// Круговорот через нашу же пару «разбор — запись»: то, что мы умеем
/// писать, обязано после чтения писаться теми же байтами. Файлы с ещё не
/// поддержанными разделами (шрифты, цвета, границы, рисунки, значения)
/// сюда не входят — для них круговорот заведомо не полон.
#[test]
fn roundtrip_on_the_supported_subset() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance/mxl");
    // Ровно те эталоны, что воспроизводит наша ЗАПИСЬ: раз мы пишем их
    // побайтно, то и прочитать обязаны без потерь. Файлы с ещё не
    // поддержанными разделами сюда не входят по построению.
    let names = [
        "00-empty",
        "01-base",
        "02-row2",
        "03-col2",
        "04-far",
        "05-escape",
        "06-multiline",
        "09-bold",
        "10-width",
        "11-height",
        "12-merge",
        "14-bgcolor",
        "16-fix",
        "17-nogrid",
        "21-param",
        "color1",
        "color2",
        "color3",
        "two-colors",
        "same-color",
        "font1",
        "font2",
        "font3",
        "font4",
        "font5",
        "font6",
        "two-fonts",
        "same-font",
        "combo-font-align",
        "combo-align-height",
        "halign-Авто",
        "halign-Лево",
        "halign-Центр",
        "halign-Право",
        "valign-Верх",
        "valign-Центр",
        "valign-Низ",
        "wrap",
        "w1",
        "w2",
        "w10",
        "w100",
        "h1",
        "h5",
        "h100",
        "two-widths",
        "same-width",
        "palette-order",
        "empty-row-height",
        "format-no-text",
        "print-scale",
        "print-area",
        "fix-left",
        "fix-top3",
        "13-border",
        "border-ГраницаСлева",
        "border-ГраницаСверху",
        "border-ГраницаСправа",
        "border-ГраницаСнизу",
        "line1",
        "line2",
        "line3",
        "border-all",
        "two-lines",
        "all-at-once",
        "07-number",
        "08-date",
        "two-values",
        "value-plus",
        "detail",
        "detail-param",
        "g1-one",
        "g2-three-rows",
        "g3-other-name",
        "g4-collapsed",
        "g5-two",
        "g6-nested",
        "g7-noname",
        "19-group",
        "draw-t0",
        "draw-left10",
        "draw-left100",
        "draw-top10",
        "wide-col",
        "tall-row",
        "two-draw",
        "m01-single",
        "m02-order",
        "m03-beyond",
        "m05-unmerge",
        "m06-columns",
        "m07-rows",
        "m10-block",
        "m11-adjacent",
        "m12-twice",
        "m13-unmerge-row",
        "m14-output",
        "merge-vertical",
        "n01-one",
        "n02-two",
        "n03-three",
        "n04-rows",
        "n05-columns",
        "n06-cross",
        "n07-rename",
        "n08-clear",
        "n09-dup",
        "param-alone",
        "text-then-param",
    ];
    for name in names {
        let oracle = std::fs::read(dir.join(format!("{name}.mxl"))).expect("нет эталона");
        let doc = bsl_rt::from_mxl_bytes(&oracle)
            .unwrap_or_else(|e| panic!("{name}: не разобрался: {e}"));
        let ours = bsl_rt::to_mxl_bytes(&doc);
        assert!(
            ours == oracle,
            "{name}: круговорот изменил файл\n--- после чтения и записи ---\n{}\n--- эталон ---\n{}",
            String::from_utf8_lossy(&ours[16..]).replace('\r', ""),
            String::from_utf8_lossy(&oracle[16..]).replace('\r', ""),
        );
    }
}

/// Разбор XML-макета. Эталон снят с платформы: `СериализаторXDTO.ЗаписатьXML`
/// выдаёт табличный документ ровно в том формате, что описан спецификацией
/// макетов, — значит и проверять разбор можно её же выводом, а не сочинённым
/// от руки файлом.
#[test]
fn xml_template_parsing() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/conformance/mxl/template-basic.xml");
    let text = std::fs::read_to_string(&path).expect("нет эталона макета");
    let doc = bsl_rt::from_template_xml(&text).expect("макет не разобрался");

    assert_eq!((doc.height(), doc.width()), (3, 3), "размеры документа");
    assert_eq!(doc.cell_text(0, 0), "Отчёт");
    assert_eq!(doc.cell_text(1, 0), "Товар");
    assert_eq!(doc.cell_text(1, 1), "Цена");
    // Пустой `<v8:lang/>` — это ПАРАМЕТР, а не текст.
    assert_eq!(doc.cell_parameter(2, 0), "Номенклатура");
    assert_eq!(doc.cell_parameter(2, 1), "Сумма");
    assert_eq!(doc.cell_text(2, 0), "", "параметр не должен стать текстом");

    // Единицы в палитре внутренние: 240 -> 30 знаков, 100 -> 25 пунктов.
    assert_eq!(doc.col_widths().get(&0), Some(&30));
    assert_eq!(doc.rows()[&0].height, Some(25));

    assert_eq!(
        doc.merges(),
        [Merge::new(0, 0, 0, 2)],
        "объединение задано добавочными колонками"
    );
    assert_eq!(
        doc.area_named("Шапка").map(|a| (a.r1, a.c1, a.r2, a.c2)),
        Some((0, 0, 0, 0))
    );
    assert_eq!(
        doc.area_named("Строка").map(|a| (a.r1, a.c1, a.r2, a.c2)),
        Some((2, 0, 2, 1))
    );
}

/// Границы. Палитра линий живёт в ШАПКЕ файла, а формат ссылается на неё
/// битами 2, 4, 8, 16 — по одному на сторону.
#[test]
fn borders_by_side() {
    let solid = Line::new(LineStyle::Solid);
    for (name, l, t, r, b) in [
        ("border-ГраницаСлева", Some(solid), None, None, None),
        ("border-ГраницаСверху", None, Some(solid), None, None),
        ("border-ГраницаСправа", None, None, Some(solid), None),
        ("border-ГраницаСнизу", None, None, None, Some(solid)),
    ] {
        let mut doc = base();
        doc.set_cell_border(0, 0, l, t, r, b);
        check(name, &doc);
    }
}

/// `Обвести(Линия)` с одним аргументом ставит ТОЛЬКО левую границу.
#[test]
fn outline_with_one_argument() {
    let mut doc = base();
    doc.set_cell_border(0, 0, Some(Line::new(LineStyle::Solid)), None, None, None);
    check("13-border", &doc);
}

#[test]
fn line_styles_and_width() {
    for (name, line) in [
        ("line1", Line::new(LineStyle::Solid).thick()),
        ("line2", Line::new(LineStyle::Dotted)),
        ("line3", Line::new(LineStyle::Double)),
    ] {
        let mut doc = base();
        doc.set_cell_border(0, 0, Some(line), None, None, None);
        check(name, &doc);
    }
}

/// Четыре стороны одной линией — одна запись в палитре и маска 30.
#[test]
fn outline_on_all_sides() {
    let line = Line::new(LineStyle::Solid);
    let mut doc = SpreadDocData::new();
    doc.set_cell_text(0, 0, "A");
    doc.set_cell_border(0, 0, Some(line), Some(line), Some(line), Some(line));
    check("border-all", &doc);
}

/// Две разные линии — две записи в палитре.
#[test]
fn two_different_lines() {
    let mut doc = SpreadDocData::new();
    doc.set_cell_text(0, 0, "A");
    doc.set_cell_border(
        0,
        0,
        None,
        Some(Line::new(LineStyle::Solid)),
        None,
        Some(Line::new(LineStyle::Double)),
    );
    check("two-lines", &doc);
}

/// Шрифт, граница, цвет и выравнивание в одной ячейке — один формат.
#[test]
fn all_styling_at_once() {
    let mut doc = base();
    doc.set_cell_font(0, 0, Font::new("Arial", 10).bold());
    doc.set_cell_border(0, 0, Some(Line::new(LineStyle::Solid)), None, None, None);
    doc.set_cell_back_color(0, 0, Color::new(255, 0, 0));
    doc.set_cell_h_align(0, 0, HAlign::Right);
    check("all-at-once", &doc);
}

/// Ячейки со значением. В файл уходит ПРЕДСТАВЛЕНИЕ строкой — тип не
/// сохраняется вовсе, поэтому число, дата и булево выглядят одинаково.
#[test]
fn value_cells() {
    for (name, presentation) in [
        ("07-number", "42"),
        ("08-date", "02.08.2026 0:00:00"),
        ("value-plus", "42"),
    ] {
        let mut doc = SpreadDocData::new();
        doc.set_cell_value(0, 0, presentation);
        check(name, &doc);
    }
}

/// Описание типа и GUID — ОДНИ на документ, сколько бы значений в нём ни
/// было.
#[test]
fn two_values_share_the_type_descriptor() {
    let mut doc = SpreadDocData::new();
    doc.set_cell_value(0, 0, "1");
    doc.set_cell_value(1, 0, "2");
    check("two-values", &doc);
}

/// Расшифровка и её параметр — биты 4 и 8 маски ячейки. Расшифровка идёт
/// ПЕРЕД текстом и обёрнута тегом типа, параметр — голой строкой.
#[test]
fn cell_drilldown() {
    let mut doc = base();
    doc.set_cell_detail(0, 0, "РасшифровкаЯчейки");
    check("detail", &doc);

    let mut doc = base();
    doc.set_cell_detail_param(0, 0, "ПарамРасшифровки");
    check("detail-param", &doc);
}

/// Группы строк. Границы закрепляются при закрытии, уровень вложенности
/// лежит в блоке числом, свёрнутость — отдельным флагом.
#[test]
fn row_groups_test() {
    let mut doc = SpreadDocData::new();
    doc.begin_row_group("Одна", false);
    doc.set_cell_text(0, 0, "A");
    doc.end_row_group();
    check("g1-one", &doc);

    let mut doc = SpreadDocData::new();
    doc.begin_row_group("Одна", false);
    doc.set_cell_text(0, 0, "A");
    doc.set_cell_text(1, 0, "B");
    doc.set_cell_text(2, 0, "C");
    doc.end_row_group();
    check("g2-three-rows", &doc);
}

#[test]
fn collapsed_group() {
    let mut doc = SpreadDocData::new();
    doc.begin_row_group("Свёрнутая", true);
    doc.set_cell_text(0, 0, "A");
    doc.end_row_group();
    check("g4-collapsed", &doc);
}

#[test]
fn two_groups_in_a_row() {
    let mut doc = SpreadDocData::new();
    doc.begin_row_group("Первая", false);
    doc.set_cell_text(0, 0, "A");
    doc.end_row_group();
    doc.begin_row_group("Вторая", false);
    doc.set_cell_text(1, 0, "B");
    doc.end_row_group();
    check("g5-two", &doc);
}

/// Вложенная группа получает уровень 1, внешняя остаётся на нуле.
#[test]
fn nested_groups() {
    let mut doc = SpreadDocData::new();
    doc.begin_row_group("Внешняя", false);
    doc.set_cell_text(0, 0, "A");
    doc.begin_row_group("Внутренняя", false);
    doc.set_cell_text(1, 0, "B");
    doc.end_row_group();
    doc.end_row_group();
    check("g6-nested", &doc);
}

/// Группа без имени — локализованная строка вырождается в `{1,0}`.
#[test]
fn group_without_a_name() {
    let mut doc = SpreadDocData::new();
    doc.begin_row_group("", false);
    doc.set_cell_text(0, 0, "A");
    doc.end_row_group();
    check("g7-noname", &doc);
}

/// Рисунки. Геометрия задаётся в миллиметрах, а в файле раскладывается на
/// «номер строки или колонки плюс смещение» — по ФАКТИЧЕСКОЙ сетке.
#[test]
fn drawing_without_geometry() {
    let mut doc = base();
    doc.add_drawing(0.0, 0.0, 0.0, 0.0);
    check("draw-t0", &doc);
}

#[test]
fn drawing_with_offset() {
    let mut doc = base();
    doc.add_drawing(10.0, 0.0, 0.0, 0.0);
    check("draw-left10", &doc);

    let mut doc = base();
    doc.add_drawing(100.0, 0.0, 0.0, 0.0);
    check("draw-left100", &doc);

    let mut doc = base();
    doc.add_drawing(0.0, 10.0, 0.0, 0.0);
    check("draw-top10", &doc);
}

/// Заданная ширина колонки сдвигает разложение: 50 знаков — это 1050
/// четвертей пункта, по 21 на знак.
#[test]
fn drawing_over_a_wide_column() {
    let mut doc = SpreadDocData::new();
    doc.set_cell_text(0, 0, "A");
    doc.set_col_width(0, 50);
    doc.add_drawing(100.0, 0.0, 50.0, 0.0);
    check("wide-col", &doc);
}

/// Заданная высота строки — так же по вертикали.
#[test]
fn drawing_over_a_tall_row() {
    let mut doc = SpreadDocData::new();
    doc.set_cell_text(0, 0, "A");
    doc.set_row_height(0, 60);
    doc.add_drawing(0.0, 50.0, 0.0, 20.0);
    check("tall-row", &doc);
}

#[test]
fn two_drawings() {
    let mut doc = SpreadDocData::new();
    doc.set_cell_text(0, 0, "A");
    doc.add_drawing(10.0, 0.0, 0.0, 0.0);
    doc.add_drawing(30.0, 0.0, 0.0, 0.0);
    check("two-draw", &doc);
}

/// Настоящий отчёт из 1С, а не документ, собранный пробой.
///
/// Он принёс сразу пять особенностей, которых не было ни в одном
/// синтетическом эталоне: версию формата 11 вместо 12, три дополнительных
/// набора колонок с привязкой строк, маску формата шире тридцати двух бит,
/// список форматных строк и объявленную ширину в одну колонку при пяти
/// занятых. Каждая из них ломала разбор.
///
/// Содержимое ОБЕЗЛИЧЕНО: значения заменены по имени колонки
/// («Организация 1», «Исполнитель 2», …), суммы — нейтральными числами той
/// же формы. Разбор смотрит на структуру, а не на текст, поэтому все пять
/// особенностей на месте.
#[test]
fn a_real_report_from_1c() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/conformance/mxl/report-real.mxl");
    let bytes = std::fs::read(&path).expect("нет эталона отчёта");
    let doc = bsl_rt::from_mxl_bytes(&bytes).expect("отчёт не разобрался");

    // Размеры и содержимое сверены с тем, что показывает сама платформа.
    assert_eq!((doc.height(), doc.width()), (28, 5));
    assert_eq!(doc.cell_text(1, 1), "Дата начала: 01.01.2020 0:00:00");
    assert_eq!(doc.cell_text(5, 1), "Исполнитель");
    assert_eq!(
        doc.cell_text(6, 4),
        "1\u{a0}000,01",
        "разделитель групп — неразрывный пробел"
    );
    assert_eq!(doc.cell_text(27, 4), "22\u{a0}000,22");

    // Группы строк отчёта: три уровня свёртки по исполнителям.
    assert_eq!(doc.row_groups().len(), 3);

    // Денежные ячейки помечены ЧИСЛОВЫМ ФОРМАТОМ — по нему выгрузка и
    // решает, писать число или строку. Саму разметку листа проверяет
    // юнит-тест в `xlsx`: архив теперь сжат, и искать в его байтах нечего.
    assert!(doc.cell_numeric(6, 4), "сумма — с числовым форматом");
    assert!(doc.cell_numeric(27, 4), "итог — с числовым форматом");

    // Три дополнительных набора колонок с привязкой строк.
    assert_eq!(doc.column_sets().len(), 3);
    assert_eq!(
        doc.column_sets()[2].count,
        5,
        "табличная часть — пять колонок"
    );

    // Физическая сетка — объединение границ всех наборов. У платформы в её
    // собственной выгрузке этого отчёта тоже ВОСЕМЬ колонок.
    assert_eq!(doc.physical_grid().len() - 1, 8);
}
