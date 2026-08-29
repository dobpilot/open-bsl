//! Каталог конфигурации: общие модули под детерминированными номерами,
//! transient entry поверх них и типизированная таблица связей.
//!
//! Каталог не сводит модули в один `Program` намеренно: фоновому заданию
//! нужен отдельный `ModuleInstance` на каждый сеанс, а слияние чанков
//! перенумеровало бы функции, имена и формы и сделало бы изоляцию модулей
//! невозможной (см. `docs/archive/plans/background-jobs.md`, раздел «Каталог общих
//! модулей и bytecode»).

use crate::chunk::Program;

/// Позиция общего модуля в каталоге конфигурации. Детерминирована местом в
/// manifest: один и тот же образ даёт одни и те же номера, поэтому номер —
/// часть переносимого формата, а не процессное свойство.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModuleId(u32);

impl ModuleId {
    #[must_use]
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    /// Позиция в `ConfigurationProgram::modules`.
    #[must_use]
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

/// Идентификатор transient entry-модуля. Не входит в каталог: entry живёт
/// столько же, сколько его `Module`, и другим модулям не адресуем.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntryId(u64);

impl EntryId {
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Номер записи в таблице связей одного модуля (`Program::links`).
/// Ширина `u16` ограничивает модуль 65 535 импортированными символами —
/// это осознанная часть ABI, а не случайность представления.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LinkSlot(u16);

impl LinkSlot {
    #[must_use]
    pub const fn new(slot: u16) -> Self {
        Self(slot)
    }

    #[must_use]
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

/// Запись таблицы связей: символ чужого модуля, разрешённый при сборке
/// конфигурации. Вид символа известен заранее, поэтому `CallImported`
/// не может исполнить переменную, а `GetImportedVar` — функцию: смешение
/// отвергается проверкой образа, а не обнаруживается в рантайме.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkEntry {
    /// Экспортная функция или процедура чужого модуля: `func` — индекс
    /// чанка в целевой программе (как у `Instr::Call`, ноль запрещён).
    Function { module: ModuleId, func: u16 },
    /// Экспортная переменная чужого модуля: `slot` — номер в его
    /// `module_vars`.
    Variable { module: ModuleId, slot: u16 },
}

/// Общий модуль каталога: стабильное имя (корневой псевдоним `//@используй`
/// либо имя, заданное `EngineBuilder::common_module`) и его программа.
#[derive(Debug, Clone)]
pub struct ModuleProgram {
    pub name: String,
    pub program: Program,
}

/// Неизменяемый каталог общих модулей одного Engine. `ModuleId` — позиция
/// в `modules`; порядок фиксируется при сборке и входит в образ.
#[derive(Debug, Clone, Default)]
pub struct ConfigurationProgram {
    pub modules: Vec<ModuleProgram>,
}

impl ConfigurationProgram {
    #[must_use]
    pub fn module(&self, id: ModuleId) -> Option<&ModuleProgram> {
        self.modules.get(id.index())
    }

    /// Поиск модуля по имени без учёта регистра — тем же правилом свёртки,
    /// что и остальные имена BSL.
    #[must_use]
    pub fn find(&self, name: &str) -> Option<(ModuleId, &ModuleProgram)> {
        self.modules
            .iter()
            .enumerate()
            .find(|(_, module)| bsl_rt::folded_eq(&module.name, name))
            .map(|(i, module)| (ModuleId::new(i as u32), module))
    }
}

/// Transient main-модуль поверх каталога: результат `Engine::compile_entry`.
/// Каталогу не принадлежит и целью фонового задания не является.
#[derive(Debug, Clone)]
pub struct EntryProgram {
    pub id: EntryId,
    pub program: Program,
}

/// Переносимый образ байт-кода формата 0.4: либо одиночная программа, как
/// раньше, либо каталог конфигурации с необязательным entry. Worker
/// принимает каталог без entry; `--run-bytecode` требует entry.
#[derive(Debug, Clone)]
pub enum BytecodeImage {
    Program(Program),
    Configuration {
        catalog: ConfigurationProgram,
        entry: Option<EntryProgram>,
    },
}
