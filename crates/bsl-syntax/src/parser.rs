use crate::ast::*;
use crate::diagnostics::{Diagnostic, ParseError};
use crate::keywords::Keyword;
use crate::lexer::{LexError, Lexer};
use crate::token::{Span, Token, TokenKind};

/// Ключевые слова, закрывающие блок операторов — используются, чтобы понять,
/// где остановиться при разборе тела `Если`/`Пока`/`Для`/... и чтобы отличить
/// "пустой `Возврат`/`ВызватьИсключение`" от случая с выражением.
const BLOCK_ENDERS: &[Keyword] = &[
    Keyword::EndProcedure,
    Keyword::EndFunction,
    Keyword::EndIf,
    Keyword::Else,
    Keyword::ElsIf,
    Keyword::EndDo,
    Keyword::Except,
    Keyword::EndTry,
];

/// Разбирает исходный текст BSL в AST.
///
/// # Errors
///
/// Возвращает [`Diagnostic`], если лексер встретил недопустимый токен или текст не
/// соответствует грамматике BSL.
pub fn parse(src: &str) -> Result<Program, Diagnostic> {
    let tokens = tokenize_all(src).map_err(Diagnostic::Lex)?;
    let mut parser = Parser { tokens, pos: 0, src };
    parser.parse_program().map_err(Diagnostic::Parse)
}

fn tokenize_all(src: &str) -> Result<Vec<Token>, LexError> {
    let mut lexer = Lexer::new(src);
    let mut tokens = Vec::new();
    loop {
        let tok = lexer.next_token()?;
        let is_eof = tok.kind == TokenKind::Eof;
        tokens.push(tok);
        if is_eof {
            return Ok(tokens);
        }
    }
}

struct Parser<'src> {
    tokens: Vec<Token>,
    pos: usize,
    src: &'src str,
}

impl<'src> Parser<'src> {
    fn text(&self, span: Span) -> &'src str {
        &self.src[span.start as usize..span.end as usize]
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn bump(&mut self) -> Token {
        let tok = self.tokens[self.pos].clone();
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    fn at(&self, kind: &TokenKind) -> bool {
        self.peek().kind == *kind
    }

    fn eat(&mut self, kind: &TokenKind) -> bool {
        if self.at(kind) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: &TokenKind) -> Result<Token, ParseError> {
        if self.at(kind) {
            Ok(self.bump())
        } else {
            Err(self.error_here(format!("ожидался {kind:?}, найден {:?}", self.peek().kind)))
        }
    }

    fn expect_lparen(&mut self) -> Result<(), ParseError> {
        self.expect(&TokenKind::LParen).map(|_| ())
    }

    fn expect_rparen(&mut self) -> Result<(), ParseError> {
        self.expect(&TokenKind::RParen).map(|_| ())
    }

    fn at_keyword(&self, kw: Keyword) -> bool {
        matches!(&self.peek().kind, TokenKind::Keyword(k) if *k == kw)
    }

    fn at_any_keyword(&self, kws: &[Keyword]) -> bool {
        kws.iter().any(|k| self.at_keyword(*k))
    }

    fn eat_keyword(&mut self, kw: Keyword) -> bool {
        if self.at_keyword(kw) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn expect_keyword(&mut self, kw: Keyword) -> Result<(), ParseError> {
        if self.eat_keyword(kw) {
            Ok(())
        } else {
            Err(self.error_here(format!(
                "ожидалось ключевое слово {kw:?}, найден {:?}",
                self.peek().kind
            )))
        }
    }

    /// Имя ЧЛЕНА после точки. В отличие от [`expect_ident`](Self::expect_ident)
    /// принимает и ключевое слово: `ТипЗначенияJSON.Null`,
    /// `Соответствие.Значение` — законные обращения, а `Null`/`Значение`
    /// одновременно ключевые слова языка. Лексер их уже пометил как
    /// `Keyword`, поэтому решение принимается здесь, по позиции: после
    /// точки идёт ИМЯ, и ничем другим оно быть не может.
    fn expect_member_name(&mut self) -> Result<String, ParseError> {
        if matches!(self.peek().kind, TokenKind::Ident | TokenKind::Keyword(_)) {
            let tok = self.bump();
            Ok(self.text(tok.span).to_string())
        } else {
            Err(self.error_here(format!(
                "ожидалось имя после точки, найден {:?}",
                self.peek().kind
            )))
        }
    }

    fn expect_ident(&mut self) -> Result<String, ParseError> {
        if matches!(self.peek().kind, TokenKind::Ident) {
            let tok = self.bump();
            Ok(self.text(tok.span).to_string())
        } else {
            Err(self.error_here(format!("ожидался идентификатор, найден {:?}", self.peek().kind)))
        }
    }

    fn at_eof(&self) -> bool {
        matches!(self.peek().kind, TokenKind::Eof)
    }

    fn skip_semicolons(&mut self) {
        while self.eat(&TokenKind::Semicolon) {}
    }

    fn error_here(&self, message: String) -> ParseError {
        ParseError {
            message,
            span: self.peek().span,
        }
    }

    /// Верно ли, что текущая позиция — граница оператора (конец блока,
    /// точка с запятой, конец файла)? Используется, чтобы понять, есть ли
    /// выражение после `Возврат`/`ВызватьИсключение`.
    fn at_stmt_boundary(&self) -> bool {
        self.at_eof() || self.at(&TokenKind::Semicolon) || self.at_any_keyword(BLOCK_ENDERS)
    }

    // --- Программа и объявления -------------------------------------------

    fn parse_program(&mut self) -> Result<Program, ParseError> {
        let mut items = Vec::new();
        self.skip_semicolons();
        while !self.at_eof() {
            items.push(self.parse_item()?);
            self.skip_semicolons();
        }
        Ok(Program { items })
    }

    fn parse_item(&mut self) -> Result<Item, ParseError> {
        if self.at_keyword(Keyword::Procedure) {
            Ok(Item::Procedure(self.parse_procedure()?))
        } else if self.at_keyword(Keyword::Function) {
            Ok(Item::Function(self.parse_function()?))
        } else if self.at_keyword(Keyword::Var) {
            Ok(Item::VarDecl(self.parse_var_decl()?))
        } else {
            Ok(Item::Stmt(self.parse_stmt()?))
        }
    }

    fn parse_params(&mut self) -> Result<Vec<Param>, ParseError> {
        self.expect_lparen()?;
        let mut params = Vec::new();
        if !self.at(&TokenKind::RParen) {
            loop {
                let by_val = self.eat_keyword(Keyword::Val);
                let name = self.expect_ident()?;
                let default = if self.eat(&TokenKind::Eq) {
                    Some(self.parse_expr()?)
                } else {
                    None
                };
                params.push(Param {
                    name,
                    by_val,
                    default,
                });
                if !self.eat(&TokenKind::Comma) {
                    break;
                }
            }
        }
        self.expect_rparen()?;
        Ok(params)
    }

    fn parse_procedure(&mut self) -> Result<ProcDecl, ParseError> {
        self.expect_keyword(Keyword::Procedure)?;
        let name = self.expect_ident()?;
        let params = self.parse_params()?;
        let export = self.eat_keyword(Keyword::Export);
        let body = self.parse_block(&[Keyword::EndProcedure])?;
        self.expect_keyword(Keyword::EndProcedure)?;
        Ok(ProcDecl {
            name,
            params,
            export,
            body,
        })
    }

    fn parse_function(&mut self) -> Result<FuncDecl, ParseError> {
        self.expect_keyword(Keyword::Function)?;
        let name = self.expect_ident()?;
        let params = self.parse_params()?;
        let export = self.eat_keyword(Keyword::Export);
        let body = self.parse_block(&[Keyword::EndFunction])?;
        self.expect_keyword(Keyword::EndFunction)?;
        Ok(FuncDecl {
            name,
            params,
            export,
            body,
        })
    }

    fn parse_var_decl(&mut self) -> Result<VarDecl, ParseError> {
        self.expect_keyword(Keyword::Var)?;
        let mut names = vec![self.expect_ident()?];
        while self.eat(&TokenKind::Comma) {
            names.push(self.expect_ident()?);
        }
        Ok(VarDecl { names })
    }

    // --- Операторы ----------------------------------------------------------

    fn parse_block(&mut self, end: &[Keyword]) -> Result<Vec<Stmt>, ParseError> {
        let mut stmts = Vec::new();
        self.skip_semicolons();
        while !self.at_eof() && !self.at_any_keyword(end) {
            stmts.push(self.parse_stmt()?);
            self.skip_semicolons();
        }
        Ok(stmts)
    }

    fn parse_stmt(&mut self) -> Result<Stmt, ParseError> {
        if self.at_keyword(Keyword::If) {
            self.parse_if()
        } else if self.at_keyword(Keyword::While) {
            self.parse_while()
        } else if self.at_keyword(Keyword::For) {
            self.parse_for()
        } else if self.at_keyword(Keyword::Try) {
            self.parse_try()
        } else if self.eat_keyword(Keyword::Return) {
            let value = if self.at_stmt_boundary() {
                None
            } else {
                Some(self.parse_expr()?)
            };
            Ok(Stmt::Return(value))
        } else if self.eat_keyword(Keyword::Break) {
            Ok(Stmt::Break)
        } else if self.eat_keyword(Keyword::Continue) {
            Ok(Stmt::Continue)
        } else if self.eat_keyword(Keyword::Raise) {
            let value = if self.at_stmt_boundary() {
                None
            } else {
                Some(self.parse_expr()?)
            };
            Ok(Stmt::Raise(value))
        } else if self.at_keyword(Keyword::Var) {
            Ok(Stmt::VarDecl(self.parse_var_decl()?))
        } else if self.eat_keyword(Keyword::Execute) {
            self.expect_lparen()?;
            let expr = self.parse_expr()?;
            self.expect_rparen()?;
            Ok(Stmt::Execute(expr))
        } else {
            self.parse_simple_stmt()
        }
    }

    fn parse_if(&mut self) -> Result<Stmt, ParseError> {
        self.expect_keyword(Keyword::If)?;
        let cond = self.parse_expr()?;
        self.expect_keyword(Keyword::Then)?;
        let then_branch = self.parse_block(&[Keyword::ElsIf, Keyword::Else, Keyword::EndIf])?;

        let mut elsif_branches = Vec::new();
        while self.eat_keyword(Keyword::ElsIf) {
            let c = self.parse_expr()?;
            self.expect_keyword(Keyword::Then)?;
            let b = self.parse_block(&[Keyword::ElsIf, Keyword::Else, Keyword::EndIf])?;
            elsif_branches.push((c, b));
        }

        let else_branch = if self.eat_keyword(Keyword::Else) {
            Some(self.parse_block(&[Keyword::EndIf])?)
        } else {
            None
        };

        self.expect_keyword(Keyword::EndIf)?;
        Ok(Stmt::If {
            cond,
            then_branch,
            elsif_branches,
            else_branch,
        })
    }

    fn parse_while(&mut self) -> Result<Stmt, ParseError> {
        self.expect_keyword(Keyword::While)?;
        let cond = self.parse_expr()?;
        self.expect_keyword(Keyword::Do)?;
        let body = self.parse_block(&[Keyword::EndDo])?;
        self.expect_keyword(Keyword::EndDo)?;
        Ok(Stmt::While { cond, body })
    }

    fn parse_for(&mut self) -> Result<Stmt, ParseError> {
        self.expect_keyword(Keyword::For)?;
        if self.eat_keyword(Keyword::Each) {
            let var = self.expect_ident()?;
            self.expect_keyword(Keyword::In)?;
            let iter = self.parse_expr()?;
            self.expect_keyword(Keyword::Do)?;
            let body = self.parse_block(&[Keyword::EndDo])?;
            self.expect_keyword(Keyword::EndDo)?;
            Ok(Stmt::ForEach { var, iter, body })
        } else {
            let var = self.expect_ident()?;
            self.expect(&TokenKind::Eq)?;
            let from = self.parse_expr()?;
            self.expect_keyword(Keyword::To)?;
            let to = self.parse_expr()?;
            self.expect_keyword(Keyword::Do)?;
            let body = self.parse_block(&[Keyword::EndDo])?;
            self.expect_keyword(Keyword::EndDo)?;
            Ok(Stmt::ForNumeric {
                var,
                from,
                to,
                body,
            })
        }
    }

    fn parse_try(&mut self) -> Result<Stmt, ParseError> {
        self.expect_keyword(Keyword::Try)?;
        let body = self.parse_block(&[Keyword::Except])?;
        self.expect_keyword(Keyword::Except)?;
        let except_body = self.parse_block(&[Keyword::EndTry])?;
        self.expect_keyword(Keyword::EndTry)?;
        Ok(Stmt::Try { body, except_body })
    }

    /// Присваивание либо вызов-как-оператор. Левая часть разбирается только
    /// до постфиксной цепочки (без сравнений и бинарных операций) — иначе
    /// `x = 5` разобрался бы как выражение сравнения `x = 5`, а не как
    /// присваивание: `=` в BSL один и тот же токен для обоих случаев,
    /// разница только в позиции.
    fn parse_simple_stmt(&mut self) -> Result<Stmt, ParseError> {
        let expr = self.parse_postfix()?;
        if self.eat(&TokenKind::Eq) {
            let value = self.parse_expr()?;
            Ok(Stmt::Assign { target: expr, value })
        } else {
            Ok(Stmt::ExprStmt(expr))
        }
    }

    // --- Выражения ------------------------------------------------------
    //
    // Приоритет от слабого к сильному: Или, И, Не, сравнения, `+ -`, `* /`,
    // унарный минус, постфиксная цепочка (`.`, `[]`, `()`), первичное.

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_and()?;
        while self.eat_keyword(Keyword::Or) {
            let rhs = self.parse_and()?;
            lhs = Expr::Binary {
                op: BinaryOp::Or,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_and(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_not()?;
        while self.eat_keyword(Keyword::And) {
            let rhs = self.parse_not()?;
            lhs = Expr::Binary {
                op: BinaryOp::And,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_not(&mut self) -> Result<Expr, ParseError> {
        if self.eat_keyword(Keyword::Not) {
            let expr = self.parse_not()?;
            Ok(Expr::Unary {
                op: UnaryOp::Not,
                expr: Box::new(expr),
            })
        } else {
            self.parse_comparison()
        }
    }

    fn parse_comparison(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_additive()?;
        loop {
            let op = match self.peek().kind {
                TokenKind::Eq => BinaryOp::Eq,
                TokenKind::NotEq => BinaryOp::NotEq,
                TokenKind::Lt => BinaryOp::Lt,
                TokenKind::Gt => BinaryOp::Gt,
                TokenKind::Le => BinaryOp::Le,
                TokenKind::Ge => BinaryOp::Ge,
                _ => break,
            };
            self.bump();
            let rhs = self.parse_additive()?;
            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_additive(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_multiplicative()?;
        loop {
            let op = match self.peek().kind {
                TokenKind::Plus => BinaryOp::Add,
                TokenKind::Minus => BinaryOp::Sub,
                _ => break,
            };
            self.bump();
            let rhs = self.parse_multiplicative()?;
            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_unary()?;
        loop {
            let op = match self.peek().kind {
                TokenKind::Star => BinaryOp::Mul,
                TokenKind::Slash => BinaryOp::Div,
                _ => break,
            };
            self.bump();
            let rhs = self.parse_unary()?;
            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        if self.eat(&TokenKind::Minus) {
            let expr = self.parse_unary()?;
            Ok(Expr::Unary {
                op: UnaryOp::Neg,
                expr: Box::new(expr),
            })
        } else {
            self.parse_postfix()
        }
    }

    fn parse_postfix(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_primary()?;
        loop {
            if self.eat(&TokenKind::Dot) {
                let name = self.expect_member_name()?;
                expr = Expr::Field {
                    obj: Box::new(expr),
                    name,
                };
            } else if self.eat(&TokenKind::LBracket) {
                let index = self.parse_expr()?;
                self.expect(&TokenKind::RBracket)?;
                expr = Expr::Index {
                    obj: Box::new(expr),
                    index: Box::new(index),
                };
            } else if self.eat(&TokenKind::LParen) {
                let args = self.parse_call_args()?;
                expr = Expr::Call {
                    callee: Box::new(expr),
                    args,
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    /// Аргументы вызова; `None` на месте пропущенного аргумента: `Ф(1, , 3)`.
    fn parse_call_args(&mut self) -> Result<Vec<Option<Expr>>, ParseError> {
        let mut args = Vec::new();
        if self.eat(&TokenKind::RParen) {
            return Ok(args);
        }
        loop {
            if self.at(&TokenKind::Comma) || self.at(&TokenKind::RParen) {
                args.push(None);
            } else {
                args.push(Some(self.parse_expr()?));
            }
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        self.expect_rparen()?;
        Ok(args)
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        let tok = self.peek().clone();
        match tok.kind {
            TokenKind::Number => {
                self.bump();
                Ok(Expr::Number(self.text(tok.span).to_string()))
            }
            TokenKind::String(s) => {
                self.bump();
                Ok(Expr::Str(s))
            }
            TokenKind::Date(d) => {
                self.bump();
                Ok(Expr::Date(d))
            }
            TokenKind::Keyword(Keyword::True) => {
                self.bump();
                Ok(Expr::Bool(true))
            }
            TokenKind::Keyword(Keyword::False) => {
                self.bump();
                Ok(Expr::Bool(false))
            }
            TokenKind::Keyword(Keyword::Undefined) => {
                self.bump();
                Ok(Expr::Undefined)
            }
            TokenKind::Keyword(Keyword::Null) => {
                self.bump();
                Ok(Expr::Null)
            }
            TokenKind::Keyword(Keyword::New) => self.parse_new(),
            TokenKind::Ident => {
                self.bump();
                Ok(Expr::Ident(self.text(tok.span).to_string()))
            }
            TokenKind::LParen => {
                self.bump();
                let e = self.parse_expr()?;
                self.expect_rparen()?;
                Ok(e)
            }
            TokenKind::Question => self.parse_ternary(),
            other => Err(self.error_here(format!("неожиданный токен в выражении: {other:?}"))),
        }
    }

    fn parse_new(&mut self) -> Result<Expr, ParseError> {
        self.expect_keyword(Keyword::New)?;
        let type_name = self.expect_ident()?;
        let args = if self.eat(&TokenKind::LParen) {
            if self.eat(&TokenKind::RParen) {
                Vec::new()
            } else {
                let mut args = vec![self.parse_expr()?];
                while self.eat(&TokenKind::Comma) {
                    args.push(self.parse_expr()?);
                }
                self.expect_rparen()?;
                args
            }
        } else {
            Vec::new()
        };
        Ok(Expr::New { type_name, args })
    }

    fn parse_ternary(&mut self) -> Result<Expr, ParseError> {
        self.expect(&TokenKind::Question)?;
        self.expect_lparen()?;
        let cond = self.parse_expr()?;
        self.expect(&TokenKind::Comma)?;
        let then_expr = self.parse_expr()?;
        self.expect(&TokenKind::Comma)?;
        let else_expr = self.parse_expr()?;
        self.expect_rparen()?;
        Ok(Expr::Ternary {
            cond: Box::new(cond),
            then_expr: Box::new(then_expr),
            else_expr: Box::new(else_expr),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ok(src: &str) -> Program {
        parse(src).unwrap_or_else(|e| panic!("parse error on {src:?}: {e:?}"))
    }

    #[test]
    fn implicit_assignment_no_var_needed() {
        let prog = parse_ok("PI = 3.14;");
        assert_eq!(
            prog.items,
            vec![Item::Stmt(Stmt::Assign {
                target: Expr::Ident("PI".into()),
                value: Expr::Number("3.14".into()),
            })]
        );
    }

    #[test]
    fn function_without_return_is_callable_as_statement() {
        let prog = parse_ok(
            "Функция Ф()\nКонецФункции\nФ();",
        );
        assert!(matches!(prog.items[0], Item::Function(_)));
        assert_eq!(
            prog.items[1],
            Item::Stmt(Stmt::ExprStmt(Expr::Call {
                callee: Box::new(Expr::Ident("Ф".into())),
                args: vec![],
            }))
        );
    }

    #[test]
    fn assignment_vs_equality_disambiguated_by_position() {
        // `x = 5;` присваивание, `Если x = 5 Тогда` — сравнение.
        let prog = parse_ok("x = 5;\nЕсли x = 5 Тогда\ny = 1;\nКонецЕсли;");
        assert_eq!(
            prog.items[0],
            Item::Stmt(Stmt::Assign {
                target: Expr::Ident("x".into()),
                value: Expr::Number("5".into()),
            })
        );
        match &prog.items[1] {
            Item::Stmt(Stmt::If { cond, .. }) => {
                assert_eq!(
                    *cond,
                    Expr::Binary {
                        op: BinaryOp::Eq,
                        lhs: Box::new(Expr::Ident("x".into())),
                        rhs: Box::new(Expr::Number("5".into())),
                    }
                );
            }
            other => panic!("expected If, got {other:?}"),
        }
    }

    #[test]
    fn lvalue_is_a_path_not_a_name() {
        let prog = parse_ok("bodies[0].vx = 1;");
        let target = match &prog.items[0] {
            Item::Stmt(Stmt::Assign { target, .. }) => target.clone(),
            other => panic!("expected Assign, got {other:?}"),
        };
        assert_eq!(
            target,
            Expr::Field {
                obj: Box::new(Expr::Index {
                    obj: Box::new(Expr::Ident("bodies".into())),
                    index: Box::new(Expr::Number("0".into())),
                }),
                name: "vx".into(),
            }
        );
    }

    #[test]
    fn unary_minus_binds_tighter_than_division() {
        // `- px / SOLAR_MASS` == `(-px) / SOLAR_MASS`, не `-(px / SOLAR_MASS)`.
        let prog = parse_ok("y = - px / SOLAR_MASS;");
        match &prog.items[0] {
            Item::Stmt(Stmt::Assign { value, .. }) => {
                assert_eq!(
                    *value,
                    Expr::Binary {
                        op: BinaryOp::Div,
                        lhs: Box::new(Expr::Unary {
                            op: UnaryOp::Neg,
                            expr: Box::new(Expr::Ident("px".into())),
                        }),
                        rhs: Box::new(Expr::Ident("SOLAR_MASS".into())),
                    }
                );
            }
            other => panic!("expected Assign, got {other:?}"),
        }
    }

    #[test]
    fn skipped_arguments_are_none() {
        let prog = parse_ok("Ф(1, , 3);");
        assert_eq!(
            prog.items[0],
            Item::Stmt(Stmt::ExprStmt(Expr::Call {
                callee: Box::new(Expr::Ident("Ф".into())),
                args: vec![
                    Some(Expr::Number("1".into())),
                    None,
                    Some(Expr::Number("3".into())),
                ],
            }))
        );
    }

    #[test]
    fn const_is_not_a_reserved_word() {
        // `const` не входит в таблицу ключевых слов — используется как обычный
        // идентификатор параметра/переменной.
        let prog = parse_ok("Функция Ф(const)\nВозврат const;\nКонецФункции");
        match &prog.items[0] {
            Item::Function(f) => {
                assert_eq!(f.params[0].name, "const");
            }
            other => panic!("expected Function, got {other:?}"),
        }
    }

    #[test]
    fn for_each_and_numeric_for() {
        let prog = parse_ok("Для Каждого b Из bodies Цикл\nКонецЦикла;");
        assert!(matches!(
            prog.items[0],
            Item::Stmt(Stmt::ForEach { .. })
        ));

        let prog = parse_ok("Для i = 0 По 10 Цикл\nКонецЦикла");
        assert!(matches!(
            prog.items[0],
            Item::Stmt(Stmt::ForNumeric { .. })
        ));
    }

    #[test]
    fn new_expression_with_and_without_args() {
        let prog = parse_ok("a = Новый Массив(3, 4);");
        match &prog.items[0] {
            Item::Stmt(Stmt::Assign { value, .. }) => assert_eq!(
                *value,
                Expr::New {
                    type_name: "Массив".into(),
                    args: vec![Expr::Number("3".into()), Expr::Number("4".into())],
                }
            ),
            other => panic!("expected Assign, got {other:?}"),
        }

        let prog = parse_ok("a = Новый Массив;");
        match &prog.items[0] {
            Item::Stmt(Stmt::Assign { value, .. }) => assert_eq!(
                *value,
                Expr::New {
                    type_name: "Массив".into(),
                    args: vec![],
                }
            ),
            other => panic!("expected Assign, got {other:?}"),
        }
    }

    #[test]
    fn ternary_operator() {
        let prog = parse_ok("a = ?(x > 0, 1, -1);");
        match &prog.items[0] {
            Item::Stmt(Stmt::Assign { value, .. }) => assert!(matches!(value, Expr::Ternary { .. })),
            other => panic!("expected Assign, got {other:?}"),
        }
    }

    #[test]
    fn optional_semicolons_before_block_enders() {
        // Точка с запятой перед КонецЦикла/КонецЕсли не обязательна.
        let prog = parse_ok("Пока Истина Цикл\nx = 1\nКонецЦикла");
        assert!(matches!(prog.items[0], Item::Stmt(Stmt::While { .. })));
    }

    #[test]
    fn return_without_expression_is_undefined() {
        let prog = parse_ok("Процедура П()\nВозврат;\nКонецПроцедуры");
        match &prog.items[0] {
            Item::Procedure(p) => assert_eq!(p.body, vec![Stmt::Return(None)]),
            other => panic!("expected Procedure, got {other:?}"),
        }
    }

    #[test]
    fn try_except() {
        let prog = parse_ok("Попытка\nx = 1;\nИсключение\ny = 2;\nКонецПопытки");
        assert!(matches!(prog.items[0], Item::Stmt(Stmt::Try { .. })));
    }
}
