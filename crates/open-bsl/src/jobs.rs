//! Нативный runtime фоновых заданий: конфигурация, реестр, admission.
//!
//! Архитектурные инварианты (план фоновых заданий,
//! `docs/archive/plans/background-jobs.md`):
//! реестр хранит только владеющие `Send`-DTO и никогда не вызывает внешний
//! код под своим локом; мьютекс — синхронный `std::sync::Mutex` с короткими
//! секциями и не пересекает `await`; занятый пул означает FIFO, а не
//! ошибку; первый terminal transition выигрывает ровно один раз.

mod prepare;
mod registry;

#[cfg(test)]
pub(crate) use prepare::resolve_target;
pub(crate) use prepare::runtime_for_engine;
use prepare::{TargetTable, WorkerRecipe};
mod runtime;
mod service;
mod worker;

use worker::{take_thread_residents, worker_main};

use runtime::JobRuntimeShared;
pub(crate) use runtime::random_uuid;
pub use runtime::{
    BackgroundJobConfig, BackgroundStateFactory, HostProfileId, JobIdSource, JobRuntime,
    JobTimeSource, ShutdownReport,
};

use service::JobMessageRoute;
pub(crate) use service::{EngineJobService, WorkerJobService};

// --- Пул workers -------------------------------------------------------

#[cfg(test)]
mod test_support {
    use bsl_rt::{GraphLimits, RuntimeShapes, SerializedValueGraph};
    use std::sync::Arc;

    pub(super) fn engine() -> crate::Engine {
        crate::Engine::builder()
            .common_module(
                "Служебный",
                "Перем Счётчик Экспорт;\n\
                 Функция Сложить(Знач а, Знач б) Экспорт\n\
                     Возврат а + б;\n\
                 КонецФункции\n\
                 Процедура Упасть() Экспорт\n\
                     ВызватьИсключение \"задание падает\";\n\
                 КонецПроцедуры\n\
                 Процедура Вечно() Экспорт\n\
                     Пока Истина Цикл\n\
                     КонецЦикла;\n\
                 КонецПроцедуры\n\
                 Счётчик = 0;",
            )
            .build()
            .expect("движок с каталогом")
    }

    pub(super) fn params(values: &[bsl_rt::BslValue]) -> Arc<SerializedValueGraph> {
        let rt = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        Arc::new(
            SerializedValueGraph::capture(values, &rt, &GraphLimits::default())
                .expect("снимок параметров"),
        )
    }

    pub(super) fn number(value: i64) -> bsl_rt::BslValue {
        bsl_rt::BslValue::number_from_i64(value)
    }
}
