use crate::ast::{BinaryOp, Expr, ForSource, Program, Stmt};
use crate::func::eval_builtin;
use crate::lexer::{tokenize, Token, TokenKind};
use crate::{format_hex_rgba, parse_hex_color, EvalContext};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Color([u8; 4]),
    Str(String),
    List(Vec<Value>),
    Nil,
}

enum ControlFlow {
    Next,
    Break,
    Continue,
}

impl Value {
    pub fn as_f64(&self, line: usize) -> Result<f64, String> {
        match self {
            Value::Int(i) => Ok(*i as f64),
            Value::Float(f) => Ok(*f),
            other => Err(format!("Linia {line}: așteptat număr, primit {:?}", other)),
        }
    }

    pub fn as_u32_coord(&self, name: &str, line: usize) -> Result<u32, String> {
        match self {
            Value::Int(i) if *i >= 0 => Ok(*i as u32),
            Value::Float(f) if *f >= 0.0 && f.fract() == 0.0 => Ok(*f as u32),
            other => Err(format!(
                "Linia {line}: parametrul '{name}' trebuie să fie întreg >= 0, primit {:?}",
                other
            )),
        }
    }

    pub fn as_color(&self, line: usize) -> Result<[u8; 4], String> {
        match self {
            Value::Color(c) => Ok(*c),
            Value::Str(s) => parse_hex_color(s).map_err(|e| format!("Linia {line}: {e}")),
            other => Err(format!("Linia {line}: așteptat culoare, primit {:?}", other)),
        }
    }

    pub fn as_point_list(&self, line: usize) -> Result<Vec<(u32, u32)>, String> {
        let Value::List(items) = self else {
            return Err(format!(
                "Linia {line}: așteptată listă de coordonate [[x, y], ...], primit {:?}",
                self
            ));
        };

        let mut points = Vec::with_capacity(items.len());
        for item in items {
            let Value::List(pair) = item else {
                return Err(format!(
                    "Linia {line}: fiecare element din listă trebuie să fie o pereche [x, y], primit {:?}",
                    item
                ));
            };
            if pair.len() != 2 {
                return Err(format!(
                    "Linia {line}: perechea de coordonate trebuie să aibă exact 2 elemente [x, y], primit {}",
                    pair.len()
                ));
            }
            let x = pair[0].as_u32_coord("x", line)?;
            let y = pair[1].as_u32_coord("y", line)?;
            points.push((x, y));
        }
        Ok(points)
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Int(i) => *i != 0,
            Value::Float(f) => *f != 0.0 && !f.is_nan(),
            Value::Str(s) => !s.is_empty(),
            Value::Color(c) => c[3] > 0,
            Value::List(l) => !l.is_empty(),
            Value::Nil => false,
        }
    }

    pub fn format_for_echo(&self) -> String {
        match self {
            Value::Int(i) => i.to_string(),
            Value::Float(f) => f.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Color(c) => format_hex_rgba(*c),
            Value::Str(s) => s.clone(),
            Value::List(items) => {
                let inner: Vec<String> = items.iter().map(Value::format_for_echo).collect();
                format!("[{}]", inner.join(", "))
            }
            Value::Nil => "nil".to_string(),
        }
    }
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn current(&self) -> &Token {
        self.tokens
            .get(self.pos)
            .unwrap_or_else(|| self.tokens.last().unwrap())
    }

    fn peek(&self, offset: usize) -> &Token {
        self.tokens
            .get(self.pos + offset)
            .unwrap_or_else(|| self.tokens.last().unwrap())
    }

    fn advance(&mut self) -> Token {
        let tok = self.current().clone();
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    fn skip_separators(&mut self) {
        while matches!(self.current().kind, TokenKind::Semicolon) {
            self.advance();
        }
    }

    fn parse_program(&mut self) -> Result<Program, String> {
        let mut statements = Vec::new();
        loop {
            self.skip_separators();
            if matches!(self.current().kind, TokenKind::Eof) {
                break;
            }
            statements.push(self.parse_stmt()?);
        }
        Ok(Program { statements })
    }

    fn parse_stmt(&mut self) -> Result<Stmt, String> {
        let tok = self.current().clone();
        match tok.kind {
            TokenKind::Let => {
                self.advance();
                let name = match self.advance().kind {
                    TokenKind::Ident(n) => n,
                    _ => {
                        return Err(format!(
                            "Linia {}: așteptat nume de variabilă după 'let'",
                            tok.line
                        ))
                    }
                };
                if !matches!(self.advance().kind, TokenKind::Eq) {
                    return Err(format!(
                        "Linia {}: așteptat '=' după variabila '{name}'",
                        tok.line
                    ));
                }
                let value = self.parse_expr()?;
                self.skip_separators();
                Ok(Stmt::Let { name, value })
            }
            TokenKind::Ident(ref name) if matches!(self.peek(1).kind, TokenKind::Eq) => {
                let name = name.clone();
                self.advance(); // consumă Ident
                self.advance(); // consumă '='
                let value = self.parse_expr()?;
                self.skip_separators();
                Ok(Stmt::Assign { name, value })
            }
            TokenKind::Ident(ref name) if matches!(self.peek(1).kind, TokenKind::LBracket) => {
                let name = name.clone();
                let stmt_line = self.current().line;
                let saved_pos = self.pos;
                self.advance(); // consumă Ident

                let mut indices = Vec::new();
                while matches!(self.current().kind, TokenKind::LBracket) {
                    let bracket_line = self.current().line;
                    self.advance(); // consumă '['
                    indices.push(self.parse_expr()?);
                    if !matches!(self.advance().kind, TokenKind::RBracket) {
                        return Err(format!(
                            "Linia {bracket_line}: așteptat ']' la asignarea pe index"
                        ));
                    }
                }

                if matches!(self.current().kind, TokenKind::Eq) {
                    self.advance(); // consumă '='
                    let value = self.parse_expr()?;
                    self.skip_separators();
                    Ok(Stmt::IndexAssign {
                        target: name,
                        indices,
                        value,
                        line: stmt_line,
                    })
                } else {
                    self.pos = saved_pos;
                    let expr = self.parse_expr()?;
                    self.skip_separators();
                    Ok(Stmt::ExprStmt(expr))
                }
            }
            TokenKind::If => self.parse_if(),
            TokenKind::While => self.parse_while(),
            TokenKind::For => self.parse_for(),
            TokenKind::Break => {
                self.advance();
                self.skip_separators();
                Ok(Stmt::Break)
            }
            TokenKind::Continue => {
                self.advance();
                self.skip_separators();
                Ok(Stmt::Continue)
            }
            _ => {
                let expr = self.parse_expr()?;
                self.skip_separators();
                Ok(Stmt::ExprStmt(expr))
            }
        }
    }

    fn parse_if(&mut self) -> Result<Stmt, String> {
        let if_tok = self.advance();
        let condition = self.parse_expr()?;
        let then_branch = self.parse_block()?;

        let mut else_branch = None;
        if matches!(self.current().kind, TokenKind::Else) {
            self.advance();
            if matches!(self.current().kind, TokenKind::If) {
                let nested_if = self.parse_if()?;
                else_branch = Some(vec![nested_if]);
            } else {
                else_branch = Some(self.parse_block()?);
            }
        }

        self.skip_separators();
        Ok(Stmt::If {
            condition,
            then_branch,
            else_branch,
        })
        .map_err(|e: String| format!("Linia {}: {e}", if_tok.line))
    }

    fn parse_while(&mut self) -> Result<Stmt, String> {
        let while_tok = self.advance(); // consumă 'while'
        let condition = self.parse_expr()?;
        let body = self.parse_block()?;
        self.skip_separators();
        Ok(Stmt::While { condition, body })
            .map_err(|e: String| format!("Linia {}: {e}", while_tok.line))
    }

    fn parse_for(&mut self) -> Result<Stmt, String> {
        let for_tok = self.advance(); // consumă 'for'
        let var = match self.advance().kind {
            TokenKind::Ident(name) => name,
            _ => {
                return Err(format!(
                    "Linia {}: așteptat nume de variabilă după 'for'",
                    for_tok.line
                ))
            }
        };

        if !matches!(self.advance().kind, TokenKind::In) {
            return Err(format!(
                "Linia {}: așteptat 'in' după variabila '{var}' în bucla 'for'",
                for_tok.line
            ));
        }

        let first_expr = self.parse_expr()?;
        let source = if matches!(self.current().kind, TokenKind::DotDot | TokenKind::DotDotEq) {
            let inclusive = matches!(self.advance().kind, TokenKind::DotDotEq);
            let end_expr = self.parse_expr()?;
            ForSource::Range {
                start: first_expr,
                end: end_expr,
                inclusive,
            }
        } else {
            ForSource::Collection(first_expr)
        };

        let body = self.parse_block()?;
        self.skip_separators();
        Ok(Stmt::For { var, source, body })
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, String> {
        let lbrace = self.current().clone();
        if !matches!(lbrace.kind, TokenKind::LBrace) {
            return Err(format!(
                "Linia {}: așteptat '{{' pentru deschiderea blocului",
                lbrace.line
            ));
        }
        self.advance();

        let mut stmts = Vec::new();
        loop {
            self.skip_separators();
            let cur = self.current().clone();
            match cur.kind {
                TokenKind::RBrace => {
                    self.advance();
                    break;
                }
                TokenKind::Eof => {
                    return Err(format!(
                        "Linia {}: bloc '{{' neînchis cu '}}'",
                        lbrace.line
                    ));
                }
                _ => stmts.push(self.parse_stmt()?),
            }
        }
        Ok(stmts)
    }

    fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_and()?;
        while matches!(self.current().kind, TokenKind::OrOr) {
            self.advance();
            let right = self.parse_and()?;
            expr = Expr::Binary {
                op: BinaryOp::Or,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_and(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_equality()?;
        while matches!(self.current().kind, TokenKind::AndAnd) {
            self.advance();
            let right = self.parse_equality()?;
            expr = Expr::Binary {
                op: BinaryOp::And,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_equality(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_relational()?;
        loop {
            let tok = self.current().clone();
            let op = match tok.kind {
                TokenKind::EqEq => BinaryOp::EqEq,
                TokenKind::BangEq => BinaryOp::BangEq,
                _ => break,
            };
            self.advance();
            let right = self.parse_relational()?;
            expr = Expr::Binary {
                op,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_relational(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_additive()?;
        loop {
            let tok = self.current().clone();
            let op = match tok.kind {
                TokenKind::Lt => BinaryOp::Lt,
                TokenKind::LtEq => BinaryOp::LtEq,
                TokenKind::Gt => BinaryOp::Gt,
                TokenKind::GtEq => BinaryOp::GtEq,
                _ => break,
            };
            self.advance();
            let right = self.parse_additive()?;
            expr = Expr::Binary {
                op,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_additive(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_multiplicative()?;
        loop {
            let tok = self.current().clone();
            let op = match tok.kind {
                TokenKind::Plus => BinaryOp::Add,
                TokenKind::Minus => BinaryOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplicative()?;
            expr = Expr::Binary {
                op,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_unary()?;
        loop {
            let tok = self.current().clone();
            let op = match tok.kind {
                TokenKind::Star => BinaryOp::Mul,
                TokenKind::Slash => BinaryOp::Div,
                TokenKind::Percent => BinaryOp::Rem,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            expr = Expr::Binary {
                op,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        let tok = self.current().clone();
        if matches!(tok.kind, TokenKind::Minus) {
            self.advance();
            let inner = self.parse_unary()?;
            return Ok(Expr::UnaryMinus(Box::new(inner)));
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        let tok = self.current().clone();
        let mut expr = match &tok.kind {
            TokenKind::Int(v) => {
                self.advance();
                Expr::Int(*v)
            }
            TokenKind::Float(v) => {
                self.advance();
                Expr::Float(*v)
            }
            TokenKind::Str(s) => {
                self.advance();
                Expr::Str(s.clone())
            }
            // Păstrează aici ramurile tale exacte din lexer.rs / ast.rs, de exemplu:
            // TokenKind::HexColor(...) => { ... }
            // TokenKind::True / TokenKind::False => { ... }
            // TokenKind::Ident(name) => { ... apel funcție sau Expr::Var / Expr::Ident ... }
            TokenKind::LBracket => {
                let bracket_line = tok.line;
                self.advance(); // '['
                let mut items = Vec::new();
                while !matches!(self.current().kind, TokenKind::RBracket | TokenKind::Eof) {
                    items.push(self.parse_expr()?);
                    if matches!(self.current().kind, TokenKind::Comma) {
                        self.advance();
                    } else {
                        break;
                    }
                }
                if !matches!(self.current().kind, TokenKind::RBracket) {
                    return Err(format!(
                        "Linia {bracket_line}: așteptat ']' la finalul listei"
                    ));
                }
                self.advance(); // ']'
                Expr::List(items)
            }
            TokenKind::LParen => {
                self.advance(); // '('
                let inner = self.parse_expr()?;
                if !matches!(self.advance().kind, TokenKind::RParen) {
                    return Err(format!("Linia {}: așteptat ')'", tok.line));
                }
                inner
            }
            TokenKind::HexColor(hex) => {
                self.advance();
                let rgba = parse_hex_color(hex)
                    .map_err(|e| format!("Linia {}: {e}", tok.line))?;
                Expr::Color(rgba)
            }
            TokenKind::Ident(name) => {
                let name = name.clone();
                self.advance();
                if matches!(self.current().kind, TokenKind::LParen) {
                    self.advance(); // '('
                    let mut args = Vec::new();
                    while !matches!(self.current().kind, TokenKind::RParen | TokenKind::Eof) {
                        args.push(self.parse_expr()?);
                        if matches!(self.current().kind, TokenKind::Comma) {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    if !matches!(self.advance().kind, TokenKind::RParen) {
                        return Err(format!(
                            "Linia {}: așteptat ')' după argumentele funcției '{name}'",
                            tok.line
                        ));
                    }
                    Expr::Call { name, args }
                } else {
                    Expr::Var(name)
                }
            }
            other => {
                return Err(format!(
                    "Linia {}: expresie neașteptată {:?}",
                    tok.line, other
                ))
            }
        };

        while matches!(self.current().kind, TokenKind::LBracket) {
            let bracket_line = self.current().line;
            self.advance(); // '['
            let index = self.parse_expr()?;
            if !matches!(self.advance().kind, TokenKind::RBracket) {
                return Err(format!("Linia {bracket_line}: așteptat ']' la indexare"));
            }
            expr = Expr::Index {
                target: Box::new(expr),
                index: Box::new(index),
            };
        }

        Ok(expr)
    }
}

pub fn eval_program(source: &str, ctx: &mut EvalContext<'_>) -> Result<(), String> {
    let tokens = tokenize(source)?;
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program()?;

    for stmt in &program.statements {
        match eval_stmt(stmt, ctx, 1)? {
            ControlFlow::Next => {}
            ControlFlow::Break => {
                return Err("'break' folosit în afara unei bucle".to_string());
            }
            ControlFlow::Continue => {
                return Err("'continue' folosit în afara unei bucle".to_string());
            }
        }
    }
    Ok(())
}

fn eval_block(stmts: &[Stmt], ctx: &mut EvalContext<'_>, line: usize) -> Result<ControlFlow, String> {
    for s in stmts {
        match eval_stmt(s, ctx, line)? {
            ControlFlow::Next => {}
            cf @ (ControlFlow::Break | ControlFlow::Continue) => return Ok(cf),
        }
    }
    Ok(ControlFlow::Next)
}

fn eval_stmt(stmt: &Stmt, ctx: &mut EvalContext<'_>, line: usize) -> Result<ControlFlow, String> {
    match stmt {
        Stmt::Let { name, value } => {
            let val = eval_expr(value, ctx, line)?;
            ctx.vars.insert(name.clone(), val);
            Ok(ControlFlow::Next)
        }
        Stmt::Assign { name, value } => {
            if !ctx.vars.contains_key(name) {
                return Err(format!(
                    "Linia {line}: variabilă nedefinită '{name}' la asignare (folosește 'let {name} = ...')"
                ));
            }
            let val = eval_expr(value, ctx, line)?;
            ctx.vars.insert(name.clone(), val);
            Ok(ControlFlow::Next)
        }
        Stmt::IndexAssign {
            target,
            indices,
            value,
            line: stmt_line,
        } => {
            let l = *stmt_line;
            let new_val = eval_expr(value, ctx, l)?;

            let mut resolved_indices = Vec::with_capacity(indices.len());
            for idx_expr in indices {
                let idx_val = eval_expr(idx_expr, ctx, l)?;
                match idx_val {
                    Value::Int(i) if i >= 0 => resolved_indices.push(i as usize),
                    Value::Int(i) => {
                        return Err(format!(
                            "Linia {l}: index negativ {i} la asignarea în listă"
                        ));
                    }
                    other => {
                        return Err(format!(
                            "Linia {l}: indexul listei trebuie să fie întreg, primit {:?}",
                            other
                        ));
                    }
                }
            }

            let root = ctx.vars.get_mut(target).ok_or_else(|| {
                format!(
                    "Linia {l}: variabilă nedefinită '{target}' la asignare (folosește 'let {target} = ...')"
                )
            })?;

            assign_at_indices(root, &resolved_indices, new_val, l)?;
            Ok(ControlFlow::Next)
        }
        Stmt::If {
            condition,
            then_branch,
            else_branch,
        } => {
            let cond_val = eval_expr(condition, ctx, line)?;
            if cond_val.is_truthy() {
                eval_block(then_branch, ctx, line)
            } else if let Some(else_stmts) = else_branch {
                eval_block(else_stmts, ctx, line)
            } else {
                Ok(ControlFlow::Next)
            }
        }
        Stmt::While { condition, body } => {
            let mut steps = 0_usize;
            const MAX_WHILE_STEPS: usize = 10_000_000;
            while eval_expr(condition, ctx, line)?.is_truthy() {
                steps += 1;
                if steps > MAX_WHILE_STEPS {
                    return Err(format!(
                        "Linia {line}: bucla while a depășit limita de {MAX_WHILE_STEPS} iterații"
                    ));
                }
                match eval_block(body, ctx, line)? {
                    ControlFlow::Next | ControlFlow::Continue => {}
                    ControlFlow::Break => break,
                }
            }
            Ok(ControlFlow::Next)
        }
        Stmt::For { var, source, body } => {
            let mut steps = 0_usize;
            const MAX_FOR_STEPS: usize = 10_000_000;

            match source {
                ForSource::Range {
                    start,
                    end,
                    inclusive,
                } => {
                    let s_val = eval_expr(start, ctx, line)?;
                    let e_val = eval_expr(end, ctx, line)?;
                    let s_i = match s_val {
                        Value::Int(v) => v,
                        other => {
                            return Err(format!(
                                "Linia {line}: începutul intervalului 'for' trebuie să fie întreg, primit {:?}",
                                other
                            ))
                        }
                    };
                    let e_i = match e_val {
                        Value::Int(v) => v,
                        other => {
                            return Err(format!(
                                "Linia {line}: sfârșitul intervalului 'for' trebuie să fie întreg, primit {:?}",
                                other
                            ))
                        }
                    };

                    let mut cur = s_i;
                    let should_continue = |c: i64| -> bool {
                        if *inclusive {
                            c <= e_i
                        } else {
                            c < e_i
                        }
                    };

                    while should_continue(cur) {
                        steps += 1;
                        if steps > MAX_FOR_STEPS {
                            return Err(format!(
                                "Linia {line}: bucla for a depășit limita de {MAX_FOR_STEPS} iterații"
                            ));
                        }
                        ctx.vars.insert(var.clone(), Value::Int(cur));
                        match eval_block(body, ctx, line)? {
                            ControlFlow::Next | ControlFlow::Continue => {}
                            ControlFlow::Break => break,
                        }
                        cur += 1;
                    }
                }
                ForSource::Collection(expr) => {
                    let col_val = eval_expr(expr, ctx, line)?;
                    let Value::List(items) = col_val else {
                        return Err(format!(
                            "Linia {line}: 'for ... in' așteaptă o listă sau un interval, primit {:?}",
                            col_val
                        ));
                    };

                    for item in items {
                        steps += 1;
                        if steps > MAX_FOR_STEPS {
                            return Err(format!(
                                "Linia {line}: bucla for a depășit limita de {MAX_FOR_STEPS} iterații"
                            ));
                        }
                        ctx.vars.insert(var.clone(), item);
                        match eval_block(body, ctx, line)? {
                            ControlFlow::Next | ControlFlow::Continue => {}
                            ControlFlow::Break => break,
                        }
                    }
                }
            }
            Ok(ControlFlow::Next)
        }
        Stmt::Break => Ok(ControlFlow::Break),
        Stmt::Continue => Ok(ControlFlow::Continue),
        Stmt::ExprStmt(expr) => {
            let val = eval_expr(expr, ctx, line)?;
            if let Expr::Call { name, .. } = expr {
                if name == "get_pixel" {
                    ctx.output.push(val.format_for_echo());
                }
            }
            Ok(ControlFlow::Next)
        }
    }
}

fn assign_at_indices(
    current: &mut Value,
    indices: &[usize],
    new_val: Value,
    line: usize,
) -> Result<(), String> {
    let Some((&first, rest)) = indices.split_first() else {
        *current = new_val;
        return Ok(());
    };

    let items = match current {
        Value::List(list) => list,
        other => {
            return Err(format!(
                "Linia {line}: doar listele pot fi indexate cu [...], primit {:?}",
                other
            ));
        }
    };

    if first >= items.len() {
        return Err(format!(
            "Linia {line}: index {first} în afara limitelor listei de lungime {}",
            items.len()
        ));
    }

    assign_at_indices(&mut items[first], rest, new_val, line)
}

pub(crate) fn eval_expr(expr: &Expr, ctx: &mut EvalContext<'_>, line: usize) -> Result<Value, String> {
    match expr {
        Expr::Int(v) => Ok(Value::Int(*v)),
        Expr::Float(v) => Ok(Value::Float(*v)),
        Expr::Color(c) => Ok(Value::Color(*c)),
        Expr::Str(s) => Ok(Value::Str(s.clone())),
        Expr::Var(name) => ctx
            .vars
            .get(name)
            .cloned()
            .ok_or_else(|| format!("Linia {line}: variabilă nedefinită '{name}'")),
        Expr::List(items) => {
            let mut vals = Vec::with_capacity(items.len());
            for item in items {
                vals.push(eval_expr(item, ctx, line)?);
            }
            Ok(Value::List(vals))
        }
        Expr::Index { target, index } => {
            let target_val = eval_expr(target, ctx, line)?;
            let idx_val = eval_expr(index, ctx, line)?;
            let idx = match idx_val {
                Value::Int(i) if i >= 0 => i as usize,
                Value::Int(i) => {
                    return Err(format!(
                        "Linia {line}: index negativ {i} la indexarea listei"
                    ))
                }
                other => {
                    return Err(format!(
                        "Linia {line}: indexul listei trebuie să fie întreg, primit {:?}",
                        other
                    ))
                }
            };

            match target_val {
                Value::List(items) => {
                    if idx >= items.len() {
                        Err(format!(
                            "Linia {line}: index {idx} în afara limitelor listei de lungime {}",
                            items.len()
                        ))
                    } else {
                        Ok(items[idx].clone())
                    }
                }
                other => Err(format!(
                    "Linia {line}: doar listele pot fi indexate cu [...], primit {:?}",
                    other
                )),
            }
        }
        Expr::UnaryMinus(inner) => {
            let v = eval_expr(inner, ctx, line)?;
            match v {
                Value::Int(i) => Ok(Value::Int(-i)),
                Value::Float(f) => Ok(Value::Float(-f)),
                other => Err(format!(
                    "Linia {line}: '-' unar nu este suportat pentru {:?}",
                    other
                )),
            }
        }
        Expr::Binary { op, left, right } => match op {
            BinaryOp::And => {
                let l = eval_expr(left, ctx, line)?;
                if !l.is_truthy() {
                    return Ok(Value::Bool(false));
                }
                let r = eval_expr(right, ctx, line)?;
                Ok(Value::Bool(r.is_truthy()))
            }
            BinaryOp::Or => {
                let l = eval_expr(left, ctx, line)?;
                if l.is_truthy() {
                    return Ok(Value::Bool(true));
                }
                let r = eval_expr(right, ctx, line)?;
                Ok(Value::Bool(r.is_truthy()))
            }
            _ => {
                let l = eval_expr(left, ctx, line)?;
                let r = eval_expr(right, ctx, line)?;
                eval_binary(op, l, r, line)
            }
        },
        Expr::Call { name, args } => eval_builtin(name, args, ctx, line),
    }
}

fn eval_binary(op: &BinaryOp, l: Value, r: Value, line: usize) -> Result<Value, String> {
    match op {
        BinaryOp::EqEq => Ok(Value::Bool(values_equal(&l, &r))),
        BinaryOp::BangEq => Ok(Value::Bool(!values_equal(&l, &r))),
        BinaryOp::Lt | BinaryOp::LtEq | BinaryOp::Gt | BinaryOp::GtEq => match (l, r) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Bool(match op {
                BinaryOp::Lt => a < b,
                BinaryOp::LtEq => a <= b,
                BinaryOp::Gt => a > b,
                BinaryOp::GtEq => a >= b,
                _ => unreachable!(),
            })),
            (a, b) => {
                let fa = a.as_f64(line)?;
                let fb = b.as_f64(line)?;
                Ok(Value::Bool(match op {
                    BinaryOp::Lt => fa < fb,
                    BinaryOp::LtEq => fa <= fb,
                    BinaryOp::Gt => fa > fb,
                    BinaryOp::GtEq => fa >= fb,
                    _ => unreachable!(),
                }))
            }
        },
        BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem => {
            match (l, r) {
                (Value::Int(a), Value::Int(b)) => match op {
                    BinaryOp::Add => Ok(Value::Int(a + b)),
                    BinaryOp::Sub => Ok(Value::Int(a - b)),
                    BinaryOp::Mul => Ok(Value::Int(a * b)),
                    BinaryOp::Div => {
                        if b == 0 {
                            Err(format!("Linia {line}: împărțire la zero"))
                        } else {
                            Ok(Value::Int(a / b))
                        }
                    }
                    BinaryOp::Rem => {
                        if b == 0 {
                            Err(format!("Linia {line}: modulo la zero"))
                        } else {
                            Ok(Value::Int(a % b))
                        }
                    }
                    _ => unreachable!(),
                },
                (a, b) => {
                    let fa = a.as_f64(line)?;
                    let fb = b.as_f64(line)?;
                    match op {
                        BinaryOp::Add => Ok(Value::Float(fa + fb)),
                        BinaryOp::Sub => Ok(Value::Float(fa - fb)),
                        BinaryOp::Mul => Ok(Value::Float(fa * fb)),
                        BinaryOp::Div => {
                            if fb == 0.0 {
                                Err(format!("Linia {line}: împărțire la zero"))
                            } else {
                                Ok(Value::Float(fa / fb))
                            }
                        }
                        BinaryOp::Rem => {
                            if fb == 0.0 {
                                Err(format!("Linia {line}: modulo la zero"))
                            } else {
                                Ok(Value::Float(fa % fb))
                            }
                        }
                        _ => unreachable!(),
                    }
                }
            }
        },
        BinaryOp::And | BinaryOp::Or => unreachable!(),
    }
}

fn values_equal(l: &Value, r: &Value) -> bool {
    match (l, r) {
        (Value::Int(a), Value::Int(b)) => a == b,
        (Value::Float(a), Value::Float(b)) => (a - b).abs() < f64::EPSILON,
        (Value::Int(a), Value::Float(b)) => ((*a as f64) - b).abs() < f64::EPSILON,
        (Value::Float(a), Value::Int(b)) => (a - (*b as f64)).abs() < f64::EPSILON,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Color(a), Value::Color(b)) => a == b,
        (Value::Str(a), Value::Str(b)) => a == b,
        (Value::List(a), Value::List(b)) => {
            a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| values_equal(x, y))
        }
        (Value::Nil, Value::Nil) => true,
        _ => false,
    }
}

