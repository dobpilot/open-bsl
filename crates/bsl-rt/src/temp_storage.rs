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
use std::sync::{Arc, Mutex, Weak};

use crate::{BslValue, RtError, RtResult, RuntimeShapes, SerializedValueGraph};

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
        self.mailboxes
            .lock()
            .expect("реестр mailbox без отравления")
            .insert(token, Arc::downgrade(mailbox));
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
            random,
        }
    }

    /// Сеанс задания: свой token плюс token непосредственного вызывателя.
    #[must_use]
    pub fn for_job(token: [u8; 16], caller: [u8; 16], random: crate::RandomHandle) -> Self {
        let mut session = Self::new(token, random);
        session.caller = Some(caller);
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

    /// Забирает staging для публикации на terminal transition.
    pub fn take_staged(&mut self) -> Vec<([u8; 16], SerializedValueGraph)> {
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
                RtError::ResourceLimit(format!("«{text}» не адрес временного хранилища"))
            })?,
        };
        if target.token == self.token {
            let sequence = self.mailbox.next_sequence();
            self.local.insert(target.id, (sequence, value.clone()));
        } else if self.caller == Some(target.token) {
            // Запись по адресу вызывателя публикуется НА TERMINAL, а не
            // сразу; до него вызыватель видит прежнее значение. Сама
            // запись сериализуется сейчас: задание продолжит менять свой
            // Rc-граф, а снимок уже неизменен.
            let graph = SerializedValueGraph::capture(
                std::slice::from_ref(value),
                rt,
                &crate::GraphLimits::default(),
            )?;
            self.staged.push(StagedWrite {
                id: target.id,
                graph,
            });
        } else {
            // ИЗМЕРЕНО (JOB.TEMP.FOREIGN): запись по чужому адресу — ошибка
            // с пустым описанием у платформы; свой текст информативнее, а
            // точный пустой текст намеренно не копируется.
            return Err(RtError::ResourceLimit(
                "адрес принадлежит другому сеансу".to_string(),
            ));
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
    pub fn delete(&mut self, address: &str) {
        let Some(target) = parse_address(address) else {
            return;
        };
        if target.token != self.token {
            return;
        }
        self.local.remove(&target.id);
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
