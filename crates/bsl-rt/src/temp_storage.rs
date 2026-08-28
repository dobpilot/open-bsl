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

    /// Возвращает кредиты замещённой записи: место в staging она больше
    /// не занимает.
    fn refund(&mut self, bytes: usize) {
        debug_assert!(bytes <= self.taken, "возврат больше занятого");
        let bytes = bytes.min(self.taken);
        self.global.release(bytes);
        self.taken -= bytes;
        self.per_job_left += bytes;
    }

    /// Резервирует кредиты под сериализацию — атомарно и ДО аллокации,
    /// но не больше `cap`. Параллельные сериализаторы делят глобальный
    /// бюджет резервами, а не читают один и тот же снимок остатка,
    /// поэтому суммарная временная память сериализаций не превышает
    /// `max_live_staged_temp_bytes`. Неиспользованную часть возвращает
    /// `StagingLease::commit` либо его `Drop`.
    fn lease(&mut self, cap: usize) -> StagingLease<'_> {
        let cap = cap.min(self.per_job_left);
        let reserved = self.global.take_up_to(cap);
        StagingLease {
            budget: self,
            reserved,
        }
    }

    /// Остаток per-job кредитов — потолок роста лизинга.
    fn per_job_left(&self) -> usize {
        self.per_job_left
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

    /// Докупает кредитов у обоих бюджетов и возвращает, СКОЛЬКО выдано.
    /// Частичная выдача — не отказ: обходу довольно того, что покрывает
    /// его текущее списание, а требование выдать порцию целиком
    /// отвергало бы запись, которая помещается в оставшийся общий
    /// бюджет. Взятое остаётся за лизингом и вернётся его `Drop`.
    fn grow(&mut self, bytes: usize) -> usize {
        let room = self.budget.per_job_left.saturating_sub(self.reserved);
        let taken = self.budget.global.take_up_to(bytes.min(room));
        self.reserved += taken;
        taken
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

/// Тестовый шлюз окна «кредит взят, снимок ещё не построен».
#[cfg(test)]
struct StagingLeaseGate {
    /// Пауза срабатывает только для сеанса с этим token — параллельные
    /// тесты того же процесса шлюз не задевает.
    session: [u8; 16],
    /// Остановиться, когда удержано не меньше стольких байт. Порог по
    /// ОБЪЁМУ, а не по номеру выдачи: запас, если он есть, меняет число
    /// выдач, и сравнивать варианты по счётчику нельзя.
    stop_at_reserved: usize,
    held: Mutex<bool>,
    released: std::sync::Condvar,
    entered: std::sync::atomic::AtomicBool,
}

#[cfg(test)]
static STAGING_LEASE_GATE: Mutex<Option<Arc<StagingLeaseGate>>> = Mutex::new(None);

/// Держатель шлюза: снимает его на ЛЮБОМ выходе из теста, включая
/// панику. Без этого сорвавшийся тест оставлял бы процессно-глобальный
/// шлюз взведённым, и весь бинарник зависал бы на следующем staged-`put`
/// того же сеанса вместо понятной ошибки.
#[cfg(test)]
struct StagingGateGuard(#[allow(dead_code)] std::sync::MutexGuard<'static, ()>);

/// Шлюз один на процесс, поэтому пробы, которые им пользуются, идут по
/// очереди: параллельный тест иначе затирал бы чужой шлюз своим.
#[cfg(test)]
static STAGING_GATE_TURN: Mutex<()> = Mutex::new(());

#[cfg(test)]
impl StagingGateGuard {
    fn install(gate: &Arc<StagingLeaseGate>) -> Self {
        let turn = STAGING_GATE_TURN
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *STAGING_LEASE_GATE.lock().expect("шлюз без отравления") = Some(Arc::clone(gate));
        StagingGateGuard(turn)
    }
}

#[cfg(test)]
impl Drop for StagingGateGuard {
    fn drop(&mut self) {
        if let Ok(mut slot) = STAGING_LEASE_GATE.lock()
            && let Some(gate) = slot.take()
        {
            // Открыть на выходе обязательно: припаркованный в окне поток
            // иначе не проснётся никогда.
            if let Ok(mut held) = gate.held.lock() {
                *held = false;
            }
            gate.released.notify_all();
        }
    }
}

/// Держит поток между взятием кредита и сериализацией. Вне тестов и без
/// выставленного шлюза — пустышка.
fn staging_lease_pause(session: [u8; 16], reserved: usize) {
    #[cfg(not(test))]
    let _ = (session, reserved);
    #[cfg(test)]
    {
        let gate = STAGING_LEASE_GATE
            .lock()
            .expect("шлюз без отравления")
            .clone();
        let Some(gate) = gate else {
            return;
        };
        if gate.session != session || reserved < gate.stop_at_reserved {
            return;
        }
        if gate.entered.load(std::sync::atomic::Ordering::SeqCst) {
            // Останавливаемся ровно один раз за прогон.
            return;
        }
        gate.entered
            .store(true, std::sync::atomic::Ordering::SeqCst);
        let mut held = gate.held.lock().expect("шлюз без отравления");
        while *held {
            held = gate.released.wait(held).expect("шлюз без отравления");
        }
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

    /// Сериализует staged-запись под ТОЧНЫМ кредитом. Размер будущего
    /// снимка сначала считается сухим обходом — он не выделяет ни узлов,
    /// ни строк, — затем ровно этот размер резервируется атомарно, и
    /// только потом строится сам снимок. Резерва «на глаз» больше нет:
    /// сосед не получает отказа из-за чужого запаса, а память сверх
    /// бюджета не выделяется даже временно. Исчерпание кредитов —
    /// ловимая `ResourceLimit`; ошибка НЕпереносимого значения
    /// пробрасывается как есть (обе приходят из обхода одним вариантом).
    fn capture_staged(
        value: &BslValue,
        rt: &RuntimeShapes,
        budget: &mut StagingBudget,
        session: [u8; 16],
    ) -> RtResult<SerializedValueGraph> {
        // Сухой обход идёт под РЕЗЕРВИРУЮЩИМ бюджетом: кредиты берутся
        // порциями уже во время измерения, поэтому и служебная память
        // обходчика покрыта настоящим резервом. Параллельные измерения
        // не могут суммарно выйти за общий предел — раньше глобальный
        // кредит брался только ПОСЛЕ измерения, и пик был workers ×
        // per-job.
        let ceiling = budget.per_job_left();
        let mut lease = budget.lease(0);
        let needed = {
            let taken = &mut lease;
            let mut reserve = |bytes: usize| {
                let granted = taken.grow(bytes);
                if granted > 0 {
                    // Тестовое окно ВНУТРИ сухого прохода: кредит уже
                    // удержан, обход ещё идёт. Проба конкурентности
                    // обязана застать именно это состояние.
                    staging_lease_pause(session, taken.max_bytes());
                }
                granted
            };
            SerializedValueGraph::measure_with(
                std::slice::from_ref(value),
                rt,
                crate::GraphBudget::Reserving {
                    ceiling,
                    reserve: &mut reserve,
                },
            )
            .map_err(Self::staging_budget_error)?
        };
        // Ужимать резерв не нужно: кредиты берутся ровно по списаниям
        // обхода, поэтому удержанное и есть измеренный пик. Излишка,
        // который мог бы вытеснить соседа, не возникает вовсе — это
        // сторожит `a_held_late_reserve_does_not_reject_a_fitting_neighbour`.
        debug_assert_eq!(lease.max_bytes(), needed, "резерв разошёлся с измеренным");
        // Резерв измерения переиспользуется под сам снимок: он уже не
        // меньше нужного, а излишек вернётся вместе с `commit`.
        debug_assert!(lease.max_bytes() >= needed, "резерв меньше измеренного");
        let limits = crate::GraphLimits {
            max_bytes: lease.max_bytes(),
        };
        let graph = SerializedValueGraph::capture(std::slice::from_ref(value), rt, &limits)
            .map_err(Self::staging_budget_error)?;
        lease.commit(graph.byte_size());
        Ok(graph)
    }

    /// Отказ по МЕСТУ переводится в ловимую ошибку staging-бюджета;
    /// всякая другая причина (значение не пересекает границу сеансов)
    /// проходит нетронутой — бюджет её не маскирует.
    fn staging_budget_error(error: RtError) -> RtError {
        match error {
            RtError::ResourceLimit(text) if text == crate::value_graph::BUDGET_EXCEEDED => {
                HostError::new(
                    HostErrorCode::ResourceLimit,
                    "запись не помещается в staging-бюджет временного хранилища задания",
                )
                .raise()
            }
            other => other,
        }
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
            // Прежняя staged-запись ТОГО ЖЕ адреса замещается: публикация
            // всё равно оставила бы новейшую, а держать обе значит платить
            // за одно значение дважды — цикл записей по одному адресу
            // исчерпал бы бюджет на ровном месте. Замена ТРАНЗАКЦИОННА:
            // кредит прежнего графа держится, пока новый не снят (обе
            // копии в этот момент реально в памяти, и счётчик обязан их
            // видеть), и возвращается только при успехе. Отказ не трогает
            // ни write-set, ни кредиты — откатывать нечего.
            let session = self.token;
            let graph = match &mut self.staging_budget {
                Some(budget) => Self::capture_staged(value, rt, budget, session)?,
                None => SerializedValueGraph::capture(
                    std::slice::from_ref(value),
                    rt,
                    &crate::GraphLimits::default(),
                )?,
            };
            if let Some(index) = self.staged.iter().position(|write| write.id == target.id) {
                let previous = self.staged.remove(index);
                if let Some(budget) = &mut self.staging_budget {
                    budget.refund(previous.graph.byte_size());
                }
            }
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
    // НЕ ИЗМЕРЕНО(JOB.TEMP.READ_YOUR_WRITES): ИЗМЕРЕНО на файловой базе
    // (2026-08-27) — задание читает по caller-адресу `Неопределено`, что
    // совпадает и с документацией синтакс-помощника 8.3.27 («данные...
    // сразу после помещения недоступны в фоновом сеансе»), и с нашей
    // семантикой; клиент-серверное подтверждение остаётся за следующей
    // сессией — файловая база расходится с ней по чужому `seanceId`.
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
    /// Резерв ограничен запрошенным потолком: мелкая запись не занимает
    /// весь предел и не создаёт соседям ложных отказов.
    #[test]
    fn staging_credits_are_released_on_commit_and_on_drop() {
        let global = Arc::new(GlobalStagingBudget::new(1_000));
        {
            let mut budget = StagingBudget::new(800, Arc::clone(&global));
            let lease = budget.lease(100);
            assert_eq!(lease.max_bytes(), 100, "резерв ограничен запрошенным");
            assert_eq!(global.used(), 100, "резерв взят из глобального счётчика");
            lease.commit(60);
            assert_eq!(global.used(), 60, "commit вернул неиспользованное");
            let lease = budget.lease(usize::MAX);
            assert_eq!(lease.max_bytes(), 740, "потолок — остаток per-job");
            drop(lease);
            assert_eq!(global.used(), 60, "drop без commit вернул резерв");
            // Замещение записи возвращает её кредиты обоим бюджетам.
            budget.refund(60);
            assert_eq!(global.used(), 0);
            assert_eq!(budget.per_job_left(), 800);
        }
        assert_eq!(global.used(), 0);

        // Глобальный предел жёстче per-job: пока первый резерв держится,
        // второй сериализатор получает лишь остаток.
        let mut first = StagingBudget::new(1_000, Arc::clone(&global));
        let mut second = StagingBudget::new(1_000, Arc::clone(&global));
        let first_lease = first.lease(usize::MAX);
        assert_eq!(first_lease.max_bytes(), 1_000);
        let second_lease = second.lease(usize::MAX);
        assert_eq!(
            second_lease.max_bytes(),
            0,
            "глобальный остаток занят резервом"
        );
        drop(second_lease);
        first_lease.commit(700);
        let second_lease = second.lease(usize::MAX);
        assert_eq!(second_lease.max_bytes(), 300);
        second_lease.commit(300);
        assert_eq!(global.used(), 1_000);
    }

    /// Повторная запись по ОДНОМУ адресу замещает прежнюю, а не копится:
    /// публикация всё равно оставила бы новейшую, и платить бюджетом за
    /// каждую итерацию цикла нельзя.
    #[test]
    fn a_repeated_staged_write_replaces_the_previous_one() {
        let global = Arc::new(GlobalStagingBudget::new(64 << 10));
        let mut session = TempStorageSession::for_job(
            [1; 16],
            [2; 16],
            crate::HostEnv::process().random(),
            StagingBudget::new(8 << 10, Arc::clone(&global)),
        );
        let rt = crate::RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let caller_address = format!(
            "{ADDRESS_PREFIX}{}{ADDRESS_SEANCE}{}",
            crate::uuid::format(&[9; 16]),
            crate::uuid::format(&[2; 16])
        );
        // Каждая запись — килобайт; без замещения восемь итераций
        // исчерпали бы per-job бюджет.
        for index in 0..24 {
            let value = BslValue::Str(crate::BslString::from_str(&format!(
                "{index}{}",
                "х".repeat(1000)
            )));
            session
                .put(&value, Some(&caller_address), &rt)
                .expect("повторная запись по тому же адресу");
        }
        let writes = session.take_staged();
        assert_eq!(writes.len(), 1, "адрес обязан остаться один");
        let mut shapes = crate::RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let value = writes[0]
            .1
            .materialize(&mut shapes)
            .expect("снимок читается")
            .pop()
            .expect("значение");
        let BslValue::Str(text) = value else {
            panic!("не строка");
        };
        assert!(
            text.to_string().starts_with("23"),
            "публикуется новейшая запись"
        );
        assert_eq!(global.used(), 0, "забор write-set вернул кредиты");
    }

    /// Кредит берётся ТОЧНЫЙ: пока первая мелкая запись удерживает свой
    /// (шлюз останавливает поток МЕЖДУ взятием кредита и сериализацией),
    /// соседний сериализатор при тесном общем пределе всё равно
    /// проходит. Со старым резервом «на глаз» он получил бы отказ.
    #[test]
    fn a_held_small_write_does_not_starve_a_concurrent_one() {
        let limit = 64 << 10;
        let global = Arc::new(GlobalStagingBudget::new(limit));
        let caller_token = [2u8; 16];
        let caller_address = format!(
            "{ADDRESS_PREFIX}{}{ADDRESS_SEANCE}{}",
            crate::uuid::format(&[9; 16]),
            crate::uuid::format(&caller_token)
        );
        let gate = Arc::new(StagingLeaseGate {
            // Токены тестов НЕ пересекаются, а очередь на шлюз держит
            // `StagingGateGuard`: он один на процесс.
            session: [30; 16],
            stop_at_reserved: 1,
            held: Mutex::new(true),
            released: std::sync::Condvar::new(),
            entered: std::sync::atomic::AtomicBool::new(false),
        });
        let _gate_guard = StagingGateGuard::install(&gate);
        let first = {
            let global = Arc::clone(&global);
            let caller_address = caller_address.clone();
            std::thread::spawn(move || {
                let mut session = TempStorageSession::for_job(
                    [30; 16],
                    caller_token,
                    crate::HostEnv::process().random(),
                    StagingBudget::new(limit, Arc::clone(&global)),
                );
                let rt = crate::RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
                session
                    .put(
                        &BslValue::Str(crate::BslString::from_str("первая мелкая")),
                        Some(&caller_address),
                        &rt,
                    )
                    .expect("первая мелкая запись");
            })
        };
        // Ждём, пока первый поток ВОЗЬМЁТ кредит и остановится в окне.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !gate.entered.load(std::sync::atomic::Ordering::SeqCst) {
            assert!(
                std::time::Instant::now() < deadline,
                "первый сериализатор не дошёл до окна"
            );
            std::thread::yield_now();
        }
        assert!(global.used() > 0, "кредит первой записи держится");
        let mut second = TempStorageSession::for_job(
            [31; 16],
            caller_token,
            crate::HostEnv::process().random(),
            StagingBudget::new(limit, Arc::clone(&global)),
        );
        let rt = crate::RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let outcome = second.put(
            &BslValue::Str(crate::BslString::from_str("вторая мелкая")),
            Some(&caller_address),
            &rt,
        );
        *gate.held.lock().expect("шлюз без отравления") = false;
        gate.released.notify_all();
        first.join().expect("поток первой записи");
        outcome.expect("мелкие записи обязаны помещаться обе даже при удержанном кредите");
    }

    /// КОНКУРЕНТНАЯ проба порционного резерва: первый сеанс
    /// останавливается ВНУТРИ сухого прохода, уже удержав первую порцию,
    /// и в этот момент второй сеанс делает свою запись. Оба графа вместе
    /// помещаются в общий лимит, поэтому оба обязаны пройти — запас
    /// порции не должен вытеснять соседа.
    #[test]
    fn a_held_reserve_chunk_does_not_reject_a_fitting_neighbour() {
        // 10 КиБ на двоих, графы по несколько килобайт.
        let limit = 10 << 10;
        let global = Arc::new(GlobalStagingBudget::new(limit));
        let caller_token = [2u8; 16];
        let caller_address = format!(
            "{ADDRESS_PREFIX}{}{ADDRESS_SEANCE}{}",
            crate::uuid::format(&[9; 16]),
            crate::uuid::format(&caller_token)
        );
        let gate = Arc::new(StagingLeaseGate {
            session: [70; 16],
            stop_at_reserved: 1,
            held: Mutex::new(true),
            released: std::sync::Condvar::new(),
            entered: std::sync::atomic::AtomicBool::new(false),
        });
        let _gate_guard = StagingGateGuard::install(&gate);
        let first = {
            let global = Arc::clone(&global);
            let caller_address = caller_address.clone();
            std::thread::spawn(move || {
                let mut session = TempStorageSession::for_job(
                    [70; 16],
                    caller_token,
                    crate::HostEnv::process().random(),
                    StagingBudget::new(limit, Arc::clone(&global)),
                );
                let rt = crate::RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
                let value = BslValue::Str(crate::BslString::from_str(&"a".repeat(3 << 10)));
                session.put(&value, Some(&caller_address), &rt).is_ok()
            })
        };
        // Ждём, пока первый удержит порцию ВНУТРИ сухого прохода.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !gate.entered.load(std::sync::atomic::Ordering::SeqCst) {
            assert!(
                std::time::Instant::now() < deadline,
                "первый сеанс не дошёл до окна внутри сухого прохода"
            );
            std::thread::yield_now();
        }
        let held = global.used();
        assert!(held > 0, "первый обязан держать порцию");

        let mut second = TempStorageSession::for_job(
            [71; 16],
            caller_token,
            crate::HostEnv::process().random(),
            StagingBudget::new(limit, Arc::clone(&global)),
        );
        let rt = crate::RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let outcome = second.put(
            &BslValue::Str(crate::BslString::from_str(&"b".repeat(3 << 10))),
            Some(&caller_address),
            &rt,
        );
        *gate.held.lock().expect("шлюз без отравления") = false;
        gate.released.notify_all();
        assert!(first.join().expect("поток первой записи"), "первая запись");
        outcome.expect("сосед помещается в общий лимит и обязан пройти");
    }

    /// Удержание ПОЗДНЕГО кредита не отбирает у соседа: обход
    /// останавливается на десятой выдаче, когда прежний амортизированный
    /// запас уже успел бы накопиться, а суммарные пики обоих графов
    /// подобраны под самый общий лимит. Проходить обязаны оба —
    /// значит, лишнего резерва нет вовсе.
    #[test]
    fn a_held_late_reserve_does_not_reject_a_fitting_neighbour() {
        let caller_token = [2u8; 16];
        let caller_address = format!(
            "{ADDRESS_PREFIX}{}{ADDRESS_SEANCE}{}",
            crate::uuid::format(&[9; 16]),
            crate::uuid::format(&caller_token)
        );
        // Пики обоих графов известны заранее: лимит — ровно их сумма.
        let rt = crate::RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        // Значения не `Send` (внутри `Rc`), поэтому поток строит своё —
        // тем же детерминированным способом, что и замер ниже.
        fn build_first() -> BslValue {
            BslValue::new_array(
                (0..64)
                    .map(|i| BslValue::Str(crate::BslString::from_str(&format!("значение {i}"))))
                    .collect(),
            )
        }
        let first_value = build_first();
        let second_value = BslValue::Str(crate::BslString::from_str("сосед"));
        let first_peak = SerializedValueGraph::measure(
            std::slice::from_ref(&first_value),
            &rt,
            &crate::GraphLimits::default(),
        )
        .expect("пик первого");
        let second_peak = SerializedValueGraph::measure(
            std::slice::from_ref(&second_value),
            &rt,
            &crate::GraphLimits::default(),
        )
        .expect("пик второго");
        let limit = first_peak + second_peak;
        let global = Arc::new(GlobalStagingBudget::new(limit));
        let gate = Arc::new(StagingLeaseGate {
            session: [80; 16],
            // Останавливаемся, когда удержан ВЕСЬ пик первого графа:
            // при честном резерве соседу остаётся ровно его доля, при
            // любом запасе — уже нет.
            stop_at_reserved: first_peak,
            held: Mutex::new(true),
            released: std::sync::Condvar::new(),
            entered: std::sync::atomic::AtomicBool::new(false),
        });
        let _gate_guard = StagingGateGuard::install(&gate);
        let first = {
            let global = Arc::clone(&global);
            let caller_address = caller_address.clone();
            std::thread::spawn(move || {
                let mut session = TempStorageSession::for_job(
                    [80; 16],
                    caller_token,
                    crate::HostEnv::process().random(),
                    StagingBudget::new(limit, Arc::clone(&global)),
                );
                let rt = crate::RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
                session
                    .put(&build_first(), Some(&caller_address), &rt)
                    .is_ok()
            })
        };
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !gate.entered.load(std::sync::atomic::Ordering::SeqCst) {
            assert!(
                std::time::Instant::now() < deadline,
                "первый сеанс не дошёл до поздней выдачи"
            );
            std::thread::yield_now();
        }
        let held = global.used();
        assert!(held > 0, "первый обязан держать кредит");

        let mut second = TempStorageSession::for_job(
            [81; 16],
            caller_token,
            crate::HostEnv::process().random(),
            StagingBudget::new(limit, Arc::clone(&global)),
        );
        let outcome = second.put(&second_value, Some(&caller_address), &rt);
        *gate.held.lock().expect("шлюз без отравления") = false;
        gate.released.notify_all();
        assert!(first.join().expect("поток первой записи"), "первая запись");
        outcome.expect("сосед укладывается в общий лимит и обязан пройти");
    }

    /// Общий лимит МЕНЬШЕ одной порции резерва: обе мелкие записи всё
    /// равно проходят, потому что при тесном потолке берётся ровно
    /// нужное, а излишек порции возвращается сразу после измерения.
    /// Первый сеанс жив и держит свой кредит, пока второй пробует.
    #[test]
    fn a_budget_below_one_reserve_chunk_still_admits_both_writes() {
        // 1 КиБ на двоих при порции резерва 8 КиБ.
        let global = Arc::new(GlobalStagingBudget::new(1 << 10));
        let caller_token = [2u8; 16];
        let caller_address = format!(
            "{ADDRESS_PREFIX}{}{ADDRESS_SEANCE}{}",
            crate::uuid::format(&[9; 16]),
            crate::uuid::format(&caller_token)
        );
        let rt = crate::RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let mut first = TempStorageSession::for_job(
            [60; 16],
            caller_token,
            crate::HostEnv::process().random(),
            StagingBudget::new(1 << 10, Arc::clone(&global)),
        );
        first
            .put(
                &BslValue::Str(crate::BslString::from_str("первая")),
                Some(&caller_address),
                &rt,
            )
            .expect("первая запись");
        let held = global.used();
        assert!(held > 0 && held < (1 << 10), "первая держит {held} байт");
        let mut second = TempStorageSession::for_job(
            [61; 16],
            caller_token,
            crate::HostEnv::process().random(),
            StagingBudget::new(1 << 10, Arc::clone(&global)),
        );
        second
            .put(
                &BslValue::Str(crate::BslString::from_str("вторая")),
                Some(&caller_address),
                &rt,
            )
            .expect("вторая запись помещается: сумма графов много меньше лимита");
        assert!(global.used() <= (1 << 10));
        drop((first, second));
        assert_eq!(global.used(), 0, "кредиты вернулись");
    }

    /// ТЕСНЫЙ общий бюджет: порция резерва больше остатка, но запись,
    /// которая в остаток помещается, обязана пройти. Прежде частичная
    /// выдача считалась отказом, и вторая мелкая запись падала, хотя
    /// места ей хватало.
    #[test]
    fn a_tight_global_budget_still_admits_a_fitting_write() {
        // 10 КиБ на двоих при порции резерва 8 КиБ: второй получит
        // остаток около двух килобайт — этого достаточно.
        let global = Arc::new(GlobalStagingBudget::new(10 << 10));
        let caller_token = [2u8; 16];
        let caller_address = format!(
            "{ADDRESS_PREFIX}{}{ADDRESS_SEANCE}{}",
            crate::uuid::format(&[9; 16]),
            crate::uuid::format(&caller_token)
        );
        let rt = crate::RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let mut first = TempStorageSession::for_job(
            [50; 16],
            caller_token,
            crate::HostEnv::process().random(),
            StagingBudget::new(10 << 10, Arc::clone(&global)),
        );
        first
            .put(
                &BslValue::Str(crate::BslString::from_str("первая")),
                Some(&caller_address),
                &rt,
            )
            .expect("первая запись");
        // Первый сеанс ЖИВ и держит свой кредит.
        let mut second = TempStorageSession::for_job(
            [51; 16],
            caller_token,
            crate::HostEnv::process().random(),
            StagingBudget::new(10 << 10, Arc::clone(&global)),
        );
        second
            .put(
                &BslValue::Str(crate::BslString::from_str("вторая")),
                Some(&caller_address),
                &rt,
            )
            .expect("вторая запись помещается в остаток общего бюджета");
        assert!(global.used() <= (10 << 10));
        drop((first, second));
        assert_eq!(global.used(), 0, "кредиты вернулись");
    }

    /// Замещение записи ТРАНЗАКЦИОННО: пока новый снимок собирается,
    /// кредит прежнего держится (обе копии в памяти), и глобальный
    /// счётчик их видит; отказ не трогает ни write-set, ни кредиты.
    #[test]
    fn a_replacement_holds_the_previous_credit_until_it_succeeds() {
        let global = Arc::new(GlobalStagingBudget::new(64 << 10));
        let mut session = TempStorageSession::for_job(
            [1; 16],
            [2; 16],
            crate::HostEnv::process().random(),
            // Per-job бюджет вмещает одну запись по 8 КиБ, но не две.
            StagingBudget::new(12 << 10, Arc::clone(&global)),
        );
        let rt = crate::RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let caller_address = format!(
            "{ADDRESS_PREFIX}{}{ADDRESS_SEANCE}{}",
            crate::uuid::format(&[9; 16]),
            crate::uuid::format(&[2; 16])
        );
        let big = BslValue::Str(crate::BslString::from_str(&"a".repeat(8 << 10)));
        session
            .put(&big, Some(&caller_address), &rt)
            .expect("первая запись");
        let after_first = global.used();
        assert!(after_first >= (8 << 10), "кредит первой записи держится");
        // Замена ТЕМ ЖЕ размером: обе копии одновременно в бюджет не
        // помещаются, поэтому попытка отвергается — и откат не портит ни
        // накопленный write-set, ни кредиты.
        let error = session
            .put(&big, Some(&caller_address), &rt)
            .expect_err("две копии сразу не помещаются в per-job бюджет");
        assert!(
            matches!(error, RtError::Host(_)),
            "ловимый отказ: {error:?}"
        );
        assert_eq!(
            global.used(),
            after_first,
            "отказ замены не изменил занятые кредиты"
        );
        let writes = session.take_staged();
        assert_eq!(writes.len(), 1, "прежняя запись цела");
        assert_eq!(global.used(), 0, "забор write-set вернул кредит");
    }

    /// Мелкая запись не занимает весь глобальный предел на время
    /// сериализации: соседний сериализатор получает свой кредит, а не
    /// ложный отказ.
    #[test]
    fn a_small_write_does_not_reserve_the_whole_global_budget() {
        let global = Arc::new(GlobalStagingBudget::new(256 << 10));
        let mut session = TempStorageSession::for_job(
            [1; 16],
            [2; 16],
            crate::HostEnv::process().random(),
            StagingBudget::new(256 << 10, Arc::clone(&global)),
        );
        let rt = crate::RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let caller_address = format!(
            "{ADDRESS_PREFIX}{}{ADDRESS_SEANCE}{}",
            crate::uuid::format(&[9; 16]),
            crate::uuid::format(&[2; 16])
        );
        session
            .put(
                &BslValue::Str(crate::BslString::from_str("мелкая")),
                Some(&caller_address),
                &rt,
            )
            .expect("мелкая запись");
        assert!(
            global.used() < (64 << 10),
            "мелкая запись заняла {} байт глобального бюджета",
            global.used()
        );
    }

    /// Значение, не пересекающее границу сеансов, отвечает своей ошибкой
    /// и в staged-записи: бюджет не подменяет её текстом «не помещается».
    #[test]
    fn a_non_portable_staged_value_keeps_its_own_error() {
        let global = Arc::new(GlobalStagingBudget::new(1 << 20));
        let mut session = TempStorageSession::for_job(
            [1; 16],
            [2; 16],
            crate::HostEnv::process().random(),
            StagingBudget::new(1 << 20, Arc::clone(&global)),
        );
        let rt = crate::RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let caller_address = format!(
            "{ADDRESS_PREFIX}{}{ADDRESS_SEANCE}{}",
            crate::uuid::format(&[9; 16]),
            crate::uuid::format(&[2; 16])
        );
        let value =
            BslValue::new_object(crate::user_message::UserMessageObject::with_text("не едет"));
        let error = session
            .put(&value, Some(&caller_address), &rt)
            .expect_err("значение не переносится между сеансами");
        let RtError::ResourceLimit(text) = &error else {
            panic!("не тот класс ошибки: {error:?}");
        };
        assert!(
            text.contains("не переносится"),
            "бюджет не имеет права маскировать причину: {text}"
        );
        assert_eq!(global.used(), 0, "отказ не занимает кредиты");
    }

    /// Два параллельных сериализатора ДЕЛЯТ глобальный бюджет резервами:
    /// каждый ограничен своим кредитом, поэтому суммарная временная
    /// память сериализаций не превышает предел (а не workers × предел), и
    /// из двух записей, не помещающихся вместе, проходит ровно одна.
    /// ОБА сеанса удерживаются живыми до завершения обеих попыток —
    /// иначе успевший освободить кредит пропустил бы соседа, и тест
    /// случайно проходил бы вдвоём.
    #[test]
    fn two_serializers_reserve_the_global_budget_before_capture() {
        let limit = 64 << 10;
        let global = Arc::new(GlobalStagingBudget::new(limit));
        let start = Arc::new(std::sync::Barrier::new(2));
        let hold = Arc::new(std::sync::Barrier::new(2));
        let caller_token = [2u8; 16];
        let caller_address = format!(
            "{ADDRESS_PREFIX}{}{ADDRESS_SEANCE}{}",
            crate::uuid::format(&[9; 16]),
            crate::uuid::format(&caller_token)
        );
        let mut workers = Vec::new();
        for index in 0..2u8 {
            let global = Arc::clone(&global);
            let start = Arc::clone(&start);
            let hold = Arc::clone(&hold);
            let caller_address = caller_address.clone();
            workers.push(std::thread::spawn(move || {
                // Сеанс не `Send` — каждый поток строит свой.
                let mut session = TempStorageSession::for_job(
                    [index + 40; 16],
                    caller_token,
                    crate::HostEnv::process().random(),
                    StagingBudget::new(limit, Arc::clone(&global)),
                );
                let rt = crate::RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
                // 48 КиБ на запись: две вместе в 64 КиБ не помещаются.
                let value = BslValue::Str(crate::BslString::from_str(&"y".repeat(48 << 10)));
                start.wait();
                let outcome = session.put(&value, Some(&caller_address), &rt);
                assert!(
                    global.used() <= limit,
                    "сумма резервов и кредитов не превышает предел"
                );
                // Сеанс ЖИВ, пока сосед не закончил свою попытку: иначе
                // освобождённый кредит пропустил бы обоих.
                hold.wait();
                drop(session);
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

    /// Забор write-set сеансом задания освобождает кредиты сразу    /// Забор write-set сеансом задания освобождает кредиты сразу: они
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
