//! Временное хранилище: сеансовая карта значений, mailbox для публикаций
//! заданий и staging их записей.
//!
//! Измеренный клиент-серверный контракт (`JOB.TEMP.*` плана фоновых
//! заданий): адрес — строка `e1cib/tempstorage/<uuid>?seanceId=<token>`;
//! чужой, закрытый или удалённый адрес читается как `Неопределено`,
//! удаляется как no-op, а запись отвечает ловимой ошибкой; задание пишет
//! по адресу непосредственного вызывателя ОТЛОЖЕННО (staging) и читает
//! его как `Неопределено`; публикация — атомарно на terminal transition
//! (commit после успеха и неперехваченной BSL-ошибки, rollback после
//! отмены и инфраструктурных сбоев).

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, Weak};

use crate::{
    BslValue, HostError, HostErrorCode, RtError, RtResult, RuntimeShapes, SerializedValueGraph,
};

/// Глобальный бюджет staging всех живых заданий одного runtime
/// (`max_live_staged_temp_bytes`). Счётчик атомарный: кредиты берут
/// worker-потоки без общего лока.
pub struct GlobalStagingBudget {
    limit: usize,
    used: AtomicUsize,
}

impl GlobalStagingBudget {
    #[must_use]
    pub fn new(limit: usize) -> Self {
        Self {
            limit,
            used: AtomicUsize::new(0),
        }
    }

    /// Атомарно резервирует из остатка как можно больше, но не более
    /// `cap`; возвращает зарезервированное. Ноль — остатка нет. Резерв,
    /// а не снимок: параллельные сериализаторы делят предел между собой,
    /// и сумма одновременных резервов никогда не превышает `limit`.
    fn take_up_to(&self, cap: usize) -> usize {
        let mut current = self.used.load(Ordering::Relaxed);
        loop {
            let grab = self.limit.saturating_sub(current).min(cap);
            if grab == 0 {
                return 0;
            }
            match self.used.compare_exchange_weak(
                current,
                current + grab,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return grab,
                Err(actual) => current = actual,
            }
        }
    }

    fn release(&self, bytes: usize) {
        self.used.fetch_sub(bytes, Ordering::Relaxed);
    }

    /// Занятые байты — для граничных тестов освобождения резервов.
    #[must_use]
    pub fn used(&self) -> usize {
        self.used.load(Ordering::Relaxed)
    }
}

/// Кредиты staging одного задания: per-job остаток
/// (`max_staged_temp_bytes_per_job`) плюс доля глобального бюджета.
/// Освобождение гарантировано на каждом terminal/abort-пути: явно — при
/// заборе write-set на публикацию, в остальных случаях (rollback, паника
/// worker, дроп сеанса) — `Drop`.
pub struct StagingBudget {
    per_job_left: usize,
    global: Arc<GlobalStagingBudget>,
    taken: usize,
}

impl StagingBudget {
    #[must_use]
    pub fn new(per_job: usize, global: Arc<GlobalStagingBudget>) -> Self {
        Self {
            per_job_left: per_job,
            global,
            taken: 0,
        }
    }

    fn release_taken(&mut self) {
        self.global.release(self.taken);
        self.taken = 0;
    }

    /// Резервирует под сериализацию ВЕСЬ доступный остаток кредитов —
    /// атомарно и ДО аллокации. Параллельные сериализаторы делят
    /// глобальный бюджет резервами, а не читают один и тот же снимок
    /// остатка, поэтому суммарная временная память сериализаций не
    /// превышает `max_live_staged_temp_bytes`. Неиспользованную часть
    /// возвращает `StagingLease::commit` либо его `Drop`.
    fn lease(&mut self) -> StagingLease<'_> {
        let reserved = self.global.take_up_to(self.per_job_left);
        StagingLease {
            budget: self,
            reserved,
        }
    }
}

impl Drop for StagingBudget {
    fn drop(&mut self) {
        self.release_taken();
    }
}

/// Лизинг кредитов под одну сериализацию: глобальная часть уже
/// зарезервирована, сериализатор ограничивается `max_bytes`. `commit`
/// фиксирует фактический размер снимка за бюджетом, `Drop` возвращает
/// неиспользованный резерв — и после `commit`, и при ошибке сериализации
/// или панике без него.
struct StagingLease<'a> {
    budget: &'a mut StagingBudget,
    reserved: usize,
}

impl StagingLease<'_> {
    /// Предел сериализации: per-job и глобальный остатки уже учтены.
    fn max_bytes(&self) -> usize {
        self.reserved
    }

    /// Фиксирует занятые снимком байты; остаток резерва вернёт `Drop`.
    fn commit(mut self, bytes: usize) {
        debug_assert!(bytes <= self.reserved, "снимок больше резерва лизинга");
        self.budget.per_job_left -= bytes;
        self.budget.taken += bytes;
        self.reserved -= bytes;
    }
}

impl Drop for StagingLease<'_> {
    fn drop(&mut self) {
        self.budget.global.release(self.reserved);
        self.reserved = 0;
    }
}

/// Приёмник публикаций заданий одного сеанса-вызывателя. `Send + Sync`:
/// задания пишут из workers, сеанс читает у себя.
#[derive(Default)]
pub struct TempMailbox {
    /// Публикации заданий: номер видимости + снимок. Номер выдаётся в
    /// момент ПУБЛИКАЦИИ — с этого момента запись новее локальных,
    /// сделанных раньше.
    committed: Mutex<HashMap<[u8; 16], (u64, SerializedValueGraph)>>,
    /// Общий порядок видимости записей адресов этого сеанса: локальные
    /// записи и публикации заданий нумеруются одним счётчиком, и по
    /// адресу побеждает новейшая.
    sequence: std::sync::atomic::AtomicU64,
}

impl TempMailbox {
    fn next_sequence(&self) -> u64 {
        self.sequence
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }
}

/// Реестр mailbox'ов всех живых сеансов одного движка: token -> Weak.
/// Слабая ссылка: drop сеанса закрывает token, а поздняя публикация
/// задания в закрытый сеанс не оживляет его.
#[derive(Default)]
pub struct TempStorageHub {
    mailboxes: Mutex<HashMap<[u8; 16], Weak<TempMailbox>>>,
}

impl TempStorageHub {
    pub fn register(&self, token: [u8; 16], mailbox: &Arc<TempMailbox>) {
        let mut mailboxes = self
            .mailboxes
            .lock()
            .expect("реестр mailbox без отравления");
        // Записи закрытых сеансов чистятся попутно: реестр живёт столько
        // же, сколько движок, и без уборки рос бы с каждым сеансом.
        mailboxes.retain(|_, mailbox| mailbox.strong_count() > 0);
        mailboxes.insert(token, Arc::downgrade(mailbox));
    }

    /// Живой mailbox сеанса, если тот ещё не закрыт.
    #[must_use]
    pub fn mailbox(&self, token: [u8; 16]) -> Option<Arc<TempMailbox>> {
        self.mailboxes
            .lock()
            .expect("реестр mailbox без отравления")
            .get(&token)
            .and_then(Weak::upgrade)
    }

    /// Публикует write-set задания в mailbox вызывателя: последний
    /// записавший по адресу выигрывает. `false` — сеанс вызывателя уже
    /// закрыт и публикация не состоялась.
    #[must_use]
    pub fn commit(&self, caller: [u8; 16], writes: Vec<([u8; 16], SerializedValueGraph)>) -> bool {
        let Some(mailbox) = self.mailbox(caller) else {
            return false;
        };
        let mut committed = mailbox.committed.lock().expect("mailbox без отравления");
        for (id, graph) in writes {
            let sequence = mailbox.next_sequence();
            committed.insert(id, (sequence, graph));
        }
        true
    }
}

/// Отложенная запись задания по адресу вызывателя.
pub struct StagedWrite {
    pub id: [u8; 16],
    pub graph: SerializedValueGraph,
}

/// Сеансовое временное хранилище: живёт в `HostEnv` одного `State`.
pub struct TempStorageSession {
    /// Token сеанса — вторая половина адреса.
    token: [u8; 16],
    /// Локальные значения с номером видимости: `Rc`-граф без
    /// сериализации, алиасы сохраняются.
    local: HashMap<[u8; 16], (u64, BslValue)>,
    /// Приёмник публикаций заданий ЭТОГО сеанса.
    mailbox: Arc<TempMailbox>,
    /// Token сеанса-вызывателя — только у сеанса задания.
    caller: Option<[u8; 16]>,
    /// Staging записей по адресам вызывателя — публикуется на terminal.
    staged: Vec<StagedWrite>,
    /// Кредиты staging (только у сеанса задания): взводятся до записи,
    /// освобождаются при заборе write-set либо `Drop`.
    staging_budget: Option<StagingBudget>,
    /// Живые адреса, ВЫДАННЫЕ этим сеансом. По синтакс-помощнику запись
    /// по адресу «должна быть получена ранее с помощью данного метода», а
    /// запись по уже удалённому адресу — исключение (подтверждено замером
    /// `Q78.OWN.DELETED.WRITE.ERROR`).
    issued: std::collections::HashSet<[u8; 16]>,
    /// Источник UUID адресов.
    random: crate::RandomHandle,
}

/// Разобранный адрес временного хранилища.
struct TempAddress {
    id: [u8; 16],
    token: [u8; 16],
}

const ADDRESS_PREFIX: &str = "e1cib/tempstorage/";
const ADDRESS_SEANCE: &str = "?seanceId=";

fn format_token(token: &[u8; 16]) -> String {
    crate::uuid::format(token)
}

fn parse_address(text: &str) -> Option<TempAddress> {
    let rest = text.strip_prefix(ADDRESS_PREFIX)?;
    let (uuid, token) = rest.split_once(ADDRESS_SEANCE)?;
    Some(TempAddress {
        id: crate::uuid::parse(uuid).ok()?,
        token: crate::uuid::parse(token).ok()?,
    })
}

impl TempStorageSession {
    #[must_use]
    pub fn new(token: [u8; 16], random: crate::RandomHandle) -> Self {
        Self {
            token,
            local: HashMap::new(),
            mailbox: Arc::new(TempMailbox::default()),
            caller: None,
            staged: Vec::new(),
            issued: std::collections::HashSet::new(),
            staging_budget: None,
            random,
        }
    }

    /// Сеанс задания: свой token, token непосредственного вызывателя и
    /// кредиты staging.
    #[must_use]
    pub fn for_job(
        token: [u8; 16],
        caller: [u8; 16],
        random: crate::RandomHandle,
        staging_budget: StagingBudget,
    ) -> Self {
        let mut session = Self::new(token, random);
        session.caller = Some(caller);
        session.staging_budget = Some(staging_budget);
        session
    }

    #[must_use]
    pub fn token(&self) -> [u8; 16] {
        self.token
    }

    #[must_use]
    pub fn mailbox(&self) -> &Arc<TempMailbox> {
        &self.mailbox
    }

    /// Token сеанса-вызывателя, если это сеанс задания.
    #[must_use]
    pub fn caller(&self) -> Option<[u8; 16]> {
        self.caller
    }

    /// Забирает staging для публикации на terminal transition. Кредиты
    /// staging освобождаются здесь: write-set покидает сеанс, а публикация
    /// копирует его в память сеанса-получателя.
    pub fn take_staged(&mut self) -> Vec<([u8; 16], SerializedValueGraph)> {
        if let Some(budget) = &mut self.staging_budget {
            budget.release_taken();
        }
        std::mem::take(&mut self.staged)
            .into_iter()
            .map(|write| (write.id, write.graph))
            .collect()
    }

    /// `ПоместитьВоВременноеХранилище`: запись по своему адресу — в
    /// локальную карту; по адресу вызывателя (в сеансе задания) — в
    /// staging; чужой адрес — ловимая ошибка, значения владельца целы.
    /// Без адреса создаётся новый адрес этого сеанса.
    pub fn put(
        &mut self,
        value: &BslValue,
        address: Option<&str>,
        rt: &RuntimeShapes,
    ) -> RtResult<String> {
        let target = match address {
            None | Some("") => TempAddress {
                id: {
                    let mut bytes = [0u8; 16];
                    self.random.fill(&mut bytes);
                    crate::uuid::v4_from_bytes(bytes)
                },
                token: self.token,
            },
            Some(text) => parse_address(text).ok_or_else(|| {
                HostError::new(
                    HostErrorCode::InvalidTemporaryStorageAddress,
                    format!("«{text}» не адрес временного хранилища"),
                )
                .raise()
            })?,
        };
        if target.token == self.token {
            if address.is_some() && !self.issued.contains(&target.id) {
                // ИЗМЕРЕНО (`Q78.OWN.DELETED.WRITE.ERROR`) и подтверждено
                // синтакс-помощником: запись по удалённому (или никогда не
                // выдававшемуся) адресу — исключение, значения целы.
                return Err(HostError::new(
                    HostErrorCode::InvalidTemporaryStorageAddress,
                    "адрес временного хранилища удалён или не выдавался",
                )
                .raise());
            }
            self.issued.insert(target.id);
            let sequence = self.mailbox.next_sequence();
            self.local.insert(target.id, (sequence, value.clone()));
        } else if self.caller == Some(target.token) {
            // Запись по адресу вызывателя публикуется НА TERMINAL, а не
            // сразу; до него вызыватель видит прежнее значение. Сама
            // запись сериализуется сейчас: задание продолжит менять свой
            // Rc-граф, а снимок уже неизменен. Кредиты РЕЗЕРВИРУЮТСЯ
            // лизингом ДО сериализации: параллельные сериализаторы делят
            // глобальный бюджет резервами и не собирают графы до одного
            // и того же остатка; неиспользованная часть возвращается
            // сразу после снимка, а отказ — ловимая `ResourceLimit`, не
            // меняющая накопленный write-set.
            let graph = match &mut self.staging_budget {
                Some(budget) => {
                    let lease = budget.lease();
                    let limits = crate::GraphLimits {
                        max_bytes: lease.max_bytes(),
                    };
                    let graph =
                        SerializedValueGraph::capture(std::slice::from_ref(value), rt, &limits)
                            .map_err(|error| {
                                match error {
                        RtError::ResourceLimit(_) => HostError::new(
                            HostErrorCode::ResourceLimit,
                            "запись не помещается в staging-бюджет временного хранилища задания",
                        )
                        .raise(),
                        other => other,
                    }
                            })?;
                    lease.commit(graph.byte_size());
                    graph
                }
                None => SerializedValueGraph::capture(
                    std::slice::from_ref(value),
                    rt,
                    &crate::GraphLimits::default(),
                )?,
            };
            self.staged.push(StagedWrite {
                id: target.id,
                graph,
            });
        } else {
            // ИЗМЕРЕНО (JOB.TEMP.FOREIGN): запись по чужому адресу — ошибка
            // с пустым описанием у платформы; свой текст информативнее, а
            // точный пустой текст намеренно не копируется.
            return Err(HostError::new(
                HostErrorCode::InvalidTemporaryStorageAddress,
                "адрес принадлежит другому сеансу",
            )
            .raise());
        }
        Ok(format!(
            "{ADDRESS_PREFIX}{}{ADDRESS_SEANCE}{}",
            crate::uuid::format(&target.id),
            format_token(&target.token)
        ))
    }

    /// `ПолучитьИзВременногоХранилища`: чужой, закрытый или удалённый
    /// адрес — `Неопределено` (ИЗМЕРЕНО, `JOB.TEMP.FOREIGN`); staged-запись
    /// по адресу вызывателя тоже `Неопределено` до публикации.
    // НЕ ИЗМЕРЕНО(JOB.TEMP.READ_YOUR_WRITES): что job платформы читает по
    // caller-адресу после собственной staging-записи, снято только с
    // документации («данные... сразу после помещения недоступны в фоновом
    // сеансе» — синтакс-помощник 8.3.27), платформенный замер не
    // выполнялся; наша семантика совпадает с документированной.
    pub fn get(&self, address: &str, rt: &mut RuntimeShapes) -> RtResult<BslValue> {
        let Some(target) = parse_address(address) else {
            return Ok(BslValue::Undefined);
        };
        if target.token != self.token {
            return Ok(BslValue::Undefined);
        }
        let local = self.local.get(&target.id);
        let committed = self
            .mailbox
            .committed
            .lock()
            .expect("mailbox без отравления");
        // По адресу побеждает новейшая запись: локальная либо публикация
        // задания — общий счётчик видимости решает.
        match (local, committed.get(&target.id)) {
            (Some((local_seq, value)), Some((published_seq, graph))) => {
                if published_seq > local_seq {
                    let mut values = graph.materialize(rt)?;
                    Ok(values.pop().unwrap_or(BslValue::Undefined))
                } else {
                    Ok(value.clone())
                }
            }
            (Some((_, value)), None) => Ok(value.clone()),
            (None, Some((_, graph))) => {
                let mut values = graph.materialize(rt)?;
                Ok(values.pop().unwrap_or(BslValue::Undefined))
            }
            (None, None) => Ok(BslValue::Undefined),
        }
    }

    /// `УдалитьИзВременногоХранилища`: успешный no-op для любого не
    /// своего адреса (ИЗМЕРЕНО, `JOB.TEMP.FOREIGN`).
    // НЕ ИЗМЕРЕНО(JOB.TEMP.LIFETIME): повторное использование удалённого
    // адреса, срок жизни адресов UUID-владельца и новый адрес задания
    // после закрытия его сеанса не замерены; выбрано «запись по
    // удалённому адресу — ошибка» (по `Q78.OWN.DELETED.WRITE.ERROR`),
    // адреса живут до конца сеанса.
    // НЕ ИЗМЕРЕНО(JOB.TEMP.STAGED_DELETE): публикуется ли удаление
    // caller-адреса из сеанса задания и по каким terminal-правилам, не
    // замерено; выбран локальный no-op без staging удаления — данные
    // вызывателя не меняются.
    pub fn delete(&mut self, address: &str) {
        let Some(target) = parse_address(address) else {
            return;
        };
        if target.token != self.token {
            return;
        }
        self.local.remove(&target.id);
        self.issued.remove(&target.id);
        self.mailbox
            .committed
            .lock()
            .expect("mailbox без отравления")
            .remove(&target.id);
    }
}

// --- Глобальные функции ядра -------------------------------------------

/// `ПоместитьВоВременноеХранилище(Значение[, Адрес])`.
pub(crate) fn put_to_temp_storage(
    ctx: &mut crate::CallContext<'_>,
    args: &[BslValue],
) -> RtResult<BslValue> {
    let session = std::rc::Rc::clone(ctx.temp_storage()?);
    let address = match args.get(1) {
        None | Some(BslValue::Undefined) => None,
        Some(value) => Some(value.as_str("ПоместитьВоВременноеХранилище")?.to_string()),
    };
    let value = args.first().cloned().unwrap_or(BslValue::Undefined);
    let address = session
        .borrow_mut()
        .put(&value, address.as_deref(), ctx.runtime_shapes())?;
    Ok(BslValue::Str(crate::BslString::from_str(&address)))
}

/// `ПолучитьИзВременногоХранилища(Адрес)`.
pub(crate) fn get_from_temp_storage(
    ctx: &mut crate::CallContext<'_>,
    args: &[BslValue],
) -> RtResult<BslValue> {
    let session = std::rc::Rc::clone(ctx.temp_storage()?);
    let address = args
        .first()
        .ok_or(RtError::TypeError {
            expected: "Строка",
            op: "ПолучитьИзВременногоХранилища",
        })?
        .as_str("ПолучитьИзВременногоХранилища")?
        .to_string();
    let session = session.borrow();
    session.get(&address, ctx.runtime_shapes())
}

/// `УдалитьИзВременногоХранилища(Адрес)`.
pub(crate) fn delete_from_temp_storage(
    ctx: &mut crate::CallContext<'_>,
    args: &[BslValue],
) -> RtResult<BslValue> {
    let session = std::rc::Rc::clone(ctx.temp_storage()?);
    let address = args
        .first()
        .ok_or(RtError::TypeError {
            expected: "Строка",
            op: "УдалитьИзВременногоХранилища",
        })?
        .as_str("УдалитьИзВременногоХранилища")?
        .to_string();
    session.borrow_mut().delete(&address);
    Ok(BslValue::Undefined)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Кредиты staging резервируются лизингом ДО сериализации и
    /// освобождаются на всех путях: `commit` возвращает неиспользованную
    /// часть, `Drop` лизинга — резерв целиком, `Drop` бюджета — занятое.
    #[test]
    fn staging_credits_are_released_on_commit_and_on_drop() {
        let global = Arc::new(GlobalStagingBudget::new(1_000));
        {
            let mut budget = StagingBudget::new(800, Arc::clone(&global));
            let lease = budget.lease();
            assert_eq!(lease.max_bytes(), 800, "резерв ограничен per-job");
            assert_eq!(global.used(), 800, "резерв взят из глобального счётчика");
            lease.commit(500);
            assert_eq!(global.used(), 500, "commit вернул неиспользованное");
            let lease = budget.lease();
            assert_eq!(lease.max_bytes(), 300, "per-job остаток после commit");
            drop(lease);
            assert_eq!(global.used(), 500, "drop без commit вернул резерв");
            // Drop бюджета без забора — rollback: счётчик чист.
        }
        assert_eq!(global.used(), 0);

        // Глобальный предел жёстче per-job: пока первый резерв держится,
        // второй сериализатор получает ноль, а после commit — остаток.
        let mut first = StagingBudget::new(1_000, Arc::clone(&global));
        let mut second = StagingBudget::new(1_000, Arc::clone(&global));
        let first_lease = first.lease();
        assert_eq!(first_lease.max_bytes(), 1_000);
        let second_lease = second.lease();
        assert_eq!(
            second_lease.max_bytes(),
            0,
            "глобальный остаток занят резервом"
        );
        drop(second_lease);
        first_lease.commit(700);
        let second_lease = second.lease();
        assert_eq!(second_lease.max_bytes(), 300);
        second_lease.commit(300);
        assert_eq!(global.used(), 1_000);
    }

    /// Два параллельных сериализатора ДЕЛЯТ глобальный бюджет резервами:
    /// каждый ограничен своим лизингом, поэтому суммарная временная
    /// память сериализаций не превышает предел (а не workers × предел),
    /// и из двух записей, не помещающихся вместе, проходит ровно одна.
    #[test]
    fn two_serializers_reserve_the_global_budget_before_capture() {
        let limit = 64 << 10;
        let global = Arc::new(GlobalStagingBudget::new(limit));
        let barrier = Arc::new(std::sync::Barrier::new(2));
        let caller_token = [2u8; 16];
        let caller_address = format!(
            "{ADDRESS_PREFIX}{}{ADDRESS_SEANCE}{}",
            crate::uuid::format(&[9; 16]),
            crate::uuid::format(&caller_token)
        );
        let mut workers = Vec::new();
        for index in 0..2u8 {
            let global = Arc::clone(&global);
            let barrier = Arc::clone(&barrier);
            let caller_address = caller_address.clone();
            workers.push(std::thread::spawn(move || {
                // Сеанс не `Send` — каждый поток строит свой.
                let mut session = TempStorageSession::for_job(
                    [index + 10; 16],
                    caller_token,
                    crate::HostEnv::process().random(),
                    StagingBudget::new(limit, Arc::clone(&global)),
                );
                let rt = crate::RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
                // 48 КиБ на запись: две вместе в 64 КиБ не помещаются.
                let value = BslValue::Str(crate::BslString::from_str(&"y".repeat(48 << 10)));
                barrier.wait();
                let outcome = session.put(&value, Some(&caller_address), &rt);
                assert!(
                    global.used() <= limit,
                    "сумма резервов и кредитов не превышает предел"
                );
                if outcome.is_ok() {
                    assert!(global.used() > 0, "успешный staging держит кредит");
                }
                // Сеанс не `Send` — дропается здесь же, возвращая кредит.
                outcome.is_ok()
            }));
        }
        let successes = workers
            .into_iter()
            .map(|worker| worker.join().expect("поток сериализатора"))
            .filter(|ok| *ok)
            .count();
        assert_eq!(
            successes, 1,
            "две записи по 48 КиБ не делят 64 КиБ: проходит ровно одна"
        );
        assert_eq!(global.used(), 0, "drop сеансов вернул кредиты");
    }

    /// Сериализация staging-записи ограничена ДОСТУПНЫМ остатком
    /// кредитов ДО крупной аллокации: большой граф при маленьком лимите
    /// отвергается ловимой `ResourceLimit`, не меняет накопленный
    /// write-set и не занимает кредиты.
    #[test]
    fn an_over_budget_staged_write_is_refused_before_capture() {
        let global = Arc::new(GlobalStagingBudget::new(10_000));
        let mut session = TempStorageSession::for_job(
            [1; 16],
            [2; 16],
            crate::HostEnv::process().random(),
            StagingBudget::new(1_024, Arc::clone(&global)),
        );
        let rt = crate::RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let caller_address = format!(
            "{ADDRESS_PREFIX}{}{ADDRESS_SEANCE}{}",
            crate::uuid::format(&[9; 16]),
            crate::uuid::format(&[2; 16])
        );
        // Порядок значений — сам замер: за большой строкой лежит вовсе
        // не сериализуемое значение. Ограниченный бюджетом обходчик
        // останавливается на строке и до него не доходит; сериализация
        // «сначала целиком, потом проверка» дошла бы и ответила ошибкой
        // непереносимого типа, а не ловимой `ResourceLimit`.
        let huge = BslValue::new_array(vec![
            BslValue::Str(crate::BslString::from_str(&"х".repeat(100_000))),
            BslValue::new_object(crate::user_message::UserMessageObject::with_text("хвост")),
        ]);
        let error = session
            .put(&huge, Some(&caller_address), &rt)
            .expect_err("граф больше per-job бюджета");
        match error {
            RtError::Host(host) => assert_eq!(host.code, HostErrorCode::ResourceLimit),
            other => panic!("не тот класс ошибки: {other:?}"),
        }
        assert_eq!(global.used(), 0, "отказ не занимает кредиты");
        assert!(
            session.take_staged().is_empty(),
            "отказ не меняет накопленный write-set"
        );

        // Запись в пределах бюджета после отказа проходит как обычно.
        let small = BslValue::Str(crate::BslString::from_str("помещается"));
        session
            .put(&small, Some(&caller_address), &rt)
            .expect("малый граф в бюджете");
        assert_eq!(session.take_staged().len(), 1);
    }

    /// Забор write-set сеансом задания освобождает кредиты сразу: они
    /// покрывают только период staging.
    #[test]
    fn a_job_session_releases_credits_when_staged_writes_are_taken() {
        let global = Arc::new(GlobalStagingBudget::new(10_000));
        let mut session = TempStorageSession::for_job(
            [1; 16],
            [2; 16],
            crate::HostEnv::process().random(),
            StagingBudget::new(10_000, Arc::clone(&global)),
        );
        let rt = crate::RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let caller_address = format!(
            "{ADDRESS_PREFIX}{}{ADDRESS_SEANCE}{}",
            crate::uuid::format(&[9; 16]),
            crate::uuid::format(&[2; 16])
        );
        let value = BslValue::Str(crate::BslString::from_str("значение"));
        session
            .put(&value, Some(&caller_address), &rt)
            .expect("staging-запись");
        assert!(global.used() > 0, "кредит взят под staging");
        let writes = session.take_staged();
        assert_eq!(writes.len(), 1);
        assert_eq!(global.used(), 0, "забор write-set вернул кредит");
    }
}
