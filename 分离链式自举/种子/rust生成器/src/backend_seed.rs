// ═══════════════════════════════════════════════════════════════
// Nova 纯血原生 · Rust版Nova编译器
// 真正解析Nova源码 → 生成x86-64机器码
// 支持全部语句类型: 函数/定义/赋值/返回/如果/否则/当/调用/追加/发射
// ═══════════════════════════════════════════════════════════════

use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq)]
enum VarType { Float }

// ═══ 词法分析 ═══

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // 关键字
    Func,           // 函数
    Define,         // 定义
    Mutable,        // 可变
    Return,         // 返回
    If,             // 如果
    Else,           // 否则
    While,          // 当
    Import,         // 导入
    True,           // 真
    False,          // 假
    Null,           // 空
    Struct,         // 结构
    Build,          // 建
    Break,          // 中断
    Continue,       // 继续
    Match,          // 匹配
    Default,        // 默认
    Try,            // 尝试
    Catch,          // 得
    Throw,          // 抛出
    ForEach,        // 每
    PlusEq,         // +=
    MinusEq,        // -=
    Tilde,          // ~
    // 字面量
    Int(i64),
    Float(f64),
    Str(String),
    // 标识符
    Ident(String),
    // 运算符
    Plus, Minus, Star, Slash, Percent,
    Eq, EqEq, NotEq, Lt, Gt, LtEq, GtEq,
    And, Or, Not, BitAnd, BitOr, BitXor,
    Shl, Shr,
    // 分隔符
    LParen, RParen, LBrace, RBrace, LBracket, RBracket,
    Comma, Dot, Colon, Semi,
    // 特殊
    Eof,
}

pub struct Lexer {
    src: Vec<u8>,
    pos: usize,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        let bytes = source.as_bytes();
        // 跳过UTF-8 BOM (EF BB BF)
        let pos = if bytes.len() >= 3 && bytes[0] == 0xEF && bytes[1] == 0xBB && bytes[2] == 0xBF { 3 } else { 0 };
        Lexer { src: bytes.to_vec(), pos }
    }

    fn peek(&self) -> u8 {
        if self.pos < self.src.len() { self.src[self.pos] } else { 0 }
    }

    fn advance(&mut self) -> u8 {
        let b = self.peek();
        if self.pos < self.src.len() { self.pos += 1; }
        b
    }

    fn skip_ws(&mut self) {
        while self.pos < self.src.len() {
            let b = self.src[self.pos];
            if b == b' ' || b == b'\t' || b == b'\r' || b == b'\n' { self.pos += 1; }
            else if b == b'/' && self.pos + 1 < self.src.len() && self.src[self.pos+1] == b'/' {
                while self.pos < self.src.len() && self.src[self.pos] != b'\n' { self.pos += 1; }
            } else { break; }
        }
    }

    /// 读取UTF-8中文标识符或ASCII标识符
    fn read_ident(&mut self) -> String {
        let start = self.pos;
        while self.pos < self.src.len() {
            let b = self.src[self.pos];
            if b >= 0x80 { // UTF-8多字节
                if b >= 0xF0 && self.pos + 3 < self.src.len() { self.pos += 4; }
                else if b >= 0xE0 && self.pos + 2 < self.src.len() { self.pos += 3; }
                else if b >= 0xC0 && self.pos + 1 < self.src.len() { self.pos += 2; }
                else { self.pos += 1; }
            } else if b.is_ascii_alphanumeric() || b == b'_' {
                self.pos += 1;
            } else { break; }
        }
        String::from_utf8_lossy(&self.src[start..self.pos]).to_string()
    }

    fn read_number(&mut self) -> Token {
        let start = self.pos;
        // 检查0x十六进制
        if self.peek() == b'0' && self.pos + 1 < self.src.len() && 
           (self.src[self.pos+1] == b'x' || self.src[self.pos+1] == b'X') {
            self.pos += 2;
            while self.pos < self.src.len() && self.src[self.pos].is_ascii_hexdigit() { self.pos += 1; }
            let s = String::from_utf8_lossy(&self.src[start..self.pos]);
            return Token::Int(i64::from_str_radix(&s[2..], 16).unwrap_or(0));
        }
        while self.pos < self.src.len() && self.src[self.pos].is_ascii_digit() { self.pos += 1; }
        if self.pos < self.src.len() && self.src[self.pos] == b'.' {
            self.pos += 1;
            while self.pos < self.src.len() && self.src[self.pos].is_ascii_digit() { self.pos += 1; }
            let s = String::from_utf8_lossy(&self.src[start..self.pos]);
            return Token::Float(s.parse().unwrap_or(0.0));
        }
        let s = String::from_utf8_lossy(&self.src[start..self.pos]);
        Token::Int(s.parse().unwrap_or(0))
    }

    fn read_string(&mut self) -> Token {
        self.advance(); // skip "
        let mut s = Vec::new();
        while self.pos < self.src.len() && self.src[self.pos] != b'"' {
            if self.src[self.pos] == b'\\' && self.pos + 1 < self.src.len() {
                self.pos += 1;
                match self.src[self.pos] {
                    b'n' => s.push(b'\n'), b'r' => s.push(b'\r'),
                    b't' => s.push(b'\t'), b'\\' => s.push(b'\\'),
                    b'"' => s.push(b'"'), _ => { s.push(b'\\'); s.push(self.src[self.pos]); }
                }
            } else { s.push(self.src[self.pos]); }
            self.pos += 1;
        }
        if self.pos < self.src.len() { self.pos += 1; } // skip "
        Token::Str(String::from_utf8_lossy(&s).to_string())
    }

    pub fn next_token(&mut self) -> Token {
        self.skip_ws();
        if self.pos >= self.src.len() { return Token::Eof; }
        let b = self.src[self.pos];
        
        // 中文关键字(UTF-8 3字节)
        if b >= 0xC0 {
            // 多字节UTF-8标识符(中文关键字/标识符)
            let ident = self.read_ident();
            return match ident.as_str() {
                "函数" | "做法" | "法" => Token::Func,
                "定义" | "称为" | "叫做" => Token::Define,
                "可变" => Token::Mutable,
                "返回" | "给出" => Token::Return,
                "如果" | "假如" | "倘若" | "要是" | "若是" | "每当" | "凡是" => Token::If,
                "否则" | "不然的话" | "否则的话" | "除此之外" => Token::Else,
                "当" | "循环" => Token::While,
                "导入" | "入" => Token::Import,
                "真" => Token::True,
                "假" => Token::False,
                "空" => Token::Null,
                "结构" => Token::Struct,
                "建" => Token::Build,
                "中断" | "止" => Token::Break,
                "继续" | "续" => Token::Continue,
                "匹配" => Token::Match,
                "默认" => Token::Default,
                "尝试" | "试" => Token::Try,
                "得" => Token::Catch,
                "抛出" | "抛" => Token::Throw,
                "每" | "每个" => Token::ForEach,
                "且" | "并且" | "而且" => Token::And,
                "或" | "或者" => Token::Or,
                "非" => Token::Not,
                "等于" => Token::EqEq,
                "不等于" => Token::NotEq,
                "大于" => Token::Gt,
                "小于" => Token::Lt,
                "大于等于" => Token::GtEq,
                "小于等于" => Token::LtEq,
                _ => Token::Ident(ident),
            };
        }
        
        // 数字
        if b.is_ascii_digit() { return self.read_number(); }
        
        // 字符串
        if b == b'"' { return self.read_string(); }
        
        // ASCII标识符
        if b.is_ascii_alphabetic() || b == b'_' {
            let ident = self.read_ident();
            return Token::Ident(ident);
        }
        
        // 运算符和分隔符
        self.pos += 1;
        match b {
            b'+' if self.peek() == b'=' => { self.pos += 1; Token::PlusEq }
            b'+' => Token::Plus,
            b'-' if self.peek() == b'=' => { self.pos += 1; Token::MinusEq }
            b'-' => Token::Minus,
            b'~' => Token::Tilde,
            b'*' => Token::Star, b'/' => Token::Slash, b'%' => Token::Percent,
            b'(' => Token::LParen, b')' => Token::RParen,
            b'{' => Token::LBrace, b'}' => Token::RBrace,
            b'[' => Token::LBracket, b']' => Token::RBracket,
            b',' => Token::Comma, b'.' => Token::Dot,
            b':' => Token::Colon, b';' => Token::Semi,
            b'!' if self.peek() == b'=' => { self.pos += 1; Token::NotEq }
            b'!' => Token::Not,
            b'=' if self.peek() == b'=' => { self.pos += 1; Token::EqEq }
            b'=' => Token::Eq,
            b'<' if self.peek() == b'=' => { self.pos += 1; Token::LtEq }
            b'<' if self.peek() == b'<' => { self.pos += 1; Token::Shl }
            b'<' => Token::Lt,
            b'>' if self.peek() == b'=' => { self.pos += 1; Token::GtEq }
            b'>' if self.peek() == b'>' => { self.pos += 1; Token::Shr }
            b'>' => Token::Gt,
            b'&' if self.peek() == b'&' => { self.pos += 1; Token::And }
            b'&' => Token::BitAnd,
            b'|' if self.peek() == b'|' => { self.pos += 1; Token::Or }
            b'|' => Token::BitOr,
            b'^' => Token::BitXor,
            _ => { Token::Ident(format!("?{}", b as char)) }
        }
    }

    /// 词法分析全部
    pub fn tokenize(source: &str) -> Vec<Token> {
        let mut lexer = Lexer::new(source);
        let mut tokens = Vec::new();
        loop {
            let tok = lexer.next_token();
            if tok == Token::Eof { break; }
            tokens.push(tok);
        }
        tokens
    }
}

// ═══ AST节点 ═══

#[derive(Debug, Clone)]
pub enum Expr {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Null,
    Ident(String),
    Binary(Box<Expr>, BinOp, Box<Expr>),
    Unary(UnaryOp, Box<Expr>),
    Call(String, Vec<Expr>),
    Index(Box<Expr>, Box<Expr>),
    Member(Box<Expr>, String),
    List(Vec<Expr>),
    StructNew(String, Vec<(String, Expr)>),
    Lambda(Vec<String>, Vec<Stmt>),
}

#[derive(Debug, Clone)]
pub enum BinOp {
    Add, Sub, Mul, Div, Mod,
    Eq, Ne, Lt, Gt, Le, Ge,
    And, Or, BitAnd, BitOr, BitXor, Shl, Shr,
}

#[derive(Debug, Clone)]
pub enum UnaryOp { Neg, Not, BitNot }

#[allow(dead_code)] // 保留字段: 未来AST反射/调试/源码重建需要 (2026-04-17 A.4)
#[derive(Debug, Clone)]
pub enum Stmt {
    FuncDef { name: String, params: Vec<String>, body: Vec<Stmt> },
    VarDef { name: String, mutable: bool, init: Option<Expr> },
    Assign { target: Expr, value: Expr },
    Return(Option<Expr>),
    If { cond: Expr, then_body: Vec<Stmt>, else_body: Vec<Stmt> },
    While { cond: Expr, body: Vec<Stmt> },
    ExprStmt(Expr),
    Import(String),
    StructDef { name: String, fields: Vec<String> },
    Break,
    Continue,
    Match { target: Expr, arms: Vec<(Option<Expr>, Vec<Stmt>)> },
    Try { body: Vec<Stmt>, catch_var: Option<String>, catch_body: Vec<Stmt> },
    Throw(Expr),
}

// ═══ Parser ═══

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self { Parser { tokens, pos: 0 } }

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }
    fn advance(&mut self) -> Token {
        let t = self.tokens.get(self.pos).cloned().unwrap_or(Token::Eof);
        self.pos += 1;
        t
    }
    fn expect(&mut self, expected: &Token) -> bool {
        if self.peek() == expected { self.advance(); true } else { false }
    }
    fn at_end(&self) -> bool { self.pos >= self.tokens.len() }

    pub fn parse_program(&mut self) -> Vec<Stmt> {
        let mut stmts = Vec::new();
        while !self.at_end() {
            if let Some(s) = self.parse_stmt() { stmts.push(s); }
        }
        stmts
    }

    fn parse_stmt(&mut self) -> Option<Stmt> {
        match self.peek().clone() {
            Token::Func => self.parse_func_def(),
            // "方法" 不再作为函数定义关键字，因为它常用作参数名/变量名
            Token::Define => self.parse_var_def(),
            Token::Return => self.parse_return(),
            Token::Ident(ref s) if s == "结果" => self.parse_return(),
            Token::If => self.parse_if(),
            Token::While => self.parse_while(),
            Token::Import => self.parse_import(),
            Token::Struct => self.parse_struct_def(),
            Token::Break => { self.advance(); Some(Stmt::Break) }
            Token::Continue => { self.advance(); Some(Stmt::Continue) }
            Token::Match => self.parse_match(),
            Token::Try => self.parse_try(),
            Token::Throw => { self.advance(); Some(Stmt::Throw(self.parse_expr())) }
            Token::RBrace => { self.advance(); None }
            Token::Eof => { self.advance(); None }
            _ => self.parse_expr_or_assign(),
        }
    }

    fn parse_func_def(&mut self) -> Option<Stmt> {
        self.advance(); // skip 函数
        let name = match self.advance() {
            Token::Ident(n) => n,
            Token::True => "真".to_string(),
            Token::False => "假".to_string(),
            Token::Null => "空".to_string(),
            Token::Mutable => "可变".to_string(),
            _ => return None,
        };
        self.expect(&Token::LParen);
        let mut params = Vec::new();
        while *self.peek() != Token::RParen && !self.at_end() {
            let p = match self.advance() {
                Token::Ident(s) => s,
                Token::True => "真".to_string(),
                Token::False => "假".to_string(),
                Token::Null => "空".to_string(),
                Token::Mutable => "可变".to_string(),
                _ => { self.expect(&Token::Comma); continue; }
            };
            params.push(p);
            self.expect(&Token::Comma);
        }
        self.expect(&Token::RParen);
        let body = self.parse_block();
        Some(Stmt::FuncDef { name, params, body })
    }

    fn parse_var_def(&mut self) -> Option<Stmt> {
        self.advance(); // skip 定义
        let mutable = self.expect(&Token::Mutable);
        let name = match self.advance() {
            Token::Ident(n) => n,
            Token::True => "真".to_string(),
            Token::False => "假".to_string(),
            Token::Null => "空".to_string(),
            _ => return None,
        };
        let init = if self.expect(&Token::Eq) { Some(self.parse_expr()) } else { None };
        Some(Stmt::VarDef { name, mutable, init })
    }

    fn parse_return(&mut self) -> Option<Stmt> {
        self.advance(); // skip 返回
        if matches!(self.peek(), Token::RBrace | Token::Eof) {
            Some(Stmt::Return(None))
        } else {
            Some(Stmt::Return(Some(self.parse_expr())))
        }
    }

    fn parse_if(&mut self) -> Option<Stmt> {
        self.advance(); // skip 如果
        self.expect(&Token::LParen);
        let mut cond = self.parse_expr();
        self.expect(&Token::RParen);
        // 处理 如果 (a) || (b) 等条件: ()只包裹部分表达式
        while let Some((op, prec)) = self.peek_binop() {
            self.advance();
            let right = self.parse_binary(prec + 1);
            cond = Expr::Binary(Box::new(cond), op, Box::new(right));
        }
        let then_body = self.parse_block();
        let else_body = if self.expect(&Token::Else) {
            if matches!(self.peek(), Token::If) {
                if let Some(nested) = self.parse_if() { vec![nested] } else { Vec::new() }
            } else {
                self.parse_block()
            }
        } else { Vec::new() };
        Some(Stmt::If { cond, then_body, else_body })
    }

    fn parse_while(&mut self) -> Option<Stmt> {
        self.advance(); // skip 当
        self.expect(&Token::LParen);
        let mut cond = self.parse_expr();
        self.expect(&Token::RParen);
        while let Some((op, prec)) = self.peek_binop() {
            self.advance();
            let right = self.parse_binary(prec + 1);
            cond = Expr::Binary(Box::new(cond), op, Box::new(right));
        }
        let body = self.parse_block();
        Some(Stmt::While { cond, body })
    }

    fn parse_import(&mut self) -> Option<Stmt> {
        self.advance(); // skip 导入
        self.expect(&Token::LParen);
        let path = match self.advance() {
            Token::Str(s) => s,
            _ => String::new(),
        };
        self.expect(&Token::RParen);
        Some(Stmt::Import(path))
    }

    fn parse_struct_def(&mut self) -> Option<Stmt> {
        self.advance(); // skip 结构
        let name = match self.advance() {
            Token::Ident(n) => n,
            _ => return None,
        };
        let mut fields = Vec::new();
        if self.expect(&Token::LParen) {
            // tuple style: 结构 Name(f1, f2, ...)
            while *self.peek() != Token::RParen && !self.at_end() {
                match self.advance() {
                    Token::Ident(f) => fields.push(f),
                    _ => {}
                }
                self.expect(&Token::Comma);
            }
            self.expect(&Token::RParen);
        } else if self.expect(&Token::LBrace) {
            // brace style: 结构 Name { f1, f2, ... }
            while *self.peek() != Token::RBrace && !self.at_end() {
                match self.advance() {
                    Token::Ident(f) => fields.push(f),
                    _ => {}
                }
                self.expect(&Token::Comma);
            }
            self.expect(&Token::RBrace);
        }
        Some(Stmt::StructDef { name, fields })
    }

    fn parse_block(&mut self) -> Vec<Stmt> {
        let mut stmts = Vec::new();
        if !self.expect(&Token::LBrace) { return stmts; }
        while *self.peek() != Token::RBrace && !self.at_end() {
            if let Some(s) = self.parse_stmt() { stmts.push(s); }
        }
        self.expect(&Token::RBrace);
        stmts
    }

    fn parse_match(&mut self) -> Option<Stmt> {
        self.advance(); // skip 匹配
        self.expect(&Token::LParen);
        let target = self.parse_expr();
        self.expect(&Token::RParen);
        self.expect(&Token::LBrace);
        let mut arms = Vec::new();
        while *self.peek() != Token::RBrace && !self.at_end() {
            if matches!(self.peek(), Token::Default) {
                self.advance(); self.expect(&Token::Colon);
                let body = self.parse_block();
                arms.push((None, body));
            } else {
                let pat = self.parse_expr();
                self.expect(&Token::Colon);
                let body = self.parse_block();
                arms.push((Some(pat), body));
            }
        }
        self.expect(&Token::RBrace);
        Some(Stmt::Match { target, arms })
    }

    fn parse_try(&mut self) -> Option<Stmt> {
        self.advance(); // skip 尝试
        let body = self.parse_block();
        let mut catch_var = None;
        let mut catch_body = Vec::new();
        if self.expect(&Token::Catch) {
            if self.expect(&Token::LParen) {
                if let Token::Ident(v) = self.advance() { catch_var = Some(v); }
                self.expect(&Token::RParen);
            }
            catch_body = self.parse_block();
        }
        Some(Stmt::Try { body, catch_var, catch_body })
    }

    fn parse_expr_or_assign(&mut self) -> Option<Stmt> {
        let expr = self.parse_expr();
        if self.expect(&Token::Eq) {
            let val = self.parse_expr();
            Some(Stmt::Assign { target: expr, value: val })
        } else if matches!(self.peek(), Token::PlusEq) {
            self.advance();
            let val = self.parse_expr();
            match &expr {
                Expr::Ident(n) => Some(Stmt::Assign { target: expr.clone(), value: Expr::Binary(Box::new(Expr::Ident(n.clone())), BinOp::Add, Box::new(val)) }),
                _ => Some(Stmt::ExprStmt(expr)),
            }
        } else if matches!(self.peek(), Token::MinusEq) {
            self.advance();
            let val = self.parse_expr();
            match &expr {
                Expr::Ident(n) => Some(Stmt::Assign { target: expr.clone(), value: Expr::Binary(Box::new(Expr::Ident(n.clone())), BinOp::Sub, Box::new(val)) }),
                _ => Some(Stmt::ExprStmt(expr)),
            }
        } else {
            Some(Stmt::ExprStmt(expr))
        }
    }

    // ═══ Pratt式表达式解析 ═══

    fn parse_expr(&mut self) -> Expr { self.parse_binary(0) }

    fn parse_binary(&mut self, min_prec: u8) -> Expr {
        let mut left = self.parse_unary();
        while let Some((op, prec)) = self.peek_binop() {
            if prec < min_prec { break; }
            self.advance();
            let right = self.parse_binary(prec + 1);
            left = Expr::Binary(Box::new(left), op, Box::new(right));
        }
        left
    }

    fn peek_binop(&self) -> Option<(BinOp, u8)> {
        match self.peek() {
            Token::Or => Some((BinOp::Or, 1)),
            Token::And => Some((BinOp::And, 2)),
            Token::BitOr => Some((BinOp::BitOr, 3)),
            Token::BitXor => Some((BinOp::BitXor, 4)),
            Token::BitAnd => Some((BinOp::BitAnd, 5)),
            Token::EqEq => Some((BinOp::Eq, 6)),
            Token::NotEq => Some((BinOp::Ne, 6)),
            Token::Lt => Some((BinOp::Lt, 7)),
            Token::Gt => Some((BinOp::Gt, 7)),
            Token::LtEq => Some((BinOp::Le, 7)),
            Token::GtEq => Some((BinOp::Ge, 7)),
            Token::Shl => Some((BinOp::Shl, 8)),
            Token::Shr => Some((BinOp::Shr, 8)),
            Token::Plus => Some((BinOp::Add, 9)),
            Token::Minus => Some((BinOp::Sub, 9)),
            Token::Star => Some((BinOp::Mul, 10)),
            Token::Slash => Some((BinOp::Div, 10)),
            Token::Percent => Some((BinOp::Mod, 10)),
            _ => None,
        }
    }

    fn parse_unary(&mut self) -> Expr {
        match self.peek().clone() {
            Token::Minus => { self.advance(); Expr::Unary(UnaryOp::Neg, Box::new(self.parse_unary())) }
            Token::Not => { self.advance(); Expr::Unary(UnaryOp::Not, Box::new(self.parse_unary())) }
            Token::Tilde => { self.advance(); Expr::Unary(UnaryOp::BitNot, Box::new(self.parse_unary())) }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> Expr {
        let mut expr = self.parse_primary();
        loop {
            match self.peek() {
                Token::LParen => {
                    self.advance();
                    let name = match &expr {
                        Expr::Ident(n) => n.clone(),
                        Expr::Member(_, field) => field.clone(),
                        _ => String::new(),
                    };
                    let mut args = Vec::new();
                    while *self.peek() != Token::RParen && !self.at_end() {
                        args.push(self.parse_expr());
                        self.expect(&Token::Comma);
                    }
                    self.expect(&Token::RParen);
                    expr = Expr::Call(name, args);
                }
                Token::LBracket => {
                    self.advance();
                    let idx = self.parse_expr();
                    self.expect(&Token::RBracket);
                    expr = Expr::Index(Box::new(expr), Box::new(idx));
                }
                Token::Dot => {
                    self.advance();
                    if let Token::Ident(field) = self.advance() {
                        expr = Expr::Member(Box::new(expr), field);
                    }
                }
                Token::LBrace if matches!(&expr, Expr::Ident(_)) => {
                    // Ident { ... } → struct construction (shorthand for 建 Ident { ... })
                    let sname = if let Expr::Ident(n) = &expr { n.clone() } else { String::new() };
                    self.advance(); // consume {
                    let mut fields = Vec::new();
                    while *self.peek() != Token::RBrace && !self.at_end() {
                        let fname = match self.advance() {
                            Token::Ident(f) => f,
                            _ => String::new(),
                        };
                        self.expect(&Token::Colon);
                        let val = self.parse_expr();
                        fields.push((fname, val));
                        self.expect(&Token::Comma);
                    }
                    self.expect(&Token::RBrace);
                    expr = Expr::StructNew(sname, fields);
                }
                _ => break,
            }
        }
        expr
    }

    fn parse_primary(&mut self) -> Expr {
        match self.advance() {
            Token::Int(n) => Expr::Int(n),
            Token::Float(f) => Expr::Float(f),
            Token::Str(s) => Expr::Str(s),
            Token::True => {
                if matches!(self.peek(), Token::LParen | Token::LBracket | Token::Dot | Token::Eq) {
                    Expr::Ident("真".to_string())
                } else { Expr::Bool(true) }
            }
            Token::False => {
                if matches!(self.peek(), Token::LParen | Token::LBracket | Token::Dot | Token::Eq) {
                    Expr::Ident("假".to_string())
                } else { Expr::Bool(false) }
            }
            Token::Null => {
                if matches!(self.peek(), Token::LParen | Token::LBracket | Token::Dot | Token::Eq) {
                    Expr::Ident("空".to_string())
                } else { Expr::Null }
            }
            Token::Mutable => Expr::Ident("可变".to_string()),
            Token::Ident(n) => Expr::Ident(n),
            Token::LParen => {
                let e = self.parse_expr();
                self.expect(&Token::RParen);
                e
            }
            Token::LBracket => {
                let mut items = Vec::new();
                while *self.peek() != Token::RBracket && !self.at_end() {
                    items.push(self.parse_expr());
                    self.expect(&Token::Comma);
                }
                self.expect(&Token::RBracket);
                Expr::List(items)
            }
            Token::Build => {
                // 建 Name { f: v, ... } or 建 Name(v1, v2, ...)
                let sname = match self.advance() {
                    Token::Ident(n) => n,
                    _ => String::new(),
                };
                let mut fields = Vec::new();
                if self.expect(&Token::LParen) {
                    // tuple style: 建 Name(v1, v2, ...)
                    while *self.peek() != Token::RParen && !self.at_end() {
                        let val = self.parse_expr();
                        fields.push((String::new(), val));
                        self.expect(&Token::Comma);
                    }
                    self.expect(&Token::RParen);
                } else if self.expect(&Token::LBrace) {
                    // brace style: 建 Name { f: v, ... }
                    while *self.peek() != Token::RBrace && !self.at_end() {
                        let fname = match self.advance() {
                            Token::Ident(f) => f,
                            _ => String::new(),
                        };
                        self.expect(&Token::Colon);
                        let val = self.parse_expr();
                        fields.push((fname, val));
                        self.expect(&Token::Comma);
                    }
                    self.expect(&Token::RBrace);
                }
                Expr::StructNew(sname, fields)
            }
            Token::Func => {
                // Lambda: 函数(params) { body }
                self.expect(&Token::LParen);
                let mut params = Vec::new();
                while *self.peek() != Token::RParen && !self.at_end() {
                    let p = match self.advance() {
                        Token::Ident(s) => s,
                        _ => { self.expect(&Token::Comma); continue; }
                    };
                    params.push(p);
                    self.expect(&Token::Comma);
                }
                self.expect(&Token::RParen);
                let body = self.parse_block();
                Expr::Lambda(params, body)
            }
            Token::LBrace => {
                // {} → 字典() (空字典字面量)
                self.expect(&Token::RBrace);
                Expr::Call("字典".to_string(), vec![])
            }
            _ => Expr::Null,
        }
    }
}

// ═══ Codegen: AST→x86-64机器码 ═══

pub struct Codegen {
    pub code: Vec<u8>,
    vars: HashMap<String, i32>,
    pub funcs: HashMap<String, u32>,
    pub call_fixups: Vec<(u32, String)>,
    var_count: i32,
    pub global_vars: HashMap<String, i32>, // 全局变量: 名→索引
    pub global_count: i32,
    pub struct_defs: HashMap<String, Vec<String>>, // 结构体: 名→字段列表
    field_ambig_warned: HashSet<String>, // 已警告的歧义字段(去重)
    var_types: HashMap<String, VarType>,
    lambda_counter: u32,
    pending_lambdas: Vec<(String, Vec<String>, Vec<Stmt>, Vec<String>)>,
    loop_stack: Vec<(usize, Vec<usize>)>, // (loop_top, break_fixups)
}

impl Codegen {
    pub fn new() -> Self {
        Codegen { code: Vec::new(), vars: HashMap::new(), funcs: HashMap::new(),
                  call_fixups: Vec::new(), var_count: 0, global_vars: HashMap::new(), global_count: 0,
                  struct_defs: HashMap::new(), field_ambig_warned: HashSet::new(),
                  var_types: HashMap::new(),
                  lambda_counter: 0, pending_lambdas: Vec::new(),
                  loop_stack: Vec::new() }
    }
    fn emit(&mut self, b: &[u8]) { self.code.extend_from_slice(b); }
    fn emit1(&mut self, b: u8) { self.code.push(b); }
    fn pos(&self) -> usize { self.code.len() }
    fn emit_i32(&mut self, v: i32) { self.code.extend_from_slice(&v.to_le_bytes()); }
    fn patch_i32(&mut self, off: usize, v: i32) {
        let b = v.to_le_bytes();
        if off+4<=self.code.len() { self.code[off..off+4].copy_from_slice(&b); }
    }
    fn patch_jmp_here(&mut self, fixup: usize) {
        let rel = self.pos() as i32 - fixup as i32 - 4;
        self.patch_i32(fixup, rel);
    }
    fn alloc_var(&mut self, name: &str) -> i32 {
        if let Some(&off) = self.vars.get(name) { return off; }
        // 检查是否是全局变量
        if let Some(&gidx) = self.global_vars.get(name) {
            let off = -1000000 - gidx; // 负偏移-1000000以下=全局
            self.vars.insert(name.to_string(), off);
            return off;
        }
        self.var_count += 1;
        let off = -(self.var_count * 8);
        self.vars.insert(name.to_string(), off);
        off
    }
    fn emit_store_var(&mut self, off: i32) {
        if off <= -1000000 {
            // 全局变量: mov [r15+idx*8], rax
            let idx = (-1000000 - off) as u32;
            let disp = idx as i32 * 8;
            self.emit(&[0x49,0x89,0x87]); // mov [r15+disp32], rax
            self.emit_i32(disp);
        } else if off >= -128 {
            self.emit(&[0x48,0x89,0x45]); self.emit1((off & 0xFF) as u8);
        } else {
            self.emit(&[0x48,0x89,0x85]); self.emit_i32(off);
        }
    }
    fn emit_load_var(&mut self, off: i32) {
        if off <= -1000000 {
            // 全局变量: mov rax, [r15+idx*8]
            let idx = (-1000000 - off) as u32;
            let disp = idx as i32 * 8;
            self.emit(&[0x49,0x8B,0x87]); // mov rax, [r15+disp32]
            self.emit_i32(disp);
        } else if off >= -128 {
            self.emit(&[0x48,0x8B,0x45]); self.emit1((off & 0xFF) as u8);
        } else {
            self.emit(&[0x48,0x8B,0x85]); self.emit_i32(off);
        }
    }

    fn emit_load_stack_slot_to_call_reg(&mut self, reg_index: usize, disp: i32) {
        let disp8 = disp >= -128 && disp <= 127;
        match reg_index {
            0 => {
                if disp == 0 {
                    self.emit(&[0x48,0x8B,0x0C,0x24]);
                } else if disp8 {
                    self.emit(&[0x48,0x8B,0x4C,0x24]); self.emit1(disp as u8);
                } else {
                    self.emit(&[0x48,0x8B,0x8C,0x24]); self.emit_i32(disp);
                }
            }
            1 => {
                if disp == 0 {
                    self.emit(&[0x48,0x8B,0x14,0x24]);
                } else if disp8 {
                    self.emit(&[0x48,0x8B,0x54,0x24]); self.emit1(disp as u8);
                } else {
                    self.emit(&[0x48,0x8B,0x94,0x24]); self.emit_i32(disp);
                }
            }
            2 => {
                if disp == 0 {
                    self.emit(&[0x4C,0x8B,0x04,0x24]);
                } else if disp8 {
                    self.emit(&[0x4C,0x8B,0x44,0x24]); self.emit1(disp as u8);
                } else {
                    self.emit(&[0x4C,0x8B,0x84,0x24]); self.emit_i32(disp);
                }
            }
            3 => {
                if disp == 0 {
                    self.emit(&[0x4C,0x8B,0x0C,0x24]);
                } else if disp8 {
                    self.emit(&[0x4C,0x8B,0x4C,0x24]); self.emit1(disp as u8);
                } else {
                    self.emit(&[0x4C,0x8B,0x8C,0x24]); self.emit_i32(disp);
                }
            }
            _ => {}
        }
    }

    fn is_float_expr(&self, e: &Expr) -> bool {
        match e {
            Expr::Float(_) => true,
            Expr::Ident(n) => self.var_types.get(n.as_str()) == Some(&VarType::Float),
            Expr::Binary(l, _, r) => self.is_float_expr(l) || self.is_float_expr(r),
            Expr::Unary(_, inner) => self.is_float_expr(inner),
            Expr::Call(n, _) => matches!(n.as_str(), "转浮点"|"浮点"),
            _ => false,
        }
    }

    fn free_vars_expr(e: &Expr, params: &[String], out: &mut Vec<String>) {
        match e {
            Expr::Ident(n) => {
                if !params.contains(n) && !out.contains(n) { out.push(n.clone()); }
            }
            Expr::Binary(l, _, r) => { Self::free_vars_expr(l, params, out); Self::free_vars_expr(r, params, out); }
            Expr::Unary(_, inner) => Self::free_vars_expr(inner, params, out),
            Expr::Call(_, args) => { for a in args { Self::free_vars_expr(a, params, out); } }
            Expr::Index(b, i) => { Self::free_vars_expr(b, params, out); Self::free_vars_expr(i, params, out); }
            Expr::Member(b, _) => Self::free_vars_expr(b, params, out),
            Expr::List(items) => { for it in items { Self::free_vars_expr(it, params, out); } }
            Expr::Lambda(_, _) => {}
            _ => {}
        }
    }
    fn free_vars_stmt(s: &Stmt, params: &[String], out: &mut Vec<String>) {
        match s {
            Stmt::ExprStmt(e) | Stmt::Return(Some(e)) => Self::free_vars_expr(e, params, out),
            Stmt::VarDef { init: Some(e), .. } => Self::free_vars_expr(e, params, out),
            Stmt::Assign { target, value } => { Self::free_vars_expr(target, params, out); Self::free_vars_expr(value, params, out); }
            Stmt::If { cond, then_body, else_body } => {
                Self::free_vars_expr(cond, params, out);
                for s in then_body { Self::free_vars_stmt(s, params, out); }
                for s in else_body { Self::free_vars_stmt(s, params, out); }
            }
            Stmt::While { cond, body } => {
                Self::free_vars_expr(cond, params, out);
                for s in body { Self::free_vars_stmt(s, params, out); }
            }
            Stmt::Match { target, arms } => {
                Self::free_vars_expr(target, params, out);
                for (pat, body) in arms {
                    if let Some(p) = pat { Self::free_vars_expr(p, params, out); }
                    for s in body { Self::free_vars_stmt(s, params, out); }
                }
            }
            Stmt::Try { body, catch_body, .. } => {
                for s in body { Self::free_vars_stmt(s, params, out); }
                for s in catch_body { Self::free_vars_stmt(s, params, out); }
            }
            Stmt::Throw(e) => Self::free_vars_expr(e, params, out),
            _ => {}
        }
    }
    fn collect_free_vars(params: &[String], body: &[Stmt]) -> Vec<String> {
        let mut out = Vec::new();
        for s in body { Self::free_vars_stmt(s, params, &mut out); }
        out
    }

    pub fn compile_program(&mut self, stmts: &[Stmt]) {
        // 第一遍：收集函数名和全局变量名
        for s in stmts {
            match s {
                Stmt::FuncDef{name,..} => { self.funcs.insert(name.clone(), 0); }
                Stmt::VarDef{name,..} => {
                    if !self.global_vars.contains_key(name) {
                        self.global_count += 1;
                        self.global_vars.insert(name.clone(), self.global_count);
                    }
                }
                Stmt::StructDef{name, fields} => {
                    self.struct_defs.insert(name.clone(), fields.clone());
                }
                _ => {}
            }
        }
        
        // 第二遍：生成__初始化全局__函数(初始化全局变量+执行顶层代码)
        {
            let init_off = self.pos() as u32;
            self.funcs.insert("__初始化全局__".to_string(), init_off);
            self.vars.clear(); self.var_count = 0; self.var_types.clear();
            // prologue with dynamic frame size (patched later)
            self.emit(&[0x55,0x48,0x89,0xE5,0x48,0x81,0xEC]);
            let ff = self.pos(); self.emit_i32(0x100); // placeholder
            for s in stmts {
                match s {
                    Stmt::FuncDef{..} | Stmt::StructDef{..} | Stmt::Import(_) => {}
                    Stmt::VarDef{name, init, ..} => {
                        let gidx = self.global_vars.get(name).copied().unwrap_or(0);
                        let off = -1000000 - gidx;
                        if let Some(expr) = init {
                            if self.is_float_expr(expr) { self.var_types.insert(name.clone(), VarType::Float); }
                            self.compile_expr(expr);
                        } else {
                            self.emit(&[0x48,0x31,0xC0]);
                        }
                        self.emit_store_var(off);
                    }
                    _ => { self.compile_stmt(s); }
                }
            }
            self.emit(&[0x31,0xC0,0xC9,0xC3]); // xor eax,eax; leave; ret
            let fs = ((self.var_count as u32 + 4) * 8 + 15) & !15;
            let fs = (fs.max(0x20) + 15) & !15;
            self.patch_i32(ff, fs as i32);
            self.vars.clear(); self.var_count = 0; self.var_types.clear();
        }
        
        // 第三遍：编译函数定义
        for s in stmts {
            if let Stmt::FuncDef{..} = s { self.compile_stmt(s); }
        }
        // 第四遍：编译pending lambdas (闭包体)
        while !self.pending_lambdas.is_empty() {
            let lambdas = std::mem::take(&mut self.pending_lambdas);
            for (lname, params, body, captures) in lambdas {
                self.funcs.insert(lname.clone(), self.pos() as u32);
                self.vars.clear(); self.var_count = 0; self.var_types.clear();
                // 第一个参数 __env 在 [rbp+16] (rcx)
                self.vars.insert("__env".to_string(), 16);
                // 用户参数从 rdx/r8/r9 开始 (shifted by 1)
                for (i, p) in params.iter().enumerate() {
                    self.vars.insert(p.clone(), 24 + (i as i32 * 8));
                }
                self.emit(&[0x55,0x48,0x89,0xE5,0x48,0x81,0xEC]);
                let ff = self.pos(); self.emit_i32(0x100);
                // save registers: rcx=env, rdx=param0, r8=param1, r9=param2
                self.emit(&[0x48,0x89,0x4D,0x10]); // mov [rbp+16], rcx (__env)
                if !params.is_empty() { self.emit(&[0x48,0x89,0x55,0x18]); } // mov [rbp+24], rdx
                if params.len() >= 2 { self.emit(&[0x4C,0x89,0x45,0x20]); } // mov [rbp+32], r8
                if params.len() >= 3 { self.emit(&[0x4C,0x89,0x4D,0x28]); } // mov [rbp+40], r9
                // 载入捕获变量到本地变量: [rbp+16]=env, cap_i = [env + (1+i)*8]
                for (i, cap) in captures.iter().enumerate() {
                    self.var_count += 1;
                    let local_off = -(self.var_count * 8);
                    self.vars.insert(cap.clone(), local_off);
                    // mov rax, [rbp+16] (env ptr)
                    self.emit(&[0x48,0x8B,0x45,0x10]);
                    // mov rax, [rax + (1+i)*8]
                    let disp = ((1 + i) * 8) as i32;
                    self.emit(&[0x48,0x8B,0x40]); self.emit1(disp as u8);
                    // mov [rbp+local_off], rax
                    self.emit_store_var(local_off);
                }
                for st in &body { self.compile_stmt(st); }
                self.emit(&[0x31,0xC0,0xC9,0xC3]);
                let fs = ((self.var_count as u32 + 4 + captures.len() as u32) * 8 + 15) & !15;
                let fs = (fs.max(0x20) + 15) & !15;
                self.patch_i32(ff, fs as i32);
            }
        }
        self.resolve_calls();
    }

    fn compile_stmt(&mut self, s: &Stmt) {
        match s {
            Stmt::FuncDef{name,params,body} => {
                self.funcs.insert(name.clone(), self.pos() as u32);
                self.vars.clear(); self.var_count = 0; self.var_types.clear();
                for(i,p) in params.iter().enumerate() { self.vars.insert(p.clone(), 16+(i as i32*8)); }
                self.emit(&[0x55,0x48,0x89,0xE5,0x48,0x81,0xEC]);
                let ff=self.pos(); self.emit_i32(0x100);
                if params.len()>=1{self.emit(&[0x48,0x89,0x4D,0x10]);}
                if params.len()>=2{self.emit(&[0x48,0x89,0x55,0x18]);}
                if params.len()>=3{self.emit(&[0x4C,0x89,0x45,0x20]);}
                if params.len()>=4{self.emit(&[0x4C,0x89,0x4D,0x28]);}
                for st in body { self.compile_stmt(st); }
                self.emit(&[0x31,0xC0,0xC9,0xC3]);
                let fs=((self.var_count as u32+4)*8+15)&!15; let fs=(fs.max(0x20)+15)&!15;
                self.patch_i32(ff, fs as i32);
            }
            Stmt::VarDef{name,init,..} => {
                // 函数内的VarDef必须创建局部变量，绝不fallback到同名全局变量
                // alloc_var会fallback到global_vars，这里直接创建局部变量
                let off = if let Some(&existing) = self.vars.get(name.as_str()) {
                    existing
                } else {
                    self.var_count += 1;
                    let new_off = -(self.var_count * 8);
                    self.vars.insert(name.clone(), new_off);
                    new_off
                };
                if let Some(e)=init {
                    if self.is_float_expr(e) { self.var_types.insert(name.clone(), VarType::Float); }
                    self.compile_expr(e); self.emit_store_var(off);
                }
            }
            Stmt::Assign{target,value} => {
                match target {
                    Expr::Ident(n) => {
                        if self.is_float_expr(value) { self.var_types.insert(n.clone(), VarType::Float); }
                        self.compile_expr(value);
                        let off=if let Some(&o)=self.vars.get(n.as_str()){o}
                                else if let Some(&gidx)=self.global_vars.get(n.as_str()){-1000000-gidx}
                                else{self.alloc_var(n)};
                        self.emit_store_var(off);
                    }
                    Expr::Index(base,idx) => {
                        // list[idx] = value → __运行时_列表设(list, idx, value)
                        self.compile_expr(base); self.emit1(0x50); // push list
                        self.compile_expr(idx); self.emit1(0x50);  // push idx
                        self.compile_expr(value);
                        self.emit(&[0x49,0x89,0xC0]); // mov r8, rax (value)
                        self.emit1(0x5A); // pop rdx (idx)
                        self.emit1(0x59); // pop rcx (list)
                        self.emit1(0xE8); let cp=self.pos() as u32; self.emit_i32(0);
                        self.call_fixups.push((cp, "__运行时_列表设".to_string()));
                    }
                    Expr::Member(base, field) => {
                        // obj.field = value → list_set(obj, field_idx, value)
                        let field_idx = self.find_field_index(field);
                        if let Some(idx) = field_idx {
                            self.compile_expr(base); self.emit1(0x50); // push obj
                            self.compile_expr(value);
                            self.emit(&[0x49,0x89,0xC0]); // mov r8, rax (value)
                            self.emit1(0x59); // pop rcx (obj/list)
                            self.emit(&[0x48,0xC7,0xC2]); self.emit_i32(idx as i32); // mov rdx, idx
                            self.emit1(0xE8); let cp=self.pos() as u32; self.emit_i32(0);
                            self.call_fixups.push((cp, "__运行时_列表设".to_string()));
                        } else {
                            self.compile_expr(value);
                        }
                    }
                    _ => { self.compile_expr(value); }
                }
            }
            Stmt::Return(e) => {
                if let Some(ex)=e { self.compile_expr(ex); } else { self.emit(&[0x31,0xC0]); }
                self.emit(&[0xC9,0xC3]);
            }
            Stmt::If{cond,then_body,else_body} => {
                self.compile_expr(cond);
                self.emit(&[0x48,0x85,0xC0,0x0F,0x84]); let je=self.pos(); self.emit_i32(0);
                for st in then_body { self.compile_stmt(st); }
                if else_body.is_empty() { self.patch_jmp_here(je); }
                else {
                    self.emit1(0xE9); let jmp=self.pos(); self.emit_i32(0);
                    self.patch_jmp_here(je);
                    for st in else_body { self.compile_stmt(st); }
                    self.patch_jmp_here(jmp);
                }
            }
            Stmt::While{cond,body} => {
                let top=self.pos();
                self.compile_expr(cond);
                self.emit(&[0x48,0x85,0xC0,0x0F,0x84]); let je=self.pos(); self.emit_i32(0);
                self.loop_stack.push((top, Vec::new()));
                for st in body { self.compile_stmt(st); }
                self.emit1(0xE9); self.emit_i32(top as i32 - self.pos() as i32 - 4);
                self.patch_jmp_here(je);
                let (_, break_fixups) = self.loop_stack.pop().unwrap();
                for bf in break_fixups { self.patch_jmp_here(bf); }
            }
            Stmt::Break => {
                if !self.loop_stack.is_empty() {
                    self.emit1(0xE9); let bf = self.pos(); self.emit_i32(0);
                    self.loop_stack.last_mut().unwrap().1.push(bf);
                }
            }
            Stmt::Continue => {
                if let Some(&(top, _)) = self.loop_stack.last() {
                    self.emit1(0xE9); self.emit_i32(top as i32 - self.pos() as i32 - 4);
                }
            }
            Stmt::Match { target, arms } => {
                self.compile_expr(target); self.emit1(0x50);
                let mut end_fixups = Vec::new();
                for (pat, body) in arms {
                    if let Some(p) = pat {
                        self.emit(&[0x48,0x8B,0x04,0x24]); // mov rax,[rsp]
                        self.emit1(0x50); // push target copy
                        self.compile_expr(p);
                        self.emit(&[0x48,0x89,0xC1]); self.emit1(0x58); // mov rcx,rax; pop rax
                        self.emit(&[0x48,0x39,0xC8,0x0F,0x85]); // cmp rax,rcx; jne
                        let jne = self.pos(); self.emit_i32(0);
                        for st in body { self.compile_stmt(st); }
                        self.emit1(0xE9); let jend = self.pos(); self.emit_i32(0);
                        end_fixups.push(jend);
                        self.patch_jmp_here(jne);
                    } else {
                        for st in body { self.compile_stmt(st); }
                    }
                }
                for ef in end_fixups { self.patch_jmp_here(ef); }
                self.emit(&[0x48,0x83,0xC4,0x08]); // add rsp,8 (pop target)
            }
            Stmt::Try { body, catch_body, .. } => {
                for st in body { self.compile_stmt(st); }
                if !catch_body.is_empty() {
                    self.emit1(0xE9); let jskip = self.pos(); self.emit_i32(0);
                    let _catch_start = self.pos();
                    for st in catch_body { self.compile_stmt(st); }
                    self.patch_jmp_here(jskip);
                }
            }
            Stmt::Throw(e) => { self.compile_expr(e); }
            Stmt::ExprStmt(e) => { self.compile_expr(e); }
            Stmt::Import(_) => {}
            Stmt::StructDef{..} => {} // already collected in first pass
        }
    }

    fn compile_expr(&mut self, e: &Expr) {
        match e {
            Expr::Int(n) => {
                if *n >= i32::MIN as i64 && *n <= i32::MAX as i64 {
                    self.emit(&[0x48,0xC7,0xC0]); self.emit_i32(*n as i32); // mov rax, imm32 (sign-ext)
                } else {
                    self.emit(&[0x48,0xB8]); // movabs rax, imm64
                    self.code.extend_from_slice(&n.to_le_bytes());
                }
            }
            Expr::Float(f) => {
                let bits = f.to_bits();
                self.emit(&[0x48,0xB8]); // movabs rax, imm64
                self.code.extend_from_slice(&bits.to_le_bytes());
            }
            Expr::Str(s) => {
                // 内嵌字符串: jmp rel8(跳过) + 字节 + \0 + lea rax,[rip+disp]
                let bytes = s.as_bytes();
                let len = bytes.len() + 1; // +1 for NUL
                if len < 126 {
                    self.emit1(0xEB); // jmp rel8
                    self.emit1(len as u8);
                    let str_off = self.pos();
                    self.emit(bytes);
                    self.emit1(0x00); // NUL terminator
                    // lea rax, [rip + disp32] 指向字符串
                    self.emit(&[0x48,0x8D,0x05]);
                    let disp = str_off as i32 - (self.pos() as i32 + 4);
                    self.emit_i32(disp);
                } else {
                    self.emit1(0xE9); // jmp rel32
                    self.emit_i32(len as i32);
                    let str_off = self.pos();
                    self.emit(bytes);
                    self.emit1(0x00);
                    self.emit(&[0x48,0x8D,0x05]);
                    let disp = str_off as i32 - (self.pos() as i32 + 4);
                    self.emit_i32(disp);
                }
            }
            Expr::Bool(true) => { self.emit(&[0x48,0xC7,0xC0,0x01,0x00,0x00,0x00]); }
            Expr::Bool(false) => { self.emit(&[0x48,0x31,0xC0]); }
            Expr::Null => { self.emit(&[0x48,0xC7,0xC0,0xB4,0xB1,0xB1,0xB1]); }
            Expr::Ident(n) => {
                if let Some(&off)=self.vars.get(n.as_str()) { self.emit_load_var(off); }
                else if let Some(&gidx)=self.global_vars.get(n.as_str()) { self.emit_load_var(-1000000-gidx); }
                else { self.emit(&[0x48,0xC7,0xC0,0xB4,0xB1,0xB1,0xB1]); }
            }
            Expr::Binary(l,op,r) => {
                let is_str = matches!(op, BinOp::Add) && (Self::is_str_concat_expr(l) || Self::is_str_concat_expr(r));
                if is_str {
                    self.compile_expr(l); self.emit1(0x50);
                    self.compile_expr(r);
                    self.emit(&[0x48,0x89,0xC2]); self.emit1(0x59);
                    self.emit(&[0x48,0x83,0xEC,0x20]); // sub rsp,32 (shadow space)
                    self.emit1(0xE8); let cp=self.pos() as u32; self.emit_i32(0);
                    self.call_fixups.push((cp, Self::runtime_name_concat()));
                    self.emit(&[0x48,0x83,0xC4,0x20]); // add rsp,32
                } else if matches!(op, BinOp::And) {
                    self.compile_expr(l);
                    self.emit(&[0x48,0x85,0xC0,0x0F,0x84]); let jz=self.pos(); self.emit_i32(0);
                    self.compile_expr(r);
                    self.patch_jmp_here(jz);
                } else if matches!(op, BinOp::Or) {
                    self.compile_expr(l);
                    self.emit(&[0x48,0x85,0xC0,0x0F,0x85]); let jnz=self.pos(); self.emit_i32(0);
                    self.compile_expr(r);
                    self.patch_jmp_here(jnz);
                } else if matches!(op, BinOp::Eq|BinOp::Ne) && (Self::is_str_expr(l) || Self::is_str_expr(r)) {
                    // 字符串相等/不等: call __运行时_串相等
                    self.compile_expr(l); self.emit1(0x50);
                    self.compile_expr(r);
                    self.emit(&[0x48,0x89,0xC2]); self.emit1(0x59); // mov rdx,rax; pop rcx
                    self.emit(&[0x48,0x83,0xEC,0x20]); // sub rsp,32 (shadow space)
                    self.emit1(0xE8); let cp=self.pos() as u32; self.emit_i32(0);
                    self.call_fixups.push((cp, "字符串相等".to_string()));
                    self.emit(&[0x48,0x83,0xC4,0x20]); // add rsp,32
                    if matches!(op, BinOp::Ne) {
                        self.emit(&[0x48,0x85,0xC0,0x0F,0x94,0xC0,0x0F,0xB6,0xC0]); // test+sete+movzx (invert)
                    }
                } else if self.is_float_expr(l) || self.is_float_expr(r) {
                    // SSE2浮点二元运算
                    let l_float = self.is_float_expr(l);
                    let r_float = self.is_float_expr(r);
                    self.compile_expr(l); self.emit1(0x50);
                    self.compile_expr(r); self.emit(&[0x48,0x89,0xC1]); self.emit1(0x58);
                    // left→xmm0, right→xmm1 (convert int→float if needed)
                    if l_float {
                        self.emit(&[0x66,0x48,0x0F,0x6E,0xC0]); // movq xmm0, rax
                    } else {
                        self.emit(&[0xF2,0x48,0x0F,0x2A,0xC0]); // cvtsi2sd xmm0, rax
                    }
                    if r_float {
                        self.emit(&[0x66,0x48,0x0F,0x6E,0xC9]); // movq xmm1, rcx
                    } else {
                        self.emit(&[0xF2,0x48,0x0F,0x2A,0xC9]); // cvtsi2sd xmm1, rcx
                    }
                    let is_cmp = matches!(op, BinOp::Eq|BinOp::Ne|BinOp::Lt|BinOp::Gt|BinOp::Le|BinOp::Ge);
                    match op {
                        BinOp::Add => self.emit(&[0xF2,0x0F,0x58,0xC1]), // addsd xmm0,xmm1
                        BinOp::Sub => self.emit(&[0xF2,0x0F,0x5C,0xC1]), // subsd xmm0,xmm1
                        BinOp::Mul => self.emit(&[0xF2,0x0F,0x59,0xC1]), // mulsd xmm0,xmm1
                        BinOp::Div => self.emit(&[0xF2,0x0F,0x5E,0xC1]), // divsd xmm0,xmm1
                        BinOp::Eq|BinOp::Ne|BinOp::Lt|BinOp::Gt|BinOp::Le|BinOp::Ge => {
                            self.emit(&[0x66,0x0F,0x2E,0xC1]); // ucomisd xmm0,xmm1
                            match op {
                                BinOp::Eq => self.emit(&[0x0F,0x94,0xC0]), // sete al
                                BinOp::Ne => self.emit(&[0x0F,0x95,0xC0]), // setne al
                                BinOp::Lt => self.emit(&[0x0F,0x92,0xC0]), // setb al
                                BinOp::Gt => self.emit(&[0x0F,0x97,0xC0]), // seta al
                                BinOp::Le => self.emit(&[0x0F,0x96,0xC0]), // setbe al
                                BinOp::Ge => self.emit(&[0x0F,0x93,0xC0]), // setae al
                                _ => {}
                            }
                            self.emit(&[0x0F,0xB6,0xC0]); // movzx eax, al
                        }
                        _ => {} // bitwise ops on floats: fallthrough as nop
                    }
                    if !is_cmp {
                        self.emit(&[0x66,0x48,0x0F,0x7E,0xC0]); // movq rax, xmm0
                    }
                } else {
                    self.compile_expr(l); self.emit1(0x50);
                    self.compile_expr(r); self.emit(&[0x48,0x89,0xC1]); self.emit1(0x58);
                    match op {
                        BinOp::Add => self.emit(&[0x48,0x01,0xC8]),
                        BinOp::Sub => self.emit(&[0x48,0x29,0xC8]),
                        BinOp::Mul => self.emit(&[0x48,0x0F,0xAF,0xC1]),
                        BinOp::Div => {
                            // div-by-zero guard: if rcx==0 → rax=0
                            self.emit(&[0x48,0x85,0xC9]);       // test rcx,rcx
                            self.emit(&[0x74,0x07]);             // je .zero (+7: 5+2=7 bytes to skip)
                            self.emit(&[0x48,0x99,0x48,0xF7,0xF9]); // cqo; idiv rcx (5B)
                            self.emit(&[0xEB,0x03]);             // jmp .done (+3: 3 bytes to skip)
                            // .zero:
                            self.emit(&[0x48,0x31,0xC0]);       // xor rax,rax (3B)
                            // .done:
                        }
                        BinOp::Mod => {
                            self.emit(&[0x48,0x85,0xC9]);       // test rcx,rcx
                            self.emit(&[0x74,0x0A]);             // je .zero (+10: 8+2=10 bytes to skip)
                            self.emit(&[0x48,0x99,0x48,0xF7,0xF9,0x48,0x89,0xD0]); // cqo;idiv;mov rax,rdx (8B)
                            self.emit(&[0xEB,0x03]);             // jmp .done (+3)
                            // .zero:
                            self.emit(&[0x48,0x31,0xC0]);       // xor rax,rax (3B)
                            // .done:
                        }
                        BinOp::Eq => self.emit(&[0x48,0x39,0xC8,0x0F,0x94,0xC0,0x0F,0xB6,0xC0]),
                        BinOp::Ne => self.emit(&[0x48,0x39,0xC8,0x0F,0x95,0xC0,0x0F,0xB6,0xC0]),
                        BinOp::Lt => self.emit(&[0x48,0x39,0xC8,0x0F,0x9C,0xC0,0x0F,0xB6,0xC0]),
                        BinOp::Gt => self.emit(&[0x48,0x39,0xC8,0x0F,0x9F,0xC0,0x0F,0xB6,0xC0]),
                        BinOp::Le => self.emit(&[0x48,0x39,0xC8,0x0F,0x9E,0xC0,0x0F,0xB6,0xC0]),
                        BinOp::Ge => self.emit(&[0x48,0x39,0xC8,0x0F,0x9D,0xC0,0x0F,0xB6,0xC0]),
                        BinOp::BitAnd => self.emit(&[0x48,0x21,0xC8]),
                        BinOp::BitOr => self.emit(&[0x48,0x09,0xC8]),
                        BinOp::BitXor => self.emit(&[0x48,0x31,0xC8]),
                        BinOp::Shl => self.emit(&[0x48,0xD3,0xE0]),
                        BinOp::Shr => self.emit(&[0x48,0xD3,0xE8]),
                        _ => {}
                    }
                }
            }
            Expr::Unary(UnaryOp::Neg,inner) => {
                self.compile_expr(inner);
                if self.is_float_expr(inner) {
                    // 浮点取反: XOR sign bit
                    self.emit(&[0x48,0xB9,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x80]); // movabs rcx, 0x8000000000000000
                    self.emit(&[0x48,0x31,0xC8]); // xor rax, rcx
                } else {
                    self.emit(&[0x48,0xF7,0xD8]); // neg rax
                }
            }
            Expr::Unary(UnaryOp::Not,inner) => { self.compile_expr(inner); self.emit(&[0x48,0x85,0xC0,0x0F,0x94,0xC0,0x0F,0xB6,0xC0]); }
            Expr::Unary(UnaryOp::BitNot,inner) => { self.compile_expr(inner); self.emit(&[0x48,0xF7,0xD0]); } // not rax
            Expr::Call(name,args) => {
                // 浮点内建函数
                if name == "转浮点" && args.len() == 1 {
                    self.compile_expr(&args[0]);
                    self.emit(&[0xF2,0x48,0x0F,0x2A,0xC0]); // cvtsi2sd xmm0, rax
                    self.emit(&[0x66,0x48,0x0F,0x7E,0xC0]); // movq rax, xmm0
                    return;
                }
                if name == "转整数" && args.len() == 1 {
                    self.compile_expr(&args[0]);
                    self.emit(&[0x66,0x48,0x0F,0x6E,0xC0]); // movq xmm0, rax
                    self.emit(&[0xF2,0x48,0x0F,0x2C,0xC0]); // cvttsd2si rax, xmm0
                    return;
                }
                // 间接调用 (闭包): name为空时,调用目标是变量中的闭包指针
                let is_indirect = name.is_empty() || (!self.funcs.contains_key(name) &&
                    (self.vars.contains_key(name.as_str()) || self.global_vars.contains_key(name.as_str())));
                if is_indirect {
                    // 先编译参数到栈
                    for a in args.iter() {
                        self.compile_expr(a);
                        self.emit1(0x50); // push rax
                    }
                    // 加载闭包指针
                    if !name.is_empty() {
                        let off = if let Some(&o) = self.vars.get(name.as_str()) { o }
                                  else if let Some(&gidx) = self.global_vars.get(name.as_str()) { -1000000 - gidx }
                                  else { 0 };
                        self.emit_load_var(off);
                    }
                    // rax = closure ptr, push it
                    self.emit1(0x50); // push closure ptr
                    // 取函数地址: mov rbx, [rax] (func_ptr at offset 0)
                    self.emit(&[0x48,0x8B,0x18]); // mov rbx, [rax]
                    // 设置参数: rcx=closure(env), rdx=arg0, r8=arg1, r9=arg2
                    self.emit1(0x59); // pop rcx (closure ptr = env)
                    let n = args.len().min(3);
                    let pop_shifted: &[&[u8]] = &[&[0x5A],&[0x41,0x58],&[0x41,0x59]]; // rdx, r8, r9
                    let extra = args.len().saturating_sub(3);
                    if extra > 0 {
                        self.emit(&[0x48,0x81,0xC4]); self.emit_i32((extra * 8) as i32);
                    }
                    for i in (0..n).rev() { self.emit(pop_shifted[i]); }
                    self.emit(&[0x48,0x83,0xEC,0x20]); // sub rsp,32 (shadow)
                    self.emit(&[0xFF,0xD3]); // call rbx (indirect)
                    self.emit(&[0x48,0x83,0xC4,0x20]); // add rsp,32
                } else {
                    // 直接调用 (普通函数)
                    for a in args.iter() {
                        self.compile_expr(a);
                        self.emit1(0x50); // push rax
                    }
                    let n = args.len().min(4);
                    for i in 0..n {
                        let disp = ((args.len() - 1 - i) * 8) as i32;
                        self.emit_load_stack_slot_to_call_reg(i, disp);
                    }
                    self.emit(&[0x48,0x83,0xEC,0x20]); // sub rsp,32 (shadow space)
                    self.emit1(0xE8); let cp=self.pos() as u32; self.emit_i32(0);
                    self.call_fixups.push((cp, name.clone()));
                    self.emit(&[0x48,0x83,0xC4,0x20]); // add rsp,32
                    if !args.is_empty() {
                        let cleanup = (args.len() * 8) as i32;
                        if cleanup <= 127 {
                            self.emit(&[0x48,0x83,0xC4]); self.emit1(cleanup as u8);
                        } else {
                            self.emit(&[0x48,0x81,0xC4]); self.emit_i32(cleanup);
                        }
                    }
                }
            }
            Expr::Index(base,idx) => {
                self.compile_expr(idx); self.emit1(0x50);
                self.compile_expr(base);
                // null guard: if rax==0 OR rax==Nova空(0xb1b1b1b4), skip dereference
                self.emit(&[0x48,0x85,0xC0]);                   // test rax,rax
                self.emit(&[0x74,0x13]);                         // je .null (+19)
                self.emit(&[0x48,0x3D,0xB4,0xB1,0xB1,0xB1]);   // cmp rax, 0xb1b1b1b4
                self.emit(&[0x74,0x0B]);                         // je .null (+11)
                self.emit(&[0x48,0x8B,0x40,0x10]); // mov rax,[rax+16] (data_ptr)
                self.emit1(0x59); // pop rcx (index)
                self.emit(&[0x48,0x8B,0x04,0xC8]); // mov rax,[rax+rcx*8]
                self.emit(&[0xEB,0x08]);       // jmp .done (+8)
                // .null:
                self.emit1(0x59);              // pop rcx (clean stack)
                self.emit(&[0x48,0xC7,0xC0,0xB4,0xB1,0xB1,0xB1]); // mov rax, Nova空
                // .done:
            }
            Expr::Member(base, field) => {
                let field_idx = self.find_field_index(field);
                self.compile_expr(base);
                if let Some(idx) = field_idx {
                    // null guard: if rax==0 or rax==sentinel → return sentinel
                    self.emit(&[0x48,0x85,0xC0]);                   // test rax,rax
                    self.emit(&[0x74,0x21]);                         // je .null (+33)
                    self.emit(&[0x48,0x3D,0xB4,0xB1,0xB1,0xB1]);   // cmp rax, 0xb1b1b1b4
                    self.emit(&[0x74,0x19]);                         // je .null (+25)
                    // list_get(base, idx): base in rax
                    self.emit(&[0x48,0x89,0xC1]); // mov rcx, rax
                    self.emit(&[0x48,0xC7,0xC2]); self.emit_i32(idx as i32); // mov rdx, idx
                    self.emit(&[0x48,0x83,0xEC,0x20]); // sub rsp,32
                    self.emit1(0xE8); let cp=self.pos() as u32; self.emit_i32(0);
                    self.call_fixups.push((cp, "__运行时_列表取".to_string()));
                    self.emit(&[0x48,0x83,0xC4,0x20]); // add rsp,32
                    self.emit(&[0xEB,0x07]);           // jmp .done (+7)
                    // .null:
                    self.emit(&[0x48,0xC7,0xC0,0xB4,0xB1,0xB1,0xB1]); // mov rax, Nova空
                    // .done:
                }
            }
            Expr::List(elems) => {
                // call __运行时_列表新建 → rax=新列表
                self.emit1(0xE8); let cp = self.pos() as u32; self.emit_i32(0);
                self.call_fixups.push((cp, "__运行时_列表新建".to_string()));
                if !elems.is_empty() {
                    // 逐个追加元素: push list; compile elem; mov rdx,rax; pop rcx; call append
                    for elem in elems {
                        self.emit1(0x50); // push rax (list)
                        self.compile_expr(elem);
                        self.emit(&[0x48,0x89,0xC2]); // mov rdx, rax (elem value)
                        self.emit1(0x59); // pop rcx (list)
                        self.emit1(0xE8); let cp2 = self.pos() as u32; self.emit_i32(0);
                        self.call_fixups.push((cp2, "__运行时_列表追加".to_string()));
                        // restore list ptr: list is still in rcx after append, but we need it in rax
                        self.emit(&[0x48,0x89,0xC8]); // mov rax, rcx
                    }
                }
            }
            Expr::StructNew(name, field_vals) => {
                // 结构体构造 → 创建列表
                self.emit1(0xE8); let cp = self.pos() as u32; self.emit_i32(0);
                self.call_fixups.push((cp, "__运行时_列表新建".to_string()));
                let is_positional = field_vals.iter().all(|(f, _)| f.is_empty());
                if is_positional {
                    // tuple style: 建 Name(v1, v2, ...)
                    for (_, val) in field_vals {
                        self.emit1(0x50); // push list
                        self.compile_expr(val);
                        self.emit(&[0x48,0x89,0xC2]); // mov rdx, rax
                        self.emit1(0x59); // pop rcx (list)
                        self.emit1(0xE8); let cp2 = self.pos() as u32; self.emit_i32(0);
                        self.call_fixups.push((cp2, "__运行时_列表追加".to_string()));
                        self.emit(&[0x48,0x89,0xC8]); // mov rax, rcx
                    }
                } else {
                    // brace style: 建 Name { f: v, ... } — order by struct definition
                    let fields = self.struct_defs.get(name).cloned().unwrap_or_default();
                    if fields.is_empty() {
                        // no struct def found, just append in order given
                        for (_, val) in field_vals {
                            self.emit1(0x50);
                            self.compile_expr(val);
                            self.emit(&[0x48,0x89,0xC2]); self.emit1(0x59);
                            self.emit1(0xE8); let cp2 = self.pos() as u32; self.emit_i32(0);
                            self.call_fixups.push((cp2, "__运行时_列表追加".to_string()));
                            self.emit(&[0x48,0x89,0xC8]);
                        }
                    } else {
                        for field_name in &fields {
                            self.emit1(0x50); // push list
                            let found = field_vals.iter().find(|(f, _)| f == field_name);
                            if let Some((_, val)) = found {
                                self.compile_expr(val);
                            } else {
                                self.emit(&[0x48,0x31,0xC0]); // xor rax,rax (default)
                            }
                            self.emit(&[0x48,0x89,0xC2]); // mov rdx, rax
                            self.emit1(0x59); // pop rcx
                            self.emit1(0xE8); let cp2 = self.pos() as u32; self.emit_i32(0);
                            self.call_fixups.push((cp2, "__运行时_列表追加".to_string()));
                            self.emit(&[0x48,0x89,0xC8]); // mov rax, rcx
                        }
                    }
                }
            }
            Expr::Lambda(params, body) => {
                // 闭包: 分析自由变量, 生成唯一函数, 创建闭包结构体
                let lname = format!("__closure_{}", self.lambda_counter);
                self.lambda_counter += 1;
                let all_free = Self::collect_free_vars(params, body);
                // 过滤: 只保留当前作用域中存在的变量
                let captures: Vec<String> = all_free.into_iter().filter(|v| {
                    self.vars.contains_key(v.as_str()) || self.global_vars.contains_key(v.as_str())
                }).collect();
                let nc = captures.len();
                // 延迟编译: 闭包体的第一个参数是env指针, 后续是用户参数
                self.pending_lambdas.push((lname.clone(), params.clone(), body.clone(), captures.clone()));
                self.funcs.insert(lname.clone(), 0);
                // 分配闭包结构体: (1+nc)*8 字节
                let alloc_sz = ((1 + nc) * 8) as i32;
                self.emit(&[0x48,0xC7,0xC1]); self.emit_i32(alloc_sz); // mov rcx, size
                self.emit1(0xE8); let cp = self.pos() as u32; self.emit_i32(0);
                self.call_fixups.push((cp, "__运行时_分配".to_string()));
                // rax = closure ptr, 保存到栈
                self.emit1(0x50); // push rax
                // 存函数地址到 [rax+0]: lea rcx, [rip+func]; mov [rax], rcx
                // 用 call fixup 方式计算函数地址
                self.emit(&[0x48,0x8D,0x0D]); // lea rcx, [rip+disp32]
                let lea_fixup = self.pos() as u32;
                self.emit_i32(0);
                self.call_fixups.push((lea_fixup, lname.clone()));
                self.emit(&[0x48,0x89,0x08]); // mov [rax], rcx
                // 存捕获变量
                for (i, cap) in captures.iter().enumerate() {
                    let off = if let Some(&o) = self.vars.get(cap.as_str()) { o }
                              else if let Some(&gidx) = self.global_vars.get(cap.as_str()) { -1000000 - gidx }
                              else { 0 };
                    self.emit_load_var(off); // rax = captured value
                    self.emit1(0x59); // pop rcx (closure ptr)
                    self.emit1(0x51); // push rcx (save it back)
                    let disp = ((1 + i) * 8) as i32;
                    self.emit(&[0x48,0x89,0x41]); self.emit1(disp as u8); // mov [rcx+disp8], rax
                }
                self.emit1(0x58); // pop rax (closure ptr = result)
            }
        }
    }

    fn find_field_index(&mut self, field: &str) -> Option<usize> {
        let mut matches: Vec<(String, usize)> = Vec::new();
        let mut sorted_names: Vec<&String> = self.struct_defs.keys().collect();
        sorted_names.sort();
        for sname in &sorted_names {
            if let Some(fields) = self.struct_defs.get(*sname) {
                if let Some(idx) = fields.iter().position(|f| f == field) {
                    matches.push((sname.to_string(), idx));
                }
            }
        }
        if matches.is_empty() { return None; }
        let first_idx = matches[0].1;
        if matches.len() > 1 && matches.iter().any(|(_, i)| *i != first_idx) {
            if !self.field_ambig_warned.contains(field) {
                self.field_ambig_warned.insert(field.to_string());
                if std::env::var("VERBOSE").unwrap_or_default() == "1" {
                    eprintln!("warn: 字段'{}'在多个结构体中索引不同: {}", field,
                        matches.iter().map(|(s, i)| format!("{}.{}={}", s, field, i)).collect::<Vec<_>>().join(", "));
                }
            }
        }
        Some(first_idx)
    }

    fn runtime_name_matches(name: &str, bytes: &[u8]) -> bool {
        name.as_bytes() == bytes
    }

    fn runtime_name_concat() -> String {
        String::from_utf8(vec![230, 139, 188, 230, 142, 165]).unwrap()
    }

    fn is_runtime_string_builtin(name: &str) -> bool {
        Self::runtime_name_matches(name, &[232, 189, 172, 229, 173, 151, 231, 172, 166, 228, 184, 178])
            || Self::runtime_name_matches(name, &[230, 139, 188, 230, 142, 165])
            || Self::runtime_name_matches(name, &[229, 173, 144, 228, 184, 178])
            || Self::runtime_name_matches(name, &[230, 155, 191, 230, 141, 162])
            || Self::runtime_name_matches(name, &[229, 164, 167, 229, 134, 153, 229, 140, 150])
            || Self::runtime_name_matches(name, &[229, 176, 143, 229, 134, 153, 229, 140, 150])
            || Self::runtime_name_matches(name, &[229, 173, 151, 231, 172, 166])
            || Self::runtime_name_matches(name, &[232, 175, 187, 230, 150, 135, 228, 187, 182])
            || Self::runtime_name_matches(name, &[229, 185, 179, 229, 143, 176, 95, 232, 175, 187, 230, 150, 135, 228, 187, 182])
    }

    fn is_str_expr(e: &Expr) -> bool {
        match e {
            Expr::Str(_) => true,
            Expr::Index(_, _) => true,
            Expr::Call(n, _) => Self::is_runtime_string_builtin(n.as_str()),
            Expr::Binary(l, BinOp::Add, r) => Self::is_str_expr(l) || Self::is_str_expr(r),
            _ => false,
        }
    }

    fn is_str_concat_expr(e: &Expr) -> bool {
        match e {
            Expr::Str(_) => true,
            Expr::Call(n, _) => Self::is_runtime_string_builtin(n.as_str()),
            Expr::Binary(l, BinOp::Add, r) => Self::is_str_concat_expr(l) || Self::is_str_concat_expr(r),
            _ => false,
        }
    }

    fn resolve_calls(&mut self) {
        let fixups = self.call_fixups.clone();
        for (pos, name) in &fixups {
            if let Some(&target) = self.funcs.get(name) {
                let rel = target as i32 - (*pos as i32 + 4);
                self.patch_i32(*pos as usize, rel);
            }
        }
    }
}

/// 返回种子编译器(Stage1)的关键字映射表: (别名, 正规形式)
pub fn seed_compiler_keywords() -> Vec<(&'static str, &'static str)> {
    vec![
        ("函数", "函数"), ("做法", "函数"), ("法", "函数"),
        ("定义", "定义"), ("称为", "定义"), ("叫做", "定义"),
        ("可变", "可变"),
        ("返回", "返回"), ("给出", "返回"),
        ("如果", "如果"), ("假如", "如果"), ("倘若", "如果"), ("要是", "如果"), ("若是", "如果"), ("每当", "如果"), ("凡是", "如果"),
        ("否则", "否则"), ("不然的话", "否则"), ("否则的话", "否则"), ("除此之外", "否则"),
        ("当", "当"), ("循环", "当"),
        ("导入", "导入"), ("入", "导入"),
        ("真", "真"), ("假", "假"), ("空", "空"),
        ("结构", "结构"), ("建", "建"),
        ("中断", "中断"), ("止", "中断"),
        ("继续", "继续"), ("续", "继续"),
        ("匹配", "匹配"), ("默认", "默认"),
        ("尝试", "尝试"), ("试", "尝试"),
        ("得", "得"),
        ("抛出", "抛出"), ("抛", "抛出"),
        ("每", "每"), ("每个", "每"),
        ("且", "且"), ("并且", "且"), ("而且", "且"),
        ("或", "或"), ("或者", "或"),
        ("非", "非"),
        ("等于", "=="), ("不等于", "!="),
        ("大于", ">"), ("小于", "<"),
        ("大于等于", ">="), ("小于等于", "<="),
    ]
}
