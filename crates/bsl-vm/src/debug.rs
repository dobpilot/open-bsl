use super::{HostIo, LinkedComponents, ModuleState, Task, run_dynamic_snippet};
use bsl_bytecode::Program;
use bsl_rt::BslValue;

/// Один активный вызов. Регистры кадра не хранятся отдельным `Vec` — все
/// кадры делят один сквозной стек значений (`Vm::stack`), кадр — это лишь
/// окно в него, как в Lua.
/// Что делать прогону после того, как крючок отладчика вернул управление.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugAction {
    /// Исполнять дальше.
    Continue,
    /// Прекратить прогон: редактор попросил остановиться совсем.
    Terminate,
}

/// Где стоит прогон в момент вызова крючка.
///
/// Кадры отдаются как `(модуль, чанк, pc)` и только на чтение: `Frame` —
/// внутреннее устройство VM, и открывать его наружу ради отладчика
/// значило бы сделать публичным то, что меняется от версии к версии.
pub struct DebugPosition<'a> {
    /// Кадры от внешнего к внутреннему; последний — текущий.
    pub frames: &'a [(u32, usize, usize)],
    /// Строка исходника инструкции, которая сейчас исполнится.
    ///
    /// `None` — у образа нет таблицы строк, то есть он собран без сведений
    /// об отладке. Считает её VM, а не хост: таблица лежит в `Program`, а
    /// программу текущего кадра резолвит драйвер — хост про каталог
    /// модулей ничего не знает.
    ///
    /// Только для ТЕКУЩЕГО кадра; строки остальных даёт
    /// [`DebugValues::line_of`], и по той же причине — лениво.
    pub line: Option<u32>,
    /// Доступ к локальным переменным кадров — ЛЕНИВЫЙ.
    ///
    /// Собирать имена и значения на каждой инструкции значило бы копировать
    /// весь кадр там, где отладчик почти всегда просто идёт дальше. Здесь
    /// лежит только ссылка; работа делается, когда спросят, то есть на
    /// остановке.
    pub values: &'a mut dyn DebugValues,
}

/// Чтение локальных и вычисление выражений в остановленном прогоне.
pub trait DebugValues {
    /// Имена и значения локальных кадра `index` (нумерация — как в
    /// [`DebugPosition::frames`]).
    ///
    /// Пусто, если у чанка нет имён локальных: их материализует сборка со
    /// сведениями об отладке, и без неё показывать нечего — номер слота
    /// без имени отладчику бесполезен.
    fn locals(&mut self, index: usize) -> Vec<(String, BslValue)>;

    /// Вычисляет выражение В ВЫБРАННОМ кадре.
    ///
    /// Кадр — любой из стека, не обязательно верхний: смотреть переменную
    /// вызывающего, стоя во вложенном вызове, — обычное дело отладки.
    /// Обычный `Вычислить` так не умеет: он исполняется в кадре, где сам
    /// написан.
    ///
    /// Семантика выражений при этом ОДНА — тот же фронтенд, что у
    /// `Вычислить`, через тот же компилятор фрагментов. Второго
    /// вычислителя не заводится.
    ///
    /// # Errors
    ///
    /// Текст ошибки компиляции или исполнения фрагмента; а также отказ,
    /// если кадра с таким номером нет.
    fn evaluate(&mut self, index: usize, source: &str) -> Result<BslValue, String>;

    /// Строка исходника кадра `index`.
    ///
    /// `None` — у образа нет таблицы строк либо у кадра нет записи.
    /// Спрашивается на остановке, а не на каждой инструкции: стек кадров
    /// нужен редактору только когда он остановился.
    fn line_of(&mut self, index: usize) -> Option<u32>;
}

/// Доступ к остановленному прогону: чтение локальных и вычисление
/// выражения в выбранном кадре.
///
/// Живёт ровно на время вызова крючка и ничего не копирует, пока не
/// спросят: в обычном прогоне отладчик почти всегда просто идёт дальше.
pub(super) struct FrameValues<'a, 'h, 'd> {
    pub(super) task: &'a mut Task,
    pub(super) program: &'a Program,
    pub(super) linked: &'a LinkedComponents<'a>,
    pub(super) host: &'a mut HostIo<'h, 'd>,
    pub(super) module_state: &'a mut ModuleState,
    pub(super) async_state: &'a mut super::AsyncState,
    pub(super) catalog: Option<&'a super::CatalogContext<'a>>,
    pub(super) session_modules: &'a mut super::SessionModules,
    pub(super) runtime_shapes: &'a mut bsl_rt::RuntimeShapes,
}

impl FrameValues<'_, '_, '_> {
    fn frame_program(&self, index: usize) -> Option<&Program> {
        let frame = self.task.frames.get(index)?;
        if let Some(code) = &frame.code {
            Some(&code.program)
        } else if frame.module == super::ROOT_MODULE {
            Some(self.program)
        } else {
            self.catalog?.program(frame.module).ok()
        }
    }
}

impl DebugValues for FrameValues<'_, '_, '_> {
    fn locals(&mut self, index: usize) -> Vec<(String, BslValue)> {
        let Some(frame) = self.task.frames.get(index) else {
            return Vec::new();
        };
        let Some(chunk) = self
            .frame_program(index)
            .and_then(|p| p.chunks.get(frame.func_id))
        else {
            return Vec::new();
        };
        chunk
            .local_names
            .iter()
            .enumerate()
            .filter_map(|(slot, name)| {
                self.task
                    .stack
                    .get(frame.reg_index(slot as u8))
                    .map(|v| (name.clone(), v.clone()))
            })
            .collect()
    }

    fn line_of(&mut self, index: usize) -> Option<u32> {
        let frame = self.task.frames.get(index)?;
        // У ВЫЗЫВАЮЩЕГО кадра `pc` уже продвинут за `Call` — он указывает
        // на инструкцию, которая исполнится ПОСЛЕ возврата. Показывать её
        // нельзя: редактор подсветил бы следующую строку, а если вызов был
        // последней инструкцией чанка, индекс равен длине таблицы и
        // строки не нашлось бы вовсе. Приостановленному кадру нужна
        // строка самого вызова, то есть предыдущая инструкция.
        let top = self.task.frames.len().saturating_sub(1);
        // Ленивая инициализация не продвигает инструкцию владельца:
        // после тела модуля она должна повториться.
        let initializing = self.task.frames.get(index + 1).is_some_and(|next| {
            next.module != super::ROOT_MODULE && next.func_id == 0 && next.code.is_none()
        });
        let pc = if index == top || initializing {
            frame.pc
        } else {
            frame.pc.saturating_sub(1)
        };
        self.frame_program(index)?
            .lines
            .get(frame.func_id)
            .and_then(|rows| rows.get(pc))
            .copied()
    }

    fn evaluate(&mut self, index: usize, source: &str) -> Result<BslValue, String> {
        let Some(frame) = self.task.frames.get(index) else {
            return Err(format!("кадра {index} нет в стеке"));
        };
        let func_id = frame.func_id;
        let code = frame.code.clone();
        let program = if let Some(code) = &code {
            &code.program
        } else if frame.module == super::ROOT_MODULE {
            self.program
        } else {
            self.catalog
                .ok_or("кадр модуля без каталога")?
                .program(frame.module)
                .map_err(|error| error.to_string())?
        };
        let dynamic_linked;
        let linked = if let Some(code) = &code {
            dynamic_linked = LinkedComponents {
                registry: self.linked.registry,
                tables: code.tables.clone(),
            };
            &dynamic_linked
        } else if frame.module == super::ROOT_MODULE {
            self.linked
        } else {
            self.catalog
                .ok_or("кадр модуля без каталога")?
                .linked(frame.module)
                .map_err(|error| error.to_string())?
        };
        let chunk = program
            .chunks
            .get(func_id)
            .ok_or_else(|| format!("чанка {func_id} нет в программе"))?;
        // Область видимости — имена этого кадра. Наличие отладочной
        // информации определяется таблицей строк: у корректного кадра
        // без локальных таблица имён также пуста.
        if program.lines.is_empty() && !chunk.instrs.is_empty() {
            return Err(
                "у кадра нет таблицы имён: образ собран без сведений об отладке".to_string(),
            );
        }
        let scope_locals = chunk.local_names.clone();
        // Кадры и стек одалживаются РАЗДЕЛЬНО: `run_dynamic_snippet` берёт
        // стек изменяемым, а кадр — по ссылке, и через один `&mut task`
        // такого не выразить. Поля непересекающиеся, поэтому разбор
        // структуры это и решает — копировать кадр не нужно, тем более что
        // `Frame` и не `Clone`.
        let Task { frames, stack, .. } = &mut *self.task;
        let frame = &frames[index];
        let module_aliases: Vec<_> = frames
            .iter()
            .flat_map(|frame| frame.module_copybacks.iter().copied())
            .collect();
        run_dynamic_snippet(
            source,
            true,
            program,
            &scope_locals,
            func_id,
            stack,
            frame,
            &module_aliases,
            linked,
            self.host,
            self.module_state,
            self.async_state,
            self.program,
            self.linked,
            self.catalog,
            self.session_modules,
            self.runtime_shapes,
        )
        .map_err(|e| format!("{e}"))
    }
}

/// Крючок отладчика.
///
/// Зовётся ПЕРЕД каждой инструкцией — но из ВНЕШНЕГО цикла, никогда из
/// `step`. Диспетчер здесь живёт на грани uop-кэша, и проверка «а не пора
/// ли остановиться» внутри него обошлась бы дороже всего, что отладчик
/// даёт; что кода `step` это не коснулось, проверяется
/// `benchmarks/hot-code-diff.sh`, а не обещанием.
///
/// Крючок вправе БЛОКИРОВАТЬ сколько угодно: пока он не вернул
/// управление, прогон стоит — это и есть остановка на точке останова.
///
/// Принадлежит ПРОГОНУ, а не программе, — по той же причине, что и
/// компилятор фрагментов рядом: VM его зовёт, но не реализует.
pub trait DebugHook {
    /// Вызывается перед исполнением очередной инструкции.
    fn before_instruction(&mut self, at: &mut DebugPosition<'_>) -> DebugAction;
}
