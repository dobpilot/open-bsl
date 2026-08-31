//! Граф потока управления над РАЗРЕШЁННЫМ ДЕРЕВОМ (`ResolvedProgram`).
//!
//! Шаг 5 плана `docs/research/performance/ssa-hotspot-analysis.md`: SSA принято строить в
//! компиляторе, до выбора инструкций. Этот модуль — его первая половина,
//! граф, и он ничего не меняет в выпускаемом байт-коде: пока это чистый
//! анализ под проверкой инвариантов.
//!
//! # Почему это не дубль графа из `bsl-bytecode`
//!
//! Там граф строится над ГОТОВЫМ байт-кодом, и для задачи шага 5 он
//! непригоден принципиально, а не по неудобству. К моменту байт-кода
//! переменные превратились в регистры, а регистры переиспользуются и
//! лгут об алиасинге: параметры без `Знач` делят слот, модульные
//! переменные накладываются на регистры кадра нулевого уровня. SSA же
//! обязана нумеровать ЗНАЧЕНИЯ переменных, а не занятые ими ячейки, и
//! потому строится там, где переменная ещё переменная.
//!
//! # Что здесь считается рекурсией, запрещённой дефектом 4
//!
//! Прошлая попытка (см. «Почему попытка повторяется») сломалась на том,
//! что валидный скрипт из 8000 последовательных `Если` давал 0,20 с без
//! JIT против 2,58 с с ним. Виноваты были обходы ГРАФА и плотная матрица
//! доминаторов — величины, растущие с числом БЛОКОВ. Обход дерева по
//! вложенности к этому отношения не имеет: последовательные `Если` —
//! соседи в одном `Vec<RStmt>`, глубина на них равна единице, а на
//! настоящей вложенности её и так ограничивает рекурсивный фронтенд
//! (измерено: разбор с резолвингом переполняют двухмегабайтный стек уже
//! на 200 уровнях, одинаково со свёрткой и без). Поэтому построитель
//! обходит дерево рекурсивно, повторяя форму `compile_stmt`, а все
//! алгоритмы НА ГРАФЕ — обход, доминаторы — итеративные и разреженные.

use bsl_sema::{LabelId, RExpr, RStmt, RStmtKind};
use std::collections::HashMap;

pub type BlockId = usize;

/// Чем блок заканчивается.
#[derive(Debug)]
pub enum Terminator<'a> {
    /// Управление переходит в единственного преемника.
    Goto(BlockId),
    /// Ветвление: условие и оба преемника.
    ///
    /// `cond` равно `None` у цикла по счётчику: его проверку задаёт не
    /// выражение дерева, а сам оператор `Для`. Отдельного варианта
    /// терминатора для этого нет намеренно — разбор потока управления у
    /// всех ветвлений один, и различие нужно только тому, кто смотрит на
    /// данные.
    Branch {
        cond: Option<&'a RExpr>,
        then_b: BlockId,
        else_b: BlockId,
    },
    /// `Возврат` — выход из функции.
    Return(Option<&'a RExpr>),
    /// `ВызватьИсключение` — выход по исключительному пути.
    Raise(Option<&'a RExpr>),
    /// Блок, из которого управление никуда не идёт: конец тела.
    Exit,
}

/// Блок: линейная последовательность операторов и один терминатор.
#[derive(Debug)]
pub struct Block<'a> {
    pub stmts: Vec<&'a RStmt>,
    pub term: Terminator<'a>,
    pub preds: Vec<BlockId>,
    /// Рёбра в обработчики `Исключение`, накрывающие этот блок.
    ///
    /// Отдельным полем, а не в терминаторе: исключение может сработать на
    /// ЛЮБОМ операторе защищённого тела, поэтому ребро идёт из блока
    /// целиком, а не из его конца. План требует у обработчиков отдельные
    /// входные рёбра именно поэтому.
    pub handlers: Vec<BlockId>,
}

pub struct Cfg<'a> {
    pub blocks: Vec<Block<'a>>,
    pub entry: BlockId,
}

impl<'a> Cfg<'a> {
    /// Преемники блока: из терминатора плюс рёбра в обработчики.
    #[must_use]
    pub fn succs(&self, b: BlockId) -> Vec<BlockId> {
        let mut out = match &self.blocks[b].term {
            Terminator::Goto(t) => vec![*t],
            Terminator::Branch { then_b, else_b, .. } => vec![*then_b, *else_b],
            Terminator::Return(_) | Terminator::Raise(_) | Terminator::Exit => Vec::new(),
        };
        out.extend(self.blocks[b].handlers.iter().copied());
        out
    }

    /// Обратный постпорядок достижимых блоков. Обход итеративный: его
    /// глубина растёт с числом блоков, а это ровно та величина, на
    /// которой сломалась прошлая попытка.
    #[must_use]
    pub fn reverse_postorder(&self) -> Vec<BlockId> {
        let n = self.blocks.len();
        let mut seen = vec![false; n];
        let mut post = Vec::with_capacity(n);
        // Стек кадров «блок и сколько его преемников уже обработано».
        let mut stack: Vec<(BlockId, usize)> = vec![(self.entry, 0)];
        seen[self.entry] = true;
        while let Some((b, i)) = stack.pop() {
            let succs = self.succs(b);
            if i < succs.len() {
                stack.push((b, i + 1));
                let s = succs[i];
                if !seen[s] {
                    seen[s] = true;
                    stack.push((s, 0));
                }
            } else {
                post.push(b);
            }
        }
        post.reverse();
        post
    }

    /// Непосредственные доминаторы, разреженно: `None` у входа и у
    /// недостижимых блоков.
    ///
    /// Алгоритм Купера—Харви—Кеннеди: итерация по обратному постпорядку
    /// до неподвижной точки. Плотной матрицы `блоки × блоки` здесь нет и
    /// быть не должно — именно она съедала память прошлой попытки.
    #[must_use]
    pub fn immediate_dominators(&self) -> Vec<Option<BlockId>> {
        let rpo = self.reverse_postorder();
        let mut order = vec![usize::MAX; self.blocks.len()];
        for (i, &b) in rpo.iter().enumerate() {
            order[b] = i;
        }
        let mut idom: Vec<Option<BlockId>> = vec![None; self.blocks.len()];
        idom[self.entry] = Some(self.entry);
        let mut changed = true;
        while changed {
            changed = false;
            for &b in rpo.iter().skip(1) {
                let mut new: Option<BlockId> = None;
                for p in &self.blocks[b].preds {
                    if idom[*p].is_none() {
                        continue;
                    }
                    new = Some(match new {
                        None => *p,
                        Some(cur) => intersect(&idom, &order, cur, *p),
                    });
                }
                if new.is_some() && new != idom[b] {
                    idom[b] = new;
                    changed = true;
                }
            }
        }
        // У входа собственный доминатор — служебное значение алгоритма,
        // наружу он отдаётся как «доминатора нет».
        idom[self.entry] = None;
        idom
    }
}

/// Ближайший общий доминатор двух блоков по обратному постпорядку.
fn intersect(idom: &[Option<BlockId>], order: &[usize], mut a: BlockId, mut b: BlockId) -> BlockId {
    while a != b {
        while order[a] > order[b] {
            let Some(next) = idom[a] else { return b };
            if next == a {
                return b;
            }
            a = next;
        }
        while order[b] > order[a] {
            let Some(next) = idom[b] else { return a };
            if next == b {
                return a;
            }
            b = next;
        }
    }
    a
}

/// Проверка инвариантов графа, независимая от построителя.
///
/// # Errors
///
/// Описание первого нарушенного инварианта.
pub fn verify(cfg: &Cfg<'_>) -> Result<(), String> {
    let n = cfg.blocks.len();
    for b in 0..n {
        for s in cfg.succs(b) {
            if s >= n {
                return Err(format!("блок {b} ссылается на несуществующий {s}"));
            }
            if !cfg.blocks[s].preds.contains(&b) {
                return Err(format!("ребро {b} -> {s} есть, обратной ссылки нет"));
            }
        }
        for &p in &cfg.blocks[b].preds {
            if p >= n {
                return Err(format!("у блока {b} несуществующий предшественник {p}"));
            }
            if !cfg.succs(p).contains(&b) {
                return Err(format!("предшественник {p} блока {b} его не знает"));
            }
        }
    }

    let rpo = cfg.reverse_postorder();
    let idom = cfg.immediate_dominators();
    let mut reachable = vec![false; n];
    for &b in &rpo {
        reachable[b] = true;
    }
    for b in 0..n {
        if b == cfg.entry {
            if idom[b].is_some() {
                return Err("у входного блока не должно быть доминатора".to_string());
            }
            continue;
        }
        // Недостижимый блок — это `Bottom`, а не «доминатор неизвестен»:
        // у него нет входного пути, и приписывать ему доминатора значило
        // бы утверждать о нём то, чего нет.
        if reachable[b] != idom[b].is_some() {
            return Err(format!(
                "блок {b}: достижимость {} не согласована с доминатором {:?}",
                reachable[b], idom[b]
            ));
        }
    }
    Ok(())
}

/// Куда уходят `Прервать` и `Продолжить` текущего цикла.
struct LoopCtx {
    brk: BlockId,
    cont: BlockId,
}

struct Builder<'a> {
    blocks: Vec<Block<'a>>,
    loops: Vec<LoopCtx>,
    /// Блоки, накрытые защищёнными телами: пока стек непуст, каждый новый
    /// блок получает рёбра в обработчики.
    handlers: Vec<BlockId>,
    labels: HashMap<LabelId, BlockId>,
    gotos: Vec<(BlockId, LabelId)>,
}

impl<'a> Builder<'a> {
    fn new_block(&mut self) -> BlockId {
        self.blocks.push(Block {
            stmts: Vec::new(),
            term: Terminator::Exit,
            preds: Vec::new(),
            handlers: self.handlers.clone(),
        });
        self.blocks.len() - 1
    }

    /// Компонует список операторов начиная с блока `cur`; возвращает блок,
    /// в котором управление оказывается после последнего оператора.
    fn seq(&mut self, stmts: &'a [RStmt], mut cur: BlockId) -> BlockId {
        for s in stmts {
            cur = self.stmt(s, cur);
        }
        cur
    }

    fn stmt(&mut self, s: &'a RStmt, cur: BlockId) -> BlockId {
        match &s.kind {
            RStmtKind::If {
                cond,
                then_branch,
                elsif_branches,
                else_branch,
            } => {
                let join = self.new_block();
                let mut cond_block = cur;
                let mut conds: Vec<(&RExpr, &[RStmt])> = vec![(cond, then_branch)];
                for (c, body) in elsif_branches {
                    conds.push((c, body));
                }
                for (c, body) in conds {
                    let then_b = self.new_block();
                    let else_b = self.new_block();
                    self.blocks[cond_block].term = Terminator::Branch {
                        cond: Some(c),
                        then_b,
                        else_b,
                    };
                    let end = self.seq(body, then_b);
                    self.blocks[end].term = Terminator::Goto(join);
                    cond_block = else_b;
                }
                let end = match else_branch {
                    Some(body) => self.seq(body, cond_block),
                    None => cond_block,
                };
                self.blocks[end].term = Terminator::Goto(join);
                join
            }
            RStmtKind::While { cond, body } => {
                let header = self.new_block();
                self.blocks[cur].term = Terminator::Goto(header);
                let body_b = self.new_block();
                let exit = self.new_block();
                self.blocks[header].term = Terminator::Branch {
                    cond: Some(cond),
                    then_b: body_b,
                    else_b: exit,
                };
                self.loops.push(LoopCtx {
                    brk: exit,
                    cont: header,
                });
                let end = self.seq(body, body_b);
                self.loops.pop();
                self.blocks[end].term = Terminator::Goto(header);
                exit
            }
            // Оба цикла по счётчику устроены одинаково с точки зрения
            // потока управления: заголовок с проверкой, тело, выход.
            // Различие — в том, что проверяется, и графа оно не касается.
            RStmtKind::ForNumeric { body, .. } | RStmtKind::ForEach { body, .. } => {
                let header = self.new_block();
                self.blocks[cur].term = Terminator::Goto(header);
                // Сам оператор цикла кладётся в ЗАГОЛОВОК, и это не
                // формальность: он ПИШЕТ переменную цикла на каждой
                // итерации. Без него анализ не видит этого определения
                // вовсе, считает переменную никогда не присваиваемой и
                // разрешает ей делить регистр с живым соседом.
                self.blocks[header].stmts.push(s);
                let body_b = self.new_block();
                let exit = self.new_block();
                self.blocks[header].term = Terminator::Branch {
                    cond: None,
                    then_b: body_b,
                    else_b: exit,
                };
                self.loops.push(LoopCtx {
                    brk: exit,
                    cont: header,
                });
                let end = self.seq(body, body_b);
                self.loops.pop();
                self.blocks[end].term = Terminator::Goto(header);
                exit
            }
            RStmtKind::Try { body, except_body } => {
                let handler = self.new_block();
                let join = self.new_block();
                let protected = self.new_block();
                self.blocks[cur].term = Terminator::Goto(protected);
                self.handlers.push(handler);
                // Блок, начинающий защищённое тело, создан ДО того, как
                // обработчик попал в стек, поэтому ребро ему добавляется
                // отдельно.
                self.blocks[protected].handlers.push(handler);
                let end = self.seq(body, protected);
                self.handlers.pop();
                self.blocks[end].term = Terminator::Goto(join);
                let hend = self.seq(except_body, handler);
                self.blocks[hend].term = Terminator::Goto(join);
                join
            }
            RStmtKind::Break | RStmtKind::Continue => {
                let target = self.loops.last().map(|l| match &s.kind {
                    RStmtKind::Break => l.brk,
                    _ => l.cont,
                });
                // Вне цикла это ошибка резолвинга, до графа она не
                // доходит; на всякий случай блок просто становится
                // выходом, а не паникует.
                match target {
                    Some(t) => self.blocks[cur].term = Terminator::Goto(t),
                    None => self.blocks[cur].term = Terminator::Exit,
                }
                self.new_block()
            }
            RStmtKind::Label(id) => {
                let b = self.new_block();
                self.labels.insert(*id, b);
                self.blocks[cur].term = Terminator::Goto(b);
                b
            }
            RStmtKind::Goto(id) => {
                self.gotos.push((cur, *id));
                self.new_block()
            }
            RStmtKind::Return(e) => {
                self.blocks[cur].term = Terminator::Return(e.as_ref());
                self.new_block()
            }
            RStmtKind::Raise(e) => {
                self.blocks[cur].term = Terminator::Raise(e.as_ref());
                self.new_block()
            }
            _ => {
                self.blocks[cur].stmts.push(s);
                cur
            }
        }
    }
}

/// Строит граф тела функции или верхнего уровня.
#[must_use]
pub fn build(body: &[RStmt]) -> Cfg<'_> {
    let mut b = Builder {
        blocks: Vec::new(),
        loops: Vec::new(),
        handlers: Vec::new(),
        labels: HashMap::new(),
        gotos: Vec::new(),
    };
    let entry = b.new_block();
    let end = b.seq(body, entry);
    b.blocks[end].term = Terminator::Exit;

    // Переходы к меткам патчатся после обхода: метка может стоять ниже
    // своего `Перейти`.
    for (from, id) in std::mem::take(&mut b.gotos) {
        match b.labels.get(&id) {
            Some(&target) => b.blocks[from].term = Terminator::Goto(target),
            None => b.blocks[from].term = Terminator::Exit,
        }
    }

    let mut cfg = Cfg {
        blocks: b.blocks,
        entry,
    };
    // Предшественники — производная от преемников, и считаются один раз
    // здесь, чтобы у построителя не было второго способа их задать.
    let n = cfg.blocks.len();
    for from in 0..n {
        for to in cfg.succs(from) {
            if !cfg.blocks[to].preds.contains(&from) {
                cfg.blocks[to].preds.push(from);
            }
        }
    }
    cfg
}
