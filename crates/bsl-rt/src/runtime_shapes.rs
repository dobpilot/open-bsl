use std::rc::Rc;

use std::sync::Arc;

use crate::component::{RuntimeRegistry, TypeCatalog};
use crate::interner::NameInterner;
use crate::shape::{Shape, ShapeTable};
use crate::types::{TypeId, TypeRef};

/// Контекст рантайм-мутации структур (`Вставить`/`Удалить`/`Свойство`):
/// имена и формы вместе, потому что превращение строкового ключа в
/// `NameId` (интернер) и переход между формами по этому `NameId` (таблица
/// форм) — одна операция с точки зрения вызывающего VM-кода, а не два
/// независимых состояния, которые можно рассинхронизировать.
///
/// Живёт на время одного `run_program`/`run_repl_chunk`/динамического
/// сниппета — затравлена уже готовыми компиляционными именами/формами ЭТОЙ
/// программы (`seeded`), а не общая на весь процесс: у каждого `Program`
/// свои `names`/`shapes`, и рантайм-расширения этой таблицы актуальны
/// только для объектов, живущих внутри одного и того же исполнения.
pub struct RuntimeShapes {
    pub names: NameInterner,
    pub shapes: ShapeTable,
    /// Типы объектов, объявленные компонентами ЭТОГО прогона
    /// (`LibraryDescriptor::types`). Обход — через [`Self::component_types`];
    /// поиск по имени идёт каталогом (`type_catalog`), а не этим вектором.
    /// Обе таблицы ЗАКРЫТЫ и ставятся ВМЕСТЕ в [`Self::seeded`]: публичное
    /// поле позволило бы рассогласовать список с каталогом присваиванием —
    /// ровно тот класс, который эта работа закрывает в других местах.
    component_types: Vec<&'static crate::TypeDescriptor>,
    /// Проверенный каталог «написание → тип» этого прогона.
    ///
    /// Указатель со счётчиком, а не ссылка: у `RuntimeShapes` нет параметра
    /// времени жизни, и заводить его ради каталога — миграция всех сигнатур,
    /// которые его принимают. Каталог неизменяем и живёт весь прогон.
    /// `Arc`, а не `Rc` (план называл `Rc`): тот же каталог держит
    /// `RuntimeRegistry`, а он входит в `EngineInner` за `Arc` фасада, то
    /// есть обязан оставаться `Send + Sync`; `Rc` это свойство снял бы.
    /// Каталог создаётся один раз и клонируется раз на прогон, так что
    /// атомарный счётчик здесь ничего не стоит.
    type_catalog: Arc<TypeCatalog>,
}

impl RuntimeShapes {
    /// Компонентные типы и каталог приходят ПРИ ПОСТРОЕНИИ из реестра и
    /// дальше не меняются. Публичного сеттера нет намеренно: компонент
    /// получает `&mut RuntimeShapes` через `CallContext::runtime_shapes()`,
    /// и метод установки позволил бы стороннему компоненту подменить каталог
    /// посреди прогона — то самое рассогласование во времени, которое эта
    /// работа убирает. `registry` — `None` там, где типов компонентов нет
    /// (тесты ядра, фрагмент без реестра).
    pub fn seeded(
        names: Vec<String>,
        shapes: Vec<Rc<Shape>>,
        registry: Option<&RuntimeRegistry>,
    ) -> Self {
        RuntimeShapes {
            names: NameInterner::from_existing(names),
            shapes: ShapeTable::from_existing(shapes),
            component_types: registry
                .map(|registry| registry.types().collect())
                .unwrap_or_default(),
            type_catalog: registry
                .map(RuntimeRegistry::type_catalog)
                .unwrap_or_default(),
        }
    }

    /// Объявленные компонентами типы — для тех, кому нужен список, а не
    /// поиск (пополнение `component_types` не через это API).
    pub fn component_types(&self) -> &[&'static crate::TypeDescriptor] {
        &self.component_types
    }

    /// Разрешение имени типа — ОДНО на оба вызывающих (`Тип("Имя")` и
    /// `Новый ОписаниеТипов("Имя")`). Возвращает `TypeRef`, а не дескриптор:
    /// `Тип("Строка")` обязан отвечать типом ядра. Приоритет — ядро раньше
    /// компонентов — записан ЗДЕСЬ, а не в двух местах по-разному, чем и
    /// снимается прежнее расхождение вызывающих в порядке опроса.
    pub fn resolve_type(&self, name: &str) -> Option<TypeRef> {
        TypeId::lookup(name)
            .map(TypeRef::Native)
            .or_else(|| self.type_catalog.resolve(name).map(TypeRef::Object))
    }
}
