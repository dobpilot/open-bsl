//! Компилятор `Выполнить`/`Вычислить` на стороне хоста.
//!
//! VM динамический код только ИСПОЛНЯЕТ: встретив `RunDynamic`, она
//! описывает область видимости и отдаёт описание сюда
//! (`bsl_bytecode::DynamicCompiler`), а обратно получает готовый
//! `DynamicUnit`. Весь фронтенд — лексика, разбор, резолвинг, кодоген, —
//! реестр компонентов, символы условной компиляции и кэш фрагментов живут
//! здесь, потому что именно движок и сессия знают о них всё.
//!
//! Здесь же раздаются номера областей. Идентичность прогона — хостова: VM
//! получает её в `DynamicUnit::scope` и возвращает обратно в
//! `DynamicScope`, но сама не придумывает, и в байт-коде счётчиков нет.

use std::collections::HashMap;
use std::rc::Rc;

use bsl_bytecode::{DynamicCompiler, DynamicKind, DynamicRequest, DynamicScope, DynamicUnit};

use crate::engine::Engine;

/// Кэшируемая компиляция динамических фрагментов для одной сессии.
///
/// Владеет [`State`](crate::State), поэтому кэш изолирован между сессиями
/// так же, как потоки вывода и часы: два `State` одного `Engine` не видят
/// фрагментов друг друга.
///
/// Кэш переживает отдельный запуск, и это осмысленно только потому, что
/// ключ УСТОЙЧИВ: повторный `State::run` того же модуля даёт те же ключи и
/// пользуется уже скомпилированным. Растёт кэш поэтому не с числом
/// запусков, а с числом РАЗНЫХ текстов фрагментов в разных областях —
/// то есть у обычного модуля упирается в конечное число мест
/// `Выполнить`/`Вычислить` в его тексте. Исключение — код, который
/// собирает текст фрагмента на ходу (`Выполнить("х = " + н)`): там
/// различных текстов столько же, сколько итераций, и кэшировать их
/// бессмысленно в любой схеме.
pub struct DynamicCode {
    engine: Engine,
    /// Модуль, который сейчас исполняется, — первая часть ключа. Без него
    /// нулевой чанк одного модуля столкнулся бы с нулевым чанком другого,
    /// исполненного той же сессией.
    module: u64,
    /// Ключ — модуль, область внутри него, вид операции и текст. Одного
    /// текста мало: один и тот же исходник в разных областях резолвится в
    /// РАЗНЫЕ номера слотов, а `Вычислить` компилируется не так, как
    /// `Выполнить`.
    cache: HashMap<(u64, DynamicScope, DynamicKind, String), Rc<DynamicUnit>>,
    /// Сколько номеров областей уже роздано. Номер получает КАЖДЫЙ
    /// выпущенный фрагмент, и вместе с ним он ложится в кэш — поэтому
    /// повторное исполнение того же фрагмента возвращает тот же номер, и
    /// вложенный в него `Выполнить` попадает в кэш, а не компилируется
    /// заново.
    scopes: u64,
    /// Сколько раз фронтенд действительно отработал. Наблюдаемость кэша:
    /// проверять его тестом по времени было бы гаданием.
    compiles: usize,
}

impl DynamicCode {
    pub(crate) fn new(engine: Engine) -> Self {
        Self {
            engine,
            module: 0,
            cache: HashMap::new(),
            scopes: DynamicScope::ROOT,
            compiles: 0,
        }
    }

    /// Говорит, чей модуль сейчас пойдёт в VM: дальше это первая часть
    /// каждого ключа кэша.
    pub(crate) fn bind_module(&mut self, module: u64) {
        self.module = module;
    }

    /// Сколько фрагментов лежит в кэше.
    #[cfg(test)]
    pub(crate) fn cached(&self) -> usize {
        self.cache.len()
    }

    /// Сколько раз фронтенд отработал за всё время жизни.
    #[cfg(test)]
    pub(crate) fn compiles(&self) -> usize {
        self.compiles
    }
}

impl DynamicCompiler for DynamicCode {
    fn compile(&mut self, request: &DynamicRequest<'_>) -> Result<Rc<DynamicUnit>, String> {
        let key = (
            self.module,
            request.scope,
            request.kind,
            request.source.to_string(),
        );
        if let Some(hit) = self.cache.get(&key) {
            return Ok(Rc::clone(hit));
        }
        // `checked_add`, а не `+=`: номер области монотонно растёт за всё
        // время жизни сессии, и переполнение `u64` — пусть и недостижимое
        // на практике — обязано быть явной паникой, а не тихим заворотом в
        // ноль (то есть в [`DynamicScope::ROOT`], чужой ключ кэша).
        self.scopes = self
            .scopes
            .checked_add(1)
            .expect("переполнение счётчика областей динамического кода");
        self.compiles += 1;
        let scope = std::num::NonZeroU64::new(self.scopes)
            .expect("счётчик увеличен перед этим вызовом, ноль недостижим");
        let unit = Rc::new(bsl_compiler::compile_dynamic_snippet(
            request,
            Some(self.engine.registry()),
            &self.engine.preproc_symbols(),
            scope,
        )?);
        self.cache.insert(key, Rc::clone(&unit));
        Ok(unit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request<'a>(
        source: &'a str,
        scope: DynamicScope,
        requirements: &'a [bsl_bytecode::LibraryRequirement],
    ) -> DynamicRequest<'a> {
        DynamicRequest {
            source,
            kind: DynamicKind::Eval,
            scope,
            locals: &[],
            module_vars: &[],
            functions: &[],
            names: &[],
            requirements,
        }
    }

    fn core() -> Vec<bsl_bytecode::LibraryRequirement> {
        vec![bsl_bytecode::LibraryRequirement::bsl_rt()]
    }

    fn root() -> DynamicScope {
        DynamicScope {
            program: DynamicScope::ROOT,
            chunk: 0,
        }
    }

    fn code() -> DynamicCode {
        DynamicCode::new(Engine::builder().build().expect("сборка движка"))
    }

    /// Один и тот же текст в одной области компилируется РОВНО ОДИН РАЗ:
    /// иначе `Выполнить` в цикле проходил бы весь фронтенд на каждой
    /// итерации. Проверяется тождеством `Rc`, а не временем.
    #[test]
    fn the_same_text_in_the_same_scope_is_compiled_once() {
        let mut dynamic = code();
        let first = dynamic.compile(&request("2 + 2", root(), &core())).unwrap();
        let second = dynamic.compile(&request("2 + 2", root(), &core())).unwrap();
        assert!(Rc::ptr_eq(&first, &second));
        assert_eq!(dynamic.compiles(), 1);
    }

    /// А в РАЗНЫХ областях — заново: там другие таблицы локальных, и общий
    /// чанк дал бы обращение по чужому номеру слота. Различаются области и
    /// по номеру чанка, и по тому, ЧЕЙ это чанк, — второе важно для
    /// вложенного `Выполнить`, у которого чанк тоже нулевой.
    #[test]
    fn the_same_text_in_another_scope_gets_its_own_chunk() {
        let mut dynamic = code();
        let first = dynamic.compile(&request("2 + 2", root(), &core())).unwrap();
        let other_chunk = dynamic
            .compile(&request(
                "2 + 2",
                DynamicScope {
                    program: DynamicScope::ROOT,
                    chunk: 1,
                },
                &core(),
            ))
            .unwrap();
        let inside_fragment = dynamic
            .compile(&request(
                "2 + 2",
                DynamicScope {
                    program: first.scope.get(),
                    chunk: 0,
                },
                &core(),
            ))
            .unwrap();
        assert!(!Rc::ptr_eq(&first, &other_chunk));
        assert!(!Rc::ptr_eq(&first, &inside_fragment));
    }

    /// И в разных МОДУЛЯХ одной сессии — тоже заново: нулевой чанк есть у
    /// каждого, а таблицы локальных у них свои.
    #[test]
    fn the_same_text_in_another_module_gets_its_own_chunk() {
        let mut dynamic = code();
        dynamic.bind_module(1);
        let first = dynamic.compile(&request("2 + 2", root(), &core())).unwrap();
        dynamic.bind_module(2);
        let second = dynamic.compile(&request("2 + 2", root(), &core())).unwrap();
        assert!(!Rc::ptr_eq(&first, &second));
    }

    /// Номер области у фрагмента СВОЙ и не совпадает с корневым, иначе
    /// вложенный `Выполнить` попал бы в ключ модуля.
    #[test]
    fn every_fragment_gets_a_scope_of_its_own() {
        let mut dynamic = code();
        let first = dynamic.compile(&request("2 + 2", root(), &core())).unwrap();
        let second = dynamic.compile(&request("3 + 3", root(), &core())).unwrap();
        // `scope` теперь `NonZeroU64` — совпасть с корнем (нулём) он не
        // может по типу; сравнение оставлено как явная документация цели.
        assert_ne!(first.scope.get(), DynamicScope::ROOT);
        assert_ne!(second.scope.get(), DynamicScope::ROOT);
        assert_ne!(first.scope, second.scope);
        // Повтор номера не тратит: он приезжает вместе с записью кэша.
        let again = dynamic.compile(&request("2 + 2", root(), &core())).unwrap();
        assert_eq!(again.scope, first.scope);
    }

    /// Кривой текст — ошибка компиляции, а не паника: её текст едет в VM,
    /// которая делает из него ловимую `RtError::DynamicError`.
    #[test]
    fn a_broken_fragment_reports_its_error_as_text() {
        let mut dynamic = code();
        let Err(error) = dynamic.compile(&request("это ((( не код", root(), &core()))
        else {
            panic!("кривой фрагмент обязан не скомпилироваться");
        };
        assert!(!error.is_empty());
    }
}
