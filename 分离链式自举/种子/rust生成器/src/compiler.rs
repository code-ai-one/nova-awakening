use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const EXTERNAL_RUNTIME_SEED_MANIFEST_REL: &str = "../../自举/nova_core/编译器/前端入口_运行时种子.nova";
pub const EXTERNAL_RUNTIME_SEED_IMPORT_REL: &str = "../../../自举/nova_core/编译器/前端入口_运行时种子.nova";
pub const LOCAL_RUNTIME_SEED_IMPORT_REL: &str = "前端入口_运行时种子.nova";
pub const LOCAL_RUNTIME_SEED_REL: &str = "编译器/前端入口_运行时种子.nova";
pub const FRONTEND_ENTRY_REL: &str = "编译器/前端入口.nova";
pub const NOVA_ENTRY_REL: &str = "Nova.nova";

#[allow(dead_code)] // 保留字段: 项目元信息用于未来诊断/合同验证 (2026-04-17 A.4)
#[derive(Debug, Clone)]
pub struct KernelProject {
    pub workspace_root: PathBuf,
    pub kernel_root: PathBuf,
    pub manifest_source: PathBuf,
    pub local_sources: Vec<PathBuf>,
    pub entry_module_rel: PathBuf,
    pub localized_manifest_lines: Vec<String>,
    pub runtime_seed_source: PathBuf,
    // 6.1: AI基因系统项目合同扩展
    pub profile: String,
    pub target: String,
    pub entry_symbol: String,
    pub image_format: String,
    pub runtime_mode: String,
    pub debug_contract: String,
    pub emit: String,
    pub link_layout: String,
    pub subsystem_tags: Vec<String>,
    pub module_tags: BTreeMap<String, Vec<String>>,
}

impl KernelProject {
    pub fn manifest_text(&self) -> String {
        let mut text = self.localized_manifest_lines.join("\n");
        text.push('\n');
        text
    }

    pub fn entry_module_text(&self) -> String {
        self.entry_module_rel.to_string_lossy().to_string()
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeSeedSyncReport {
    pub kernel_root: PathBuf,
    pub runtime_seed_source: PathBuf,
    pub runtime_seed_target: PathBuf,
    pub frontend_entry_path: PathBuf,
    pub manifest_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct CapabilityRequirement {
    pub module: &'static str,
    pub reasons: &'static [&'static str],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrontTokenKind {
    Keyword(&'static str),
    Identifier(String),
    Number(String),
    String(String),
    Symbol(&'static str),
}

#[allow(dead_code)] // 保留column字段: 未来错误定位/LSP协议需要 (2026-04-17 A.4)
#[derive(Debug, Clone)]
pub struct FrontToken {
    pub kind: FrontTokenKind,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone)]
pub struct FrontScanSummary {
    pub module: String,
    pub token_count: usize,
    pub keyword_count: usize,
}

#[derive(Debug, Clone)]
pub struct ModuleSkeleton {
    pub module: String,
    pub imports: Vec<String>,
    pub functions: Vec<FunctionSignature>,
    pub structs: Vec<StructSignature>,
    pub globals: Vec<GlobalSignature>,
}

#[derive(Debug, Clone)]
pub struct FunctionSignature {
    pub name: String,
    pub params: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct StructSignature {
    pub name: String,
    pub fields: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct GlobalSignature {
    pub name: String,
    pub mutable: bool,
}

#[derive(Debug, Clone)]
pub enum TopLevelDecl {
    Import {
        path: String,
        token_start: usize,
        token_end: usize,
    },
    Function {
        signature: FunctionSignature,
        token_start: usize,
        token_end: usize,
        body_token_span: Option<(usize, usize)>,
        body_statements: Vec<BodyStmtSummary>,
    },
    Struct {
        signature: StructSignature,
        token_start: usize,
        token_end: usize,
    },
    Global {
        signature: GlobalSignature,
        token_start: usize,
        token_end: usize,
    },
}

#[derive(Debug, Clone)]
pub struct ParsedModule {
    pub module: String,
    pub declarations: Vec<TopLevelDecl>,
}

#[derive(Debug, Clone)]
pub struct BodyStmtSummary {
    pub kind: &'static str,
    pub token_start: usize,
    pub token_end: usize,
    pub line_start: usize,
    pub line_end: usize,
    pub ast: BodyStmtAst,
    pub target_expr: Option<ParsedExpr>,
    pub primary_expr: Option<ParsedExpr>,
}

#[derive(Debug, Clone)]
pub struct ParsedExpr {
    pub kind: &'static str,
    pub token_start: usize,
    pub token_end: usize,
    pub tree: ExprTree,
}

#[allow(dead_code)] // 保留op/member/name字段: AST反射+未来codegen需要 (2026-04-17 A.4)
#[derive(Debug, Clone)]
pub enum ExprTree {
    Literal,
    Identifier(String),
    Unary {
        op: &'static str,
        expr: Box<ParsedExpr>,
    },
    Binary {
        op: &'static str,
        left: Box<ParsedExpr>,
        right: Box<ParsedExpr>,
    },
    Call {
        callee: Box<ParsedExpr>,
        args: Vec<ParsedExpr>,
    },
    Index {
        target: Box<ParsedExpr>,
        index: Box<ParsedExpr>,
    },
    Member {
        target: Box<ParsedExpr>,
        member: String,
    },
    List {
        items: Vec<ParsedExpr>,
    },
    Struct {
        name: String,
        fields: Vec<StructFieldExpr>,
    },
    Unknown,
}

#[allow(dead_code)] // 保留name字段: AST反射需要 (2026-04-17 A.4)
#[derive(Debug, Clone)]
pub struct StructFieldExpr {
    pub name: String,
    pub value: ParsedExpr,
}

impl ParsedExpr {
    fn new(tree: ExprTree, token_start: usize, token_end: usize) -> Self {
        let kind = tree.kind();
        Self {
            kind,
            token_start,
            token_end,
            tree,
        }
    }

    pub fn node_count(&self) -> usize {
        match &self.tree {
            ExprTree::Literal | ExprTree::Identifier(_) | ExprTree::Unknown => 1,
            ExprTree::Unary { expr, .. } => 1 + expr.node_count(),
            ExprTree::Binary { left, right, .. } => 1 + left.node_count() + right.node_count(),
            ExprTree::Call { callee, args } => {
                1 + callee.node_count() + args.iter().map(ParsedExpr::node_count).sum::<usize>()
            }
            ExprTree::Index { target, index } => 1 + target.node_count() + index.node_count(),
            ExprTree::Member { target, .. } => 1 + target.node_count(),
            ExprTree::List { items } => 1 + items.iter().map(ParsedExpr::node_count).sum::<usize>(),
            ExprTree::Struct { fields, .. } => {
                1 + fields.iter().map(|field| field.value.node_count()).sum::<usize>()
            }
        }
    }

    pub fn max_depth(&self) -> usize {
        match &self.tree {
            ExprTree::Literal | ExprTree::Identifier(_) | ExprTree::Unknown => 1,
            ExprTree::Unary { expr, .. } => 1 + expr.max_depth(),
            ExprTree::Binary { left, right, .. } => 1 + left.max_depth().max(right.max_depth()),
            ExprTree::Call { callee, args } => {
                1 + callee.max_depth().max(args.iter().map(ParsedExpr::max_depth).max().unwrap_or(0))
            }
            ExprTree::Index { target, index } => 1 + target.max_depth().max(index.max_depth()),
            ExprTree::Member { target, .. } => 1 + target.max_depth(),
            ExprTree::List { items } => 1 + items.iter().map(ParsedExpr::max_depth).max().unwrap_or(0),
            ExprTree::Struct { fields, .. } => {
                1 + fields.iter().map(|field| field.value.max_depth()).max().unwrap_or(0)
            }
        }
    }

    pub fn unknown_nodes(&self) -> usize {
        match &self.tree {
            ExprTree::Unknown => 1,
            ExprTree::Literal | ExprTree::Identifier(_) => 0,
            ExprTree::Unary { expr, .. } => expr.unknown_nodes(),
            ExprTree::Binary { left, right, .. } => left.unknown_nodes() + right.unknown_nodes(),
            ExprTree::Call { callee, args } => {
                callee.unknown_nodes() + args.iter().map(ParsedExpr::unknown_nodes).sum::<usize>()
            }
            ExprTree::Index { target, index } => target.unknown_nodes() + index.unknown_nodes(),
            ExprTree::Member { target, .. } => target.unknown_nodes(),
            ExprTree::List { items } => items.iter().map(ParsedExpr::unknown_nodes).sum(),
            ExprTree::Struct { fields, .. } => {
                fields.iter().map(|field| field.value.unknown_nodes()).sum()
            }
        }
    }
}

impl ExprTree {
    fn kind(&self) -> &'static str {
        match self {
            ExprTree::Literal => "字面量",
            ExprTree::Identifier(_) => "标识符",
            ExprTree::Unary { .. } => "一元",
            ExprTree::Binary { .. } => "二元",
            ExprTree::Call { .. } => "调用",
            ExprTree::Index { .. } => "索引",
            ExprTree::Member { .. } => "成员访问",
            ExprTree::List { .. } => "列表构造",
            ExprTree::Struct { .. } => "结构构造",
            ExprTree::Unknown => "未知",
        }
    }
}

#[allow(dead_code)] // 保留mutable字段: 语义分析+后端需要 (2026-04-17 A.4)
#[derive(Debug, Clone)]
pub enum BodyStmtAst {
    Define {
        mutable: bool,
        name: Option<String>,
        value: Option<ParsedExpr>,
    },
    Return {
        value: Option<ParsedExpr>,
    },
    If {
        condition: Option<ParsedExpr>,
        then_body: Vec<BodyStmtSummary>,
        else_body: Vec<BodyStmtSummary>,
    },
    While {
        condition: Option<ParsedExpr>,
        body: Vec<BodyStmtSummary>,
    },
    Break,
    Continue,
    Try,
    Throw {
        value: Option<ParsedExpr>,
    },
    Assign {
        op: &'static str,
        target: Option<ParsedExpr>,
        value: Option<ParsedExpr>,
    },
    Expr {
        value: Option<ParsedExpr>,
    },
}

impl BodyStmtAst {
    pub fn tag(&self) -> &'static str {
        match self {
            BodyStmtAst::Define { .. } => "定义",
            BodyStmtAst::Return { .. } => "返回",
            BodyStmtAst::If { .. } => "如果",
            BodyStmtAst::While { .. } => "当",
            BodyStmtAst::Break => "中断",
            BodyStmtAst::Continue => "继续",
            BodyStmtAst::Try => "尝试",
            BodyStmtAst::Throw { .. } => "抛出",
            BodyStmtAst::Assign { op: "+=", .. } => "加等赋值",
            BodyStmtAst::Assign { op: "-=", .. } => "减等赋值",
            BodyStmtAst::Assign { .. } => "赋值",
            BodyStmtAst::Expr { .. } => "表达式",
        }
    }
}

#[derive(Clone, Copy)]
struct BinaryOp {
    symbol: &'static str,
    precedence: u8,
}

struct ExprParser<'a> {
    tokens: &'a [FrontToken],
    cursor: usize,
    end: usize,
}

impl<'a> ExprParser<'a> {
    fn new(tokens: &'a [FrontToken], start: usize, end: usize) -> Self {
        Self { tokens, cursor: start, end }
    }

    fn is_done(&self) -> bool {
        self.cursor > self.end
    }

    fn peek_kind(&self) -> Option<&'a FrontTokenKind> {
        if self.cursor > self.end {
            None
        } else {
            self.tokens.get(self.cursor).map(|token| &token.kind)
        }
    }

    fn consume_symbol(&mut self, symbol: &str) -> Option<usize> {
        if matches!(self.peek_kind(), Some(FrontTokenKind::Symbol(found)) if *found == symbol) {
            let index = self.cursor;
            self.cursor += 1;
            Some(index)
        } else {
            None
        }
    }

    fn peek_unary_op(&self) -> Option<&'static str> {
        match self.peek_kind() {
            Some(FrontTokenKind::Symbol("-")) => Some("-"),
            Some(FrontTokenKind::Symbol("!")) => Some("!"),
            Some(FrontTokenKind::Symbol("~")) => Some("~"),
            Some(FrontTokenKind::Keyword("非")) => Some("非"),
            _ => None,
        }
    }

    fn peek_binary_op(&self) -> Option<BinaryOp> {
        match self.peek_kind() {
            Some(FrontTokenKind::Keyword("或")) | Some(FrontTokenKind::Symbol("||")) => {
                Some(BinaryOp { symbol: "||", precedence: 1 })
            }
            Some(FrontTokenKind::Keyword("且")) | Some(FrontTokenKind::Symbol("&&")) => {
                Some(BinaryOp { symbol: "&&", precedence: 2 })
            }
            Some(FrontTokenKind::Symbol("|")) => Some(BinaryOp { symbol: "|", precedence: 3 }),
            Some(FrontTokenKind::Symbol("^")) => Some(BinaryOp { symbol: "^", precedence: 4 }),
            Some(FrontTokenKind::Symbol("&")) => Some(BinaryOp { symbol: "&", precedence: 5 }),
            Some(FrontTokenKind::Symbol("==")) => Some(BinaryOp { symbol: "==", precedence: 6 }),
            Some(FrontTokenKind::Symbol("!=")) => Some(BinaryOp { symbol: "!=", precedence: 6 }),
            Some(FrontTokenKind::Symbol("<")) => Some(BinaryOp { symbol: "<", precedence: 7 }),
            Some(FrontTokenKind::Symbol("<=")) => Some(BinaryOp { symbol: "<=", precedence: 7 }),
            Some(FrontTokenKind::Symbol(">")) => Some(BinaryOp { symbol: ">", precedence: 7 }),
            Some(FrontTokenKind::Symbol(">=")) => Some(BinaryOp { symbol: ">=", precedence: 7 }),
            Some(FrontTokenKind::Symbol("<<")) => Some(BinaryOp { symbol: "<<", precedence: 8 }),
            Some(FrontTokenKind::Symbol(">>")) => Some(BinaryOp { symbol: ">>", precedence: 8 }),
            Some(FrontTokenKind::Symbol("+")) => Some(BinaryOp { symbol: "+", precedence: 9 }),
            Some(FrontTokenKind::Symbol("-")) => Some(BinaryOp { symbol: "-", precedence: 9 }),
            Some(FrontTokenKind::Symbol("*")) => Some(BinaryOp { symbol: "*", precedence: 10 }),
            Some(FrontTokenKind::Symbol("/")) => Some(BinaryOp { symbol: "/", precedence: 10 }),
            Some(FrontTokenKind::Symbol("%")) => Some(BinaryOp { symbol: "%", precedence: 10 }),
            _ => None,
        }
    }

    fn parse_expression(&mut self, min_precedence: u8) -> Option<ParsedExpr> {
        let mut left = self.parse_prefix()?;
        loop {
            let op = match self.peek_binary_op() {
                Some(op) if op.precedence >= min_precedence => op,
                _ => break,
            };
            self.cursor += 1;
            let right = self.parse_expression(op.precedence + 1)?;
            let token_start = left.token_start;
            let token_end = right.token_end;
            left = ParsedExpr::new(
                ExprTree::Binary {
                    op: op.symbol,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                token_start,
                token_end,
            );
        }
        Some(left)
    }

    fn parse_prefix(&mut self) -> Option<ParsedExpr> {
        if let Some(op) = self.peek_unary_op() {
            let token_start = self.cursor;
            self.cursor += 1;
            let expr = self.parse_expression(11)?;
            let token_end = expr.token_end;
            return Some(ParsedExpr::new(
                ExprTree::Unary {
                    op,
                    expr: Box::new(expr),
                },
                token_start,
                token_end,
            ));
        }

        let primary = self.parse_primary()?;
        self.parse_postfix(primary)
    }

    fn parse_primary(&mut self) -> Option<ParsedExpr> {
        match self.peek_kind()? {
            FrontTokenKind::Number(_) | FrontTokenKind::String(_) => {
                let index = self.cursor;
                self.cursor += 1;
                Some(ParsedExpr::new(ExprTree::Literal, index, index))
            }
            FrontTokenKind::Keyword("真")
            | FrontTokenKind::Keyword("假")
            | FrontTokenKind::Keyword("空") => {
                let index = self.cursor;
                self.cursor += 1;
                Some(ParsedExpr::new(ExprTree::Literal, index, index))
            }
            FrontTokenKind::Keyword("建") => self.parse_builder_struct(),
            FrontTokenKind::Identifier(name) => {
                let index = self.cursor;
                let name = name.clone();
                self.cursor += 1;
                Some(ParsedExpr::new(ExprTree::Identifier(name), index, index))
            }
            FrontTokenKind::Symbol("(") => self.parse_grouped(),
            FrontTokenKind::Symbol("[") => self.parse_list_literal(),
            _ => {
                let index = self.cursor;
                self.cursor += 1;
                Some(ParsedExpr::new(ExprTree::Unknown, index, index))
            }
        }
    }

    fn parse_grouped(&mut self) -> Option<ParsedExpr> {
        let open = self.consume_symbol("(")?;
        let mut expr = self.parse_expression(1)?;
        let close = self.consume_symbol(")")?;
        expr.token_start = open;
        expr.token_end = close;
        Some(expr)
    }

    fn parse_list_literal(&mut self) -> Option<ParsedExpr> {
        let open = self.consume_symbol("[")?;
        let mut items = Vec::new();
        let close = if let Some(close) = self.consume_symbol("]") {
            close
        } else {
            loop {
                items.push(self.parse_expression(1)?);
                if self.consume_symbol(",").is_some() {
                    if let Some(close) = self.consume_symbol("]") {
                        break close;
                    }
                    continue;
                }
                break self.consume_symbol("]")?;
            }
        };
        Some(ParsedExpr::new(ExprTree::List { items }, open, close))
    }

    fn parse_builder_struct(&mut self) -> Option<ParsedExpr> {
        let builder_start = self.cursor;
        self.cursor += 1;
        let name = extract_name(self.tokens, self.cursor)?;
        self.cursor += 1;
        self.parse_struct_literal(name, builder_start)
    }

    fn parse_postfix(&mut self, mut expr: ParsedExpr) -> Option<ParsedExpr> {
        loop {
            match self.peek_kind() {
                Some(FrontTokenKind::Symbol("(")) => expr = self.parse_call(expr)?,
                Some(FrontTokenKind::Symbol("[")) => expr = self.parse_index(expr)?,
                Some(FrontTokenKind::Symbol(".")) => expr = self.parse_member(expr)?,
                Some(FrontTokenKind::Symbol("{")) => {
                    let name = match &expr.tree {
                        ExprTree::Identifier(name) => Some(name.clone()),
                        _ => None,
                    };
                    if let Some(name) = name {
                        expr = self.parse_struct_literal(name, expr.token_start)?;
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }
        Some(expr)
    }

    fn parse_call(&mut self, callee: ParsedExpr) -> Option<ParsedExpr> {
        let token_start = callee.token_start;
        self.consume_symbol("(")?;
        let mut args = Vec::new();
        let close = if let Some(close) = self.consume_symbol(")") {
            close
        } else {
            loop {
                args.push(self.parse_expression(1)?);
                if self.consume_symbol(",").is_some() {
                    if let Some(close) = self.consume_symbol(")") {
                        break close;
                    }
                    continue;
                }
                break self.consume_symbol(")")?;
            }
        };
        Some(ParsedExpr::new(
            ExprTree::Call {
                callee: Box::new(callee),
                args,
            },
            token_start,
            close,
        ))
    }

    fn parse_index(&mut self, target: ParsedExpr) -> Option<ParsedExpr> {
        let token_start = target.token_start;
        self.consume_symbol("[")?;
        let index_expr = self.parse_expression(1)?;
        let close = self.consume_symbol("]")?;
        Some(ParsedExpr::new(
            ExprTree::Index {
                target: Box::new(target),
                index: Box::new(index_expr),
            },
            token_start,
            close,
        ))
    }

    fn parse_member(&mut self, target: ParsedExpr) -> Option<ParsedExpr> {
        let token_start = target.token_start;
        self.consume_symbol(".")?;
        let member = extract_name(self.tokens, self.cursor)?;
        let member_index = self.cursor;
        self.cursor += 1;
        Some(ParsedExpr::new(
            ExprTree::Member {
                target: Box::new(target),
                member,
            },
            token_start,
            member_index,
        ))
    }

    fn parse_struct_literal(&mut self, name: String, expr_start: usize) -> Option<ParsedExpr> {
        self.consume_symbol("{")?;
        let mut fields = Vec::new();
        let close = if let Some(close) = self.consume_symbol("}") {
            close
        } else {
            loop {
                let field_name = extract_name(self.tokens, self.cursor)?;
                self.cursor += 1;
                self.consume_symbol(":")?;
                let value = self.parse_expression(1)?;
                fields.push(StructFieldExpr {
                    name: field_name,
                    value,
                });
                if self.consume_symbol(",").is_some() {
                    if let Some(close) = self.consume_symbol("}") {
                        break close;
                    }
                    continue;
                }
                break self.consume_symbol("}")?;
            }
        };
        Some(ParsedExpr::new(
            ExprTree::Struct { name, fields },
            expr_start,
            close,
        ))
    }
}

pub fn scan_required_modules(project: &KernelProject) -> Result<Vec<FrontScanSummary>, String> {
    let mut result = Vec::new();
    for item in current_kernel_requirements(project) {
        let tokens = read_module_tokens(project, item.module, "前端扫描失败")?;
        let keyword_count = tokens
            .iter()
            .filter(|token| matches!(token.kind, FrontTokenKind::Keyword(_)))
            .count();
        result.push(FrontScanSummary {
            module: item.module.to_string(),
            token_count: tokens.len(),
            keyword_count,
        });
    }
    Ok(result)
}

pub fn parse_required_modules(project: &KernelProject) -> Result<Vec<ParsedModule>, String> {
    let modules = current_kernel_requirements(project)
        .into_iter()
        .map(|item| item.module.to_string())
        .collect::<Vec<_>>();
    parse_named_modules(project, &modules, "顶层 AST 解析失败")
}

pub fn parse_required_module_skeletons(project: &KernelProject) -> Result<Vec<ModuleSkeleton>, String> {
    let mut result = Vec::new();
    for item in current_kernel_requirements(project) {
        let tokens = read_module_tokens(project, item.module, "顶层骨架提取失败")?;
        result.push(extract_module_skeleton(item.module, &tokens));
    }
    Ok(result)
}

pub fn parse_stage1_modules(project: &KernelProject) -> Result<Vec<ParsedModule>, String> {
    let modules = stage1_module_list(project);
    parse_named_modules(project, &modules, "Stage1 AST 解析失败")
}

fn stage1_module_list(project: &KernelProject) -> Vec<String> {
    let mut modules = project
        .local_sources
        .iter()
        .filter_map(|path| {
            let text = path.to_string_lossy();
            if text.ends_with(".nova") {
                Some(text.to_string())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    modules.sort();
    modules
}

fn parse_named_modules(
    project: &KernelProject,
    modules: &[String],
    error_prefix: &str,
) -> Result<Vec<ParsedModule>, String> {
    let mut result = Vec::new();
    for module in modules {
        let tokens = read_module_tokens(project, module, error_prefix)?;
        result.push(extract_parsed_module(module, &tokens));
    }
    Ok(result)
}

fn read_module_tokens(
    project: &KernelProject,
    module: &str,
    error_prefix: &str,
) -> Result<Vec<FrontToken>, String> {
    let path = project.kernel_root.join(module);
    let source = fs::read_to_string(&path)
        .map_err(|e| format!("读取模块失败: {} ({})", path.display(), e))?;
    scan_source(&source).map_err(|e| format!("{}: {} ({})", error_prefix, module, e))
}

pub fn scan_source(source: &str) -> Result<Vec<FrontToken>, String> {
    let chars: Vec<char> = source.chars().collect();
    let mut tokens = Vec::new();
    let mut balance_stack: Vec<(char, usize, usize)> = Vec::new();
    let mut index = 0usize;
    let mut line = 1usize;
    let mut column = 1usize;

    if peek_char(&chars, 0) == Some('\u{FEFF}') {
        index = 1;
    }

    while index < chars.len() {
        let ch = chars[index];

        if ch == ' ' || ch == '\t' || ch == '\r' {
            index += 1;
            column += 1;
            continue;
        }
        if ch == '\n' {
            index += 1;
            line += 1;
            column = 1;
            continue;
        }
        if ch == '/' && peek_char(&chars, index + 1) == Some('/') {
            index += 2;
            column += 2;
            while index < chars.len() && chars[index] != '\n' {
                index += 1;
                column += 1;
            }
            continue;
        }
        if ch == '"' {
            let start_line = line;
            let start_column = column;
            let mut value = String::new();
            index += 1;
            column += 1;
            let mut closed = false;
            while index < chars.len() {
                let current = chars[index];
                if current == '"' {
                    index += 1;
                    column += 1;
                    closed = true;
                    break;
                }
                if current == '\\' {
                    let escaped = peek_char(&chars, index + 1)
                        .ok_or_else(|| format!("字符串转义不完整 @{}:{}", line, column))?;
                    value.push(match escaped {
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        '\\' => '\\',
                        '"' => '"',
                        other => other,
                    });
                    index += 2;
                    column += 2;
                    continue;
                }
                if current == '\n' {
                    return Err(format!("字符串未闭合 @{}:{}", start_line, start_column));
                }
                value.push(current);
                index += 1;
                column += 1;
            }
            if !closed {
                return Err(format!("字符串未闭合 @{}:{}", start_line, start_column));
            }
            tokens.push(FrontToken {
                kind: FrontTokenKind::String(value),
                line: start_line,
                column: start_column,
            });
            continue;
        }
        if is_number_start(ch) {
            let start_line = line;
            let start_column = column;
            let start = index;
            index += 1;
            column += 1;
            while index < chars.len() && chars[index].is_ascii_digit() {
                index += 1;
                column += 1;
            }
            if index < chars.len()
                && chars[index] == '.'
                && peek_char(&chars, index + 1).map(|c| c.is_ascii_digit()).unwrap_or(false)
            {
                index += 1;
                column += 1;
                while index < chars.len() && chars[index].is_ascii_digit() {
                    index += 1;
                    column += 1;
                }
            }
            let value: String = chars[start..index].iter().collect();
            tokens.push(FrontToken {
                kind: FrontTokenKind::Number(value),
                line: start_line,
                column: start_column,
            });
            continue;
        }
        if is_identifier_start(ch) {
            let start_line = line;
            let start_column = column;
            let start = index;
            index += 1;
            column += 1;
            while index < chars.len() && is_identifier_continue(chars[index]) {
                index += 1;
                column += 1;
            }
            let text: String = chars[start..index].iter().collect();
            let kind = match normalize_operator_word(&text) {
                Some(op) => FrontTokenKind::Symbol(op),
                None => match normalize_keyword(&text) {
                    Some(keyword) => FrontTokenKind::Keyword(keyword),
                    None => FrontTokenKind::Identifier(text),
                },
            };
            tokens.push(FrontToken {
                kind,
                line: start_line,
                column: start_column,
            });
            continue;
        }

        let start_line = line;
        let start_column = column;
        let (symbol, width) = scan_symbol(&chars, index)
            .ok_or_else(|| format!("不支持的字符 '{}' @{}:{}", ch, line, column))?;

        if symbol == "(" || symbol == "{" || symbol == "[" {
            balance_stack.push((symbol.chars().next().unwrap_or(' '), start_line, start_column));
        } else if symbol == ")" || symbol == "}" || symbol == "]" {
            let expected = match symbol {
                ")" => '(',
                "}" => '{',
                "]" => '[',
                _ => ' ',
            };
            let top = balance_stack.pop().ok_or_else(|| {
                format!("出现未匹配的闭合符号 {} @{}:{}", symbol, start_line, start_column)
            })?;
            if top.0 != expected {
                return Err(format!(
                    "括号不匹配: 打开符号 {} @{}:{}，关闭符号 {} @{}:{}",
                    top.0, top.1, top.2, symbol, start_line, start_column
                ));
            }
        }

        tokens.push(FrontToken {
            kind: FrontTokenKind::Symbol(symbol),
            line: start_line,
            column: start_column,
        });
        index += width;
        column += width;
    }

    if let Some((open, open_line, open_column)) = balance_stack.last().copied() {
        return Err(format!(
            "存在未闭合符号 {} @{}:{}",
            open, open_line, open_column
        ));
    }

    Ok(tokens)
}

fn extract_module_skeleton(module: &str, tokens: &[FrontToken]) -> ModuleSkeleton {
    let mut skeleton = ModuleSkeleton {
        module: module.to_string(),
        imports: Vec::new(),
        functions: Vec::new(),
        structs: Vec::new(),
        globals: Vec::new(),
    };
    let mut brace_depth = 0i32;
    let mut index = 0usize;

    while index < tokens.len() {
        match &tokens[index].kind {
            FrontTokenKind::Symbol("{") => {
                brace_depth += 1;
                index += 1;
                continue;
            }
            FrontTokenKind::Symbol("}") => {
                if brace_depth > 0 {
                    brace_depth -= 1;
                }
                index += 1;
                continue;
            }
            _ => {}
        }

        if brace_depth == 0 {
            match &tokens[index].kind {
                FrontTokenKind::Keyword("导入") => {
                    if let Some(path) = extract_import_path(tokens, index + 1) {
                        skeleton.imports.push(path);
                    }
                }
                FrontTokenKind::Keyword("函数") => {
                    if let Some(signature) = extract_function_signature(tokens, index + 1) {
                        skeleton.functions.push(signature);
                    }
                }
                FrontTokenKind::Keyword("结构") => {
                    if let Some(signature) = extract_struct_signature(tokens, index + 1) {
                        skeleton.structs.push(signature);
                    }
                }
                FrontTokenKind::Keyword("定义") => {
                    if let Some(signature) = extract_global_signature(tokens, index + 1) {
                        skeleton.globals.push(signature);
                    }
                }
                _ => {}
            }
        }

        index += 1;
    }

    skeleton
}

fn extract_parsed_module(module: &str, tokens: &[FrontToken]) -> ParsedModule {
    let mut declarations = Vec::new();
    let mut brace_depth = 0i32;
    let mut index = 0usize;

    while index < tokens.len() {
        match &tokens[index].kind {
            FrontTokenKind::Symbol("{") => {
                brace_depth += 1;
                index += 1;
                continue;
            }
            FrontTokenKind::Symbol("}") => {
                if brace_depth > 0 {
                    brace_depth -= 1;
                }
                index += 1;
                continue;
            }
            _ => {}
        }

        if brace_depth == 0 {
            match &tokens[index].kind {
                FrontTokenKind::Keyword("导入") => {
                    if let Some(path) = extract_import_path(tokens, index + 1) {
                        let token_end = (index + 3).min(tokens.len().saturating_sub(1));
                        declarations.push(TopLevelDecl::Import {
                            path,
                            token_start: index,
                            token_end,
                        });
                    }
                }
                FrontTokenKind::Keyword("函数") => {
                    if let Some(signature) = extract_function_signature(tokens, index + 1) {
                        let (token_end, body_token_span) = extract_function_extent(tokens, index + 1);
                        let body_statements = body_token_span
                            .map(|span| extract_body_statements(tokens, span))
                            .unwrap_or_default();
                        declarations.push(TopLevelDecl::Function {
                            signature,
                            token_start: index,
                            token_end,
                            body_token_span,
                            body_statements,
                        });
                    }
                }
                FrontTokenKind::Keyword("结构") => {
                    if let Some(signature) = extract_struct_signature(tokens, index + 1) {
                        let token_end = extract_struct_extent(tokens, index + 1);
                        declarations.push(TopLevelDecl::Struct {
                            signature,
                            token_start: index,
                            token_end,
                        });
                    }
                }
                FrontTokenKind::Keyword("定义") => {
                    if let Some(signature) = extract_global_signature(tokens, index + 1) {
                        let token_end = extract_global_extent(tokens, index + 1);
                        declarations.push(TopLevelDecl::Global {
                            signature,
                            token_start: index,
                            token_end,
                        });
                    }
                }
                _ => {}
            }
        }

        index += 1;
    }

    ParsedModule {
        module: module.to_string(),
        declarations,
    }
}

fn peek_char(chars: &[char], index: usize) -> Option<char> {
    chars.get(index).copied()
}

fn is_top_level_decl_start(kind: &FrontTokenKind) -> bool {
    matches!(
        kind,
        FrontTokenKind::Keyword("导入")
            | FrontTokenKind::Keyword("函数")
            | FrontTokenKind::Keyword("结构")
            | FrontTokenKind::Keyword("定义")
    )
}

fn matches_keyword(tokens: &[FrontToken], index: usize, keyword: &str) -> bool {
    matches!(tokens.get(index).map(|token| &token.kind), Some(FrontTokenKind::Keyword(name)) if *name == keyword)
}

fn extract_name(tokens: &[FrontToken], index: usize) -> Option<String> {
    match tokens.get(index).map(|token| &token.kind) {
        Some(FrontTokenKind::Identifier(name)) => Some(name.clone()),
        Some(FrontTokenKind::Keyword(name)) if *name == "真" || *name == "假" || *name == "空" => {
            Some((*name).to_string())
        }
        _ => None,
    }
}

fn extract_function_signature(tokens: &[FrontToken], index: usize) -> Option<FunctionSignature> {
    let name = extract_name(tokens, index)?;
    if !matches!(tokens.get(index + 1).map(|token| &token.kind), Some(FrontTokenKind::Symbol("("))) {
        return Some(FunctionSignature { name, params: Vec::new() });
    }
    let params = extract_name_list(tokens, index + 1, "(", ")");
    Some(FunctionSignature { name, params })
}

fn extract_struct_signature(tokens: &[FrontToken], index: usize) -> Option<StructSignature> {
    let name = extract_name(tokens, index)?;
    let next_kind = tokens.get(index + 1).map(|token| &token.kind);
    let fields = match next_kind {
        Some(FrontTokenKind::Symbol("(")) => extract_name_list(tokens, index + 1, "(", ")"),
        Some(FrontTokenKind::Symbol("{")) => extract_name_list(tokens, index + 1, "{", "}"),
        _ => Vec::new(),
    };
    Some(StructSignature { name, fields })
}

fn extract_global_signature(tokens: &[FrontToken], index: usize) -> Option<GlobalSignature> {
    let mut probe = index;
    let mutable = matches_keyword(tokens, probe, "可变");
    if mutable {
        probe += 1;
    }
    let name = extract_name(tokens, probe)?;
    Some(GlobalSignature { name, mutable })
}

fn extract_name_list(tokens: &[FrontToken], start_index: usize, open: &str, close: &str) -> Vec<String> {
    if !matches!(tokens.get(start_index).map(|token| &token.kind), Some(FrontTokenKind::Symbol(symbol)) if *symbol == open) {
        return Vec::new();
    }

    let mut result = Vec::new();
    let mut depth = 0i32;
    let mut index = start_index;
    while index < tokens.len() {
        match tokens.get(index).map(|token| &token.kind) {
            Some(FrontTokenKind::Symbol(symbol)) if *symbol == open => {
                depth += 1;
            }
            Some(FrontTokenKind::Symbol(symbol)) if *symbol == close => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            _ => {}
        }

        if depth == 1 {
            if let Some(name) = extract_name(tokens, index) {
                if result.last() != Some(&name) {
                    result.push(name);
                }
            }
        }
        index += 1;
    }
    result
}

fn extract_function_extent(tokens: &[FrontToken], index: usize) -> (usize, Option<(usize, usize)>) {
    let after_name = index + 1;
    let mut body_token_span = None;
    let token_end = if matches!(tokens.get(after_name).map(|token| &token.kind), Some(FrontTokenKind::Symbol("("))) {
        if let Some(params_end) = find_matching_symbol(tokens, after_name, "(", ")") {
            let body_start = params_end + 1;
            if matches!(tokens.get(body_start).map(|token| &token.kind), Some(FrontTokenKind::Symbol("{"))) {
                if let Some(body_end) = find_matching_symbol(tokens, body_start, "{", "}") {
                    body_token_span = Some((body_start, body_end));
                    body_end
                } else {
                    params_end
                }
            } else {
                params_end
            }
        } else {
            index
        }
    } else {
        index
    };
    (token_end, body_token_span)
}

fn extract_struct_extent(tokens: &[FrontToken], index: usize) -> usize {
    let next = index + 1;
    match tokens.get(next).map(|token| &token.kind) {
        Some(FrontTokenKind::Symbol("(")) => find_matching_symbol(tokens, next, "(", ")").unwrap_or(index),
        Some(FrontTokenKind::Symbol("{")) => find_matching_symbol(tokens, next, "{", "}").unwrap_or(index),
        _ => index,
    }
}

fn extract_global_extent(tokens: &[FrontToken], index: usize) -> usize {
    let start = index.saturating_sub(1);
    find_next_top_level_boundary(tokens, start)
}

fn find_matching_symbol(tokens: &[FrontToken], start: usize, open: &str, close: &str) -> Option<usize> {
    let mut depth = 0i32;
    let mut index = start;
    while index < tokens.len() {
        match tokens.get(index).map(|token| &token.kind) {
            Some(FrontTokenKind::Symbol(symbol)) if *symbol == open => {
                depth += 1;
            }
            Some(FrontTokenKind::Symbol(symbol)) if *symbol == close => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
        index += 1;
    }
    None
}

fn find_next_top_level_boundary(tokens: &[FrontToken], start: usize) -> usize {
    let mut depth = 0i32;
    let mut index = start + 1;
    while index < tokens.len() {
        match &tokens[index].kind {
            FrontTokenKind::Symbol("{") => depth += 1,
            FrontTokenKind::Symbol("}") => {
                if depth > 0 {
                    depth -= 1;
                }
            }
            kind if depth == 0 && is_top_level_decl_start(kind) => {
                return index.saturating_sub(1);
            }
            _ => {}
        }
        index += 1;
    }
    tokens.len().saturating_sub(1)
}

fn extract_body_statements(tokens: &[FrontToken], body_span: (usize, usize)) -> Vec<BodyStmtSummary> {
    let (start, end) = body_span;
    if end <= start + 1 {
        return Vec::new();
    }

    let mut result = Vec::new();
    let mut brace_depth = 0i32;
    let mut paren_depth = 0i32;
    let mut bracket_depth = 0i32;
    let mut stmt_start = start + 1;
    let mut index = start + 1;

    while index < end {
        match tokens.get(index).map(|token| &token.kind) {
            Some(FrontTokenKind::Symbol("{")) => {
                brace_depth += 1;
            }
            Some(FrontTokenKind::Symbol("}")) => {
                if brace_depth > 0 {
                    brace_depth -= 1;
                    if brace_depth == 0 && paren_depth == 0 && bracket_depth == 0 {
                        if !continues_control_statement_after_block(tokens, index, end) {
                            push_body_statement(tokens, &mut result, stmt_start, index);
                            stmt_start = index + 1;
                        }
                    }
                }
            }
            Some(FrontTokenKind::Symbol("(")) => {
                paren_depth += 1;
            }
            Some(FrontTokenKind::Symbol(")")) => {
                if paren_depth > 0 {
                    paren_depth -= 1;
                }
            }
            Some(FrontTokenKind::Symbol("[")) => {
                bracket_depth += 1;
            }
            Some(FrontTokenKind::Symbol("]")) => {
                if bracket_depth > 0 {
                    bracket_depth -= 1;
                }
            }
            Some(FrontTokenKind::Symbol(";")) if brace_depth == 0 && paren_depth == 0 && bracket_depth == 0 => {
                push_body_statement(tokens, &mut result, stmt_start, index);
                stmt_start = index + 1;
            }
            Some(FrontTokenKind::Keyword(_)) if brace_depth == 0 && paren_depth == 0 && bracket_depth == 0 && index > stmt_start => {
                if starts_new_body_statement(tokens, index) {
                    push_body_statement(tokens, &mut result, stmt_start, index);
                    stmt_start = index;
                }
            }
            _ => {}
        }
        if brace_depth == 0
            && paren_depth == 0
            && bracket_depth == 0
            && should_split_statement_on_newline(tokens, stmt_start, index, end)
        {
            push_body_statement(tokens, &mut result, stmt_start, index);
            stmt_start = index + 1;
        }
        if brace_depth == 0
            && paren_depth == 0
            && bracket_depth == 0
            && should_split_statement_inline(tokens, stmt_start, index, end)
        {
            push_body_statement(tokens, &mut result, stmt_start, index);
            stmt_start = index + 1;
        }
        index += 1;
    }

    if stmt_start < end {
        push_body_statement(tokens, &mut result, stmt_start, end.saturating_sub(1));
    }

    result
}

fn continues_control_statement_after_block(tokens: &[FrontToken], block_end: usize, body_end: usize) -> bool {
    let next = block_end + 1;
    if next >= body_end || next >= tokens.len() {
        return false;
    }
    matches!(
        tokens.get(next).map(|token| &token.kind),
        Some(FrontTokenKind::Keyword("否则")) | Some(FrontTokenKind::Keyword("得"))
    )
}

fn starts_new_body_statement(tokens: &[FrontToken], index: usize) -> bool {
    matches!(
        tokens.get(index).map(|token| &token.kind),
        Some(FrontTokenKind::Keyword("定义"))
            | Some(FrontTokenKind::Keyword("返回"))
            | Some(FrontTokenKind::Keyword("如果"))
            | Some(FrontTokenKind::Keyword("当"))
            | Some(FrontTokenKind::Keyword("中断"))
            | Some(FrontTokenKind::Keyword("继续"))
            | Some(FrontTokenKind::Keyword("尝试"))
            | Some(FrontTokenKind::Keyword("抛出"))
    )
}

fn should_split_statement_on_newline(
    tokens: &[FrontToken],
    stmt_start: usize,
    current: usize,
    body_end: usize,
) -> bool {
    let next = current + 1;
    if current < stmt_start || next >= body_end || next >= tokens.len() {
        return false;
    }
    if tokens[current].line == tokens[next].line {
        return false;
    }
    if is_line_continuation_after(&tokens[current].kind) {
        return false;
    }
    if is_line_continuation_before(&tokens[next].kind) {
        return false;
    }
    if matches!(tokens.get(stmt_start).map(|token| &token.kind), Some(FrontTokenKind::Keyword("如果")) | Some(FrontTokenKind::Keyword("当")) | Some(FrontTokenKind::Keyword("尝试")))
        && matches!(tokens.get(next).map(|token| &token.kind), Some(FrontTokenKind::Symbol("{")) | Some(FrontTokenKind::Keyword("否则")) | Some(FrontTokenKind::Keyword("得")))
    {
        return false;
    }
    true
}

fn should_split_statement_inline(
    tokens: &[FrontToken],
    stmt_start: usize,
    current: usize,
    body_end: usize,
) -> bool {
    let next = current + 1;
    if current < stmt_start || next >= body_end || next >= tokens.len() {
        return false;
    }
    if tokens[current].line != tokens[next].line {
        return false;
    }
    if !is_inline_statement_end(&tokens[current].kind) {
        return false;
    }
    if !is_inline_statement_start(&tokens[next].kind) {
        return false;
    }
    true
}

fn is_inline_statement_end(kind: &FrontTokenKind) -> bool {
    matches!(
        kind,
        FrontTokenKind::Identifier(_)
            | FrontTokenKind::Number(_)
            | FrontTokenKind::String(_)
            | FrontTokenKind::Keyword("真")
            | FrontTokenKind::Keyword("假")
            | FrontTokenKind::Keyword("空")
            | FrontTokenKind::Symbol(")")
            | FrontTokenKind::Symbol("]")
            | FrontTokenKind::Symbol("}")
    )
}

fn is_inline_statement_start(kind: &FrontTokenKind) -> bool {
    matches!(
        kind,
        FrontTokenKind::Identifier(_)
            | FrontTokenKind::Number(_)
            | FrontTokenKind::String(_)
            | FrontTokenKind::Keyword("真")
            | FrontTokenKind::Keyword("假")
            | FrontTokenKind::Keyword("空")
            | FrontTokenKind::Keyword("定义")
            | FrontTokenKind::Keyword("返回")
            | FrontTokenKind::Keyword("如果")
            | FrontTokenKind::Keyword("当")
            | FrontTokenKind::Keyword("中断")
            | FrontTokenKind::Keyword("继续")
            | FrontTokenKind::Keyword("尝试")
            | FrontTokenKind::Keyword("抛出")
    )
}

fn is_line_continuation_after(kind: &FrontTokenKind) -> bool {
    matches!(
        kind,
        FrontTokenKind::Symbol("+")
            | FrontTokenKind::Symbol("-")
            | FrontTokenKind::Symbol("*")
            | FrontTokenKind::Symbol("/")
            | FrontTokenKind::Symbol("%")
            | FrontTokenKind::Symbol("=")
            | FrontTokenKind::Symbol("+=")
            | FrontTokenKind::Symbol("-=")
            | FrontTokenKind::Symbol("==")
            | FrontTokenKind::Symbol("!=")
            | FrontTokenKind::Symbol("<")
            | FrontTokenKind::Symbol("<=")
            | FrontTokenKind::Symbol(">")
            | FrontTokenKind::Symbol(">=")
            | FrontTokenKind::Symbol("&&")
            | FrontTokenKind::Symbol("||")
            | FrontTokenKind::Symbol("<<")
            | FrontTokenKind::Symbol(">>")
            | FrontTokenKind::Symbol("&")
            | FrontTokenKind::Symbol("|")
            | FrontTokenKind::Symbol("^")
            | FrontTokenKind::Symbol(",")
            | FrontTokenKind::Symbol(":")
            | FrontTokenKind::Symbol(".")
            | FrontTokenKind::Symbol("(")
            | FrontTokenKind::Symbol("[")
            | FrontTokenKind::Symbol("{")
            | FrontTokenKind::Keyword("且")
            | FrontTokenKind::Keyword("或")
    )
}

fn is_line_continuation_before(kind: &FrontTokenKind) -> bool {
    matches!(
        kind,
        FrontTokenKind::Symbol("+")
            | FrontTokenKind::Symbol("-")
            | FrontTokenKind::Symbol("*")
            | FrontTokenKind::Symbol("/")
            | FrontTokenKind::Symbol("%")
            | FrontTokenKind::Symbol("=")
            | FrontTokenKind::Symbol("+=")
            | FrontTokenKind::Symbol("-=")
            | FrontTokenKind::Symbol("==")
            | FrontTokenKind::Symbol("!=")
            | FrontTokenKind::Symbol("<")
            | FrontTokenKind::Symbol("<=")
            | FrontTokenKind::Symbol(">")
            | FrontTokenKind::Symbol(">=")
            | FrontTokenKind::Symbol("&&")
            | FrontTokenKind::Symbol("||")
            | FrontTokenKind::Symbol("<<")
            | FrontTokenKind::Symbol(">>")
            | FrontTokenKind::Symbol("&")
            | FrontTokenKind::Symbol("|")
            | FrontTokenKind::Symbol("^")
            | FrontTokenKind::Symbol(",")
            | FrontTokenKind::Symbol(")")
            | FrontTokenKind::Symbol("]")
            | FrontTokenKind::Symbol(".")
            | FrontTokenKind::Symbol("[")
            | FrontTokenKind::Symbol("(")
            | FrontTokenKind::Symbol("{")
            | FrontTokenKind::Keyword("且")
            | FrontTokenKind::Keyword("或")
    )
}

fn push_body_statement(
    tokens: &[FrontToken],
    result: &mut Vec<BodyStmtSummary>,
    start: usize,
    end: usize,
) {
    if start > end || start >= tokens.len() {
        return;
    }
    let line_start = tokens.get(start).map(|token| token.line).unwrap_or(0);
    let token_end = end.min(tokens.len().saturating_sub(1));
    let line_end = tokens.get(token_end).map(|token| token.line).unwrap_or(line_start);
    let kind = classify_body_statement(tokens, start, end);
    let (target_expr, primary_expr) = extract_statement_exprs(tokens, kind, start, end);
    let ast = build_body_stmt_ast(tokens, start, end, kind, target_expr.clone(), primary_expr.clone());
    result.push(BodyStmtSummary {
        kind,
        token_start: start,
        token_end,
        line_start,
        line_end,
        ast,
        target_expr,
        primary_expr,
    });
}

fn extract_if_branch_statements(
    tokens: &[FrontToken],
    start: usize,
    end: usize,
) -> (Vec<BodyStmtSummary>, Vec<BodyStmtSummary>) {
    let mut then_body = Vec::new();
    let mut else_body = Vec::new();
    let cond_start = start + 1;
    let Some(cond_end) = find_matching_symbol(tokens, cond_start, "(", ")") else {
        return (then_body, else_body);
    };
    let then_start = cond_end + 1;
    if !matches!(tokens.get(then_start).map(|token| &token.kind), Some(FrontTokenKind::Symbol("{"))) {
        return (then_body, else_body);
    }
    let Some(then_end) = find_matching_symbol(tokens, then_start, "{", "}") else {
        return (then_body, else_body);
    };
    then_body = extract_body_statements(tokens, (then_start, then_end));

    let else_keyword = then_end + 1;
    if else_keyword > end || !matches!(tokens.get(else_keyword).map(|token| &token.kind), Some(FrontTokenKind::Keyword("否则"))) {
        return (then_body, else_body);
    }

    let else_start = else_keyword + 1;
    if else_start > end {
        return (then_body, else_body);
    }

    match tokens.get(else_start).map(|token| &token.kind) {
        Some(FrontTokenKind::Keyword("如果")) => {
            push_body_statement(tokens, &mut else_body, else_start, end);
        }
        Some(FrontTokenKind::Symbol("{")) => {
            if let Some(else_end) = find_matching_symbol(tokens, else_start, "{", "}") {
                else_body = extract_body_statements(tokens, (else_start, else_end));
            }
        }
        _ => {}
    }

    (then_body, else_body)
}

fn extract_while_body_statements(
    tokens: &[FrontToken],
    start: usize,
    _end: usize,
) -> Vec<BodyStmtSummary> {
    let cond_start = start + 1;
    let Some(cond_end) = find_matching_symbol(tokens, cond_start, "(", ")") else {
        return Vec::new();
    };
    let body_start = cond_end + 1;
    if !matches!(tokens.get(body_start).map(|token| &token.kind), Some(FrontTokenKind::Symbol("{"))) {
        return Vec::new();
    }
    let Some(body_end) = find_matching_symbol(tokens, body_start, "{", "}") else {
        return Vec::new();
    };
    extract_body_statements(tokens, (body_start, body_end))
}

fn classify_body_statement(tokens: &[FrontToken], start: usize, end: usize) -> &'static str {
    match tokens.get(start).map(|token| &token.kind) {
        Some(FrontTokenKind::Keyword("定义")) => "定义",
        Some(FrontTokenKind::Keyword("返回")) => "返回",
        Some(FrontTokenKind::Keyword("如果")) => "如果",
        Some(FrontTokenKind::Keyword("当")) => "当",
        Some(FrontTokenKind::Keyword("中断")) => "中断",
        Some(FrontTokenKind::Keyword("继续")) => "继续",
        Some(FrontTokenKind::Keyword("尝试")) => "尝试",
        Some(FrontTokenKind::Keyword("抛出")) => "抛出",
        _ => match find_top_level_assignment_operator(tokens, start, end) {
            Some((_, "+=")) => "加等赋值",
            Some((_, "-=")) => "减等赋值",
            Some((_, "=")) => "赋值",
            _ => "表达式",
        },
    }
}

fn extract_statement_exprs(
    tokens: &[FrontToken],
    stmt_kind: &str,
    start: usize,
    end: usize,
) -> (Option<ParsedExpr>, Option<ParsedExpr>) {
    if start >= tokens.len() || start > end {
        return (None, None);
    }

    match stmt_kind {
        "定义" => {
            let mut probe = start + 1;
            if matches_keyword(tokens, probe, "可变") {
                probe += 1;
            }
            let target_expr = extract_name(tokens, probe)
                .map(|name| ParsedExpr::new(ExprTree::Identifier(name), probe, probe));
            let primary_expr = find_top_level_assignment_operator(tokens, probe, end)
                .and_then(|(op_index, _)| parse_expression_range(tokens, op_index.saturating_add(1), end));
            (target_expr, primary_expr)
        }
        "返回" | "抛出" => (None, parse_expression_range(tokens, start + 1, end)),
        "如果" | "当" => {
            if !matches!(tokens.get(start + 1).map(|token| &token.kind), Some(FrontTokenKind::Symbol("("))) {
                return (None, None);
            }
            let Some(cond_end) = find_matching_symbol(tokens, start + 1, "(", ")") else {
                return (None, None);
            };
            if cond_end <= start + 1 {
                return (None, None);
            }
            (None, parse_expression_range(tokens, start + 2, cond_end.saturating_sub(1)))
        }
        "赋值" => {
            let Some((op_index, _)) = find_top_level_assignment_operator(tokens, start, end) else {
                return (None, None);
            };
            let target_expr = if op_index > start {
                parse_expression_range(tokens, start, op_index.saturating_sub(1))
            } else {
                None
            };
            let primary_expr = parse_expression_range(tokens, op_index.saturating_add(1), end);
            (target_expr, primary_expr)
        }
        "加等赋值" | "减等赋值" => {
            let Some((op_index, _)) = find_top_level_assignment_operator(tokens, start, end) else {
                return (None, None);
            };
            let target_expr = if op_index > start {
                parse_expression_range(tokens, start, op_index.saturating_sub(1))
            } else {
                None
            };
            let primary_expr = match (target_expr.clone(), parse_expression_range(tokens, op_index.saturating_add(1), end)) {
                (Some(target), Some(value)) => {
                    let token_start = target.token_start;
                    let token_end = value.token_end;
                    Some(ParsedExpr::new(
                        ExprTree::Binary {
                            op: if stmt_kind == "加等赋值" { "+" } else { "-" },
                            left: Box::new(target),
                            right: Box::new(value),
                        },
                        token_start,
                        token_end,
                    ))
                }
                _ => None,
            };
            (target_expr, primary_expr)
        }
        "表达式" => (None, parse_expression_range(tokens, start, end)),
        _ => (None, None),
    }
}

fn build_body_stmt_ast(
    tokens: &[FrontToken],
    start: usize,
    end: usize,
    stmt_kind: &'static str,
    target_expr: Option<ParsedExpr>,
    primary_expr: Option<ParsedExpr>,
) -> BodyStmtAst {
    match stmt_kind {
        "定义" => {
            let mut probe = start + 1;
            let mutable = matches_keyword(tokens, probe, "可变");
            if mutable {
                probe += 1;
            }
            let name = extract_name(tokens, probe).or_else(|| identifier_name_from_expr(target_expr.as_ref()));
            BodyStmtAst::Define {
                mutable,
                name,
                value: primary_expr,
            }
        }
        "返回" => BodyStmtAst::Return { value: primary_expr },
        "如果" => {
            let (then_body, else_body) = extract_if_branch_statements(tokens, start, end);
            BodyStmtAst::If {
                condition: primary_expr,
                then_body,
                else_body,
            }
        }
        "当" => BodyStmtAst::While {
            condition: primary_expr,
            body: extract_while_body_statements(tokens, start, end),
        },
        "中断" => BodyStmtAst::Break,
        "继续" => BodyStmtAst::Continue,
        "尝试" => BodyStmtAst::Try,
        "抛出" => BodyStmtAst::Throw { value: primary_expr },
        "赋值" => BodyStmtAst::Assign {
            op: "=",
            target: target_expr,
            value: primary_expr,
        },
        "加等赋值" => BodyStmtAst::Assign {
            op: "+=",
            target: target_expr,
            value: primary_expr,
        },
        "减等赋值" => BodyStmtAst::Assign {
            op: "-=",
            target: target_expr,
            value: primary_expr,
        },
        _ => BodyStmtAst::Expr {
            value: primary_expr,
        },
    }
}

fn identifier_name_from_expr(expr: Option<&ParsedExpr>) -> Option<String> {
    match expr.map(|expr| &expr.tree) {
        Some(ExprTree::Identifier(name)) => Some(name.clone()),
        _ => None,
    }
}

fn parse_expression_range(
    tokens: &[FrontToken],
    start: usize,
    end: usize,
) -> Option<ParsedExpr> {
    if start >= tokens.len() || start > end {
        return None;
    }
    let mut parser = ExprParser::new(tokens, start, end);
    let expr = parser.parse_expression(1)?;
    if parser.is_done() {
        Some(expr)
    } else {
        Some(ParsedExpr::new(
            ExprTree::Unknown,
            start,
            end.min(tokens.len().saturating_sub(1)),
        ))
    }
}

fn find_top_level_assignment_operator(
    tokens: &[FrontToken],
    start: usize,
    end: usize,
) -> Option<(usize, &'static str)> {
    let mut brace_depth = 0i32;
    let mut paren_depth = 0i32;
    let mut bracket_depth = 0i32;
    let mut index = start;
    while index <= end && index < tokens.len() {
        match tokens.get(index).map(|token| &token.kind) {
            Some(FrontTokenKind::Symbol("{")) => brace_depth += 1,
            Some(FrontTokenKind::Symbol("}")) => brace_depth -= 1,
            Some(FrontTokenKind::Symbol("(")) => paren_depth += 1,
            Some(FrontTokenKind::Symbol(")")) => paren_depth -= 1,
            Some(FrontTokenKind::Symbol("[")) => bracket_depth += 1,
            Some(FrontTokenKind::Symbol("]")) => bracket_depth -= 1,
            Some(FrontTokenKind::Symbol(symbol))
                if brace_depth == 0 && paren_depth == 0 && bracket_depth == 0 && matches!(*symbol, "=" | "+=" | "-=") =>
            {
                return Some((index, *symbol));
            }
            _ => {}
        }
        index += 1;
    }
    None
}

fn extract_import_path(tokens: &[FrontToken], index: usize) -> Option<String> {
    match (
        tokens.get(index).map(|token| &token.kind),
        tokens.get(index + 1).map(|token| &token.kind),
        tokens.get(index + 2).map(|token| &token.kind),
    ) {
        (
            Some(FrontTokenKind::Symbol("(")),
            Some(FrontTokenKind::String(path)),
            Some(FrontTokenKind::Symbol(")")),
        ) => Some(path.clone()),
        _ => None,
    }
}

fn is_number_start(ch: char) -> bool {
    ch.is_ascii_digit()
}

fn is_identifier_start(ch: char) -> bool {
    ch == '_' || ch.is_alphabetic()
}

fn is_identifier_continue(ch: char) -> bool {
    ch == '_' || ch.is_alphanumeric()
}

/// 返回Rust辅助前端的关键字映射表: (别名, 正规形式)
pub fn helper_frontend_keywords() -> Vec<(&'static str, &'static str)> {
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

// WARNING: 等于/小于/大于 在标准库中被用作变量名(如 定义 可变 小于 = [])
// 此映射仅因 Rust 种子编译器只编译 编译器源码(不编译标准库) 而暂时安全。
// Nova 编译器已改用 _p_中文算术类型() 上下文感知处理(仅在二元运算位置识别)。
// 如果 Rust 需编译含这些变量名的代码，必须移除此映射。
fn normalize_operator_word(text: &str) -> Option<&'static str> {
    match text {
        "等于" => Some("=="),
        "不等于" => Some("!="),
        "大于" => Some(">"),
        "小于" => Some("<"),
        "大于等于" => Some(">="),
        "小于等于" => Some("<="),
        _ => None,
    }
}

fn normalize_keyword(text: &str) -> Option<&'static str> {
    match text {
        // Only add aliases that do NOT conflict with identifiers in the kernel
        "函数" | "做法" | "法" => Some("函数"),
        "定义" | "称为" | "叫做" => Some("定义"),
        "可变" => Some("可变"),
        "返回" | "给出" => Some("返回"),
        "如果" | "假如" | "倘若" | "要是" | "若是" | "每当" => Some("如果"),
        "否则" | "不然的话" | "否则的话" | "除此之外" => Some("否则"),
        "当" | "循环" => Some("当"),
        "导入" | "入" => Some("导入"),
        "真" => Some("真"),
        "假" => Some("假"),
        "空" => Some("空"),
        "结构" => Some("结构"),
        "建" => Some("建"),
        "中断" | "止" => Some("中断"),
        "继续" | "续" => Some("继续"),
        "匹配" => Some("匹配"),
        "默认" => Some("默认"),
        "尝试" | "试" => Some("尝试"),
        "得" => Some("得"),
        "抛出" | "抛" => Some("抛出"),
        "每" | "每个" => Some("每"),
        "且" | "并且" | "而且" => Some("且"),
        "或" | "或者" => Some("或"),
        "非" => Some("非"),
        "凡是" => Some("如果"),
        _ => None,
    }
}

fn scan_symbol(chars: &[char], index: usize) -> Option<(&'static str, usize)> {
    let current = peek_char(chars, index)?;
    let next = peek_char(chars, index + 1);
    match (current, next) {
        ('=', Some('=')) => Some(("==", 2)),
        ('!', Some('=')) => Some(("!=", 2)),
        ('<', Some('=')) => Some(("<=", 2)),
        ('>', Some('=')) => Some((">=", 2)),
        ('&', Some('&')) => Some(("&&", 2)),
        ('|', Some('|')) => Some(("||", 2)),
        ('<', Some('<')) => Some(("<<", 2)),
        ('>', Some('>')) => Some((">>", 2)),
        ('+', Some('=')) => Some(("+=", 2)),
        ('-', Some('=')) => Some(("-=", 2)),
        ('+', _) => Some(("+", 1)),
        ('-', _) => Some(("-", 1)),
        ('*', _) => Some(("*", 1)),
        ('/', _) => Some(("/", 1)),
        ('%', _) => Some(("%", 1)),
        ('=', _) => Some(("=", 1)),
        ('!', _) => Some(("!", 1)),
        ('<', _) => Some(("<", 1)),
        ('>', _) => Some((">", 1)),
        ('&', _) => Some(("&", 1)),
        ('|', _) => Some(("|", 1)),
        ('^', _) => Some(("^", 1)),
        ('~', _) => Some(("~", 1)),
        ('(', _) => Some(("(", 1)),
        (')', _) => Some((")", 1)),
        ('{', _) => Some(("{", 1)),
        ('}', _) => Some(("}", 1)),
        ('[', _) => Some(("[", 1)),
        (']', _) => Some(("]", 1)),
        ('.', _) => Some((".", 1)),
        (',', _) => Some((",", 1)),
        (':', _) => Some((":", 1)),
        (';', _) => Some((";", 1)),
        _ => None,
    }
}

pub fn current_kernel_requirements(project: &KernelProject) -> Vec<CapabilityRequirement> {
    let known = [
        CapabilityRequirement {
            module: "标准库/基础.nova",
            reasons: &[
                "紧凑单行块与内联 if/while 体",
                "高阶函数参数与函数值调用",
                "整数/位运算/负数字面量混排",
            ],
        },
        CapabilityRequirement {
            module: "标准库/文件/路径/路径.nova",
            reasons: &[
                "高密度单行控制流",
                "反斜杠字符串字面量",
                "路径短名别名与递归路径处理",
            ],
        },
        CapabilityRequirement {
            module: "编译器/模块系统/增量缓存.nova",
            reasons: &[
                "brace 风格结构体构造",
                "单行 continue/续 控制流",
                "缓存条目字段与字典/列表混合访问",
            ],
        },
        CapabilityRequirement {
            module: "编译器/模块系统/模块图编译器.nova",
            reasons: &[
                "大函数体上的连续控制流与共享编译状态",
                "结构体/字典/列表/字段访问交织",
                "模块图扫描与链接阶段的一行式分支",
            ],
        },
        CapabilityRequirement {
            module: "编译器/模块系统/模块接口契约.nova",
            reasons: &[
                "brace 风格结构体构造与字段初始化",
                "契约注册表/索引字典联动",
            ],
        },
        CapabilityRequirement {
            module: "编译器/语义/行号追踪.nova",
            reasons: &[
                "极简一行函数体",
                "结构体字段定义与注释混排",
            ],
        },
    ];

    known
        .into_iter()
        .filter(|item| project.local_sources.iter().any(|path| path == Path::new(item.module)))
        .collect()
}

pub fn discover_workspace_root(cargo_manifest_dir: &Path) -> Result<PathBuf, String> {
    let parent = cargo_manifest_dir
        .parent()
        .ok_or_else(|| format!("无法定位种子目录: {}", cargo_manifest_dir.display()))?;
    let workspace_root = parent
        .parent()
        .ok_or_else(|| format!("无法定位分离链式自举根目录: {}", cargo_manifest_dir.display()))?;
    Ok(workspace_root.to_path_buf())
}

pub fn load_kernel_project(
    workspace_root: PathBuf,
    kernel_root_override: Option<PathBuf>,
) -> Result<KernelProject, String> {
    let kernel_root = match kernel_root_override {
        Some(path) => path,
        None => detect_default_kernel_root(&workspace_root)?,
    };

    if !kernel_root.is_dir() {
        return Err(format!("内核目录不存在: {}", kernel_root.display()));
    }

    let kernel_root = canonicalize_existing_path(kernel_root, "内核目录")?;

    let manifest_source = canonicalize_existing_path(pick_manifest(&kernel_root)?, "内核清单")?;
    let content = fs::read_to_string(&manifest_source)
        .map_err(|e| format!("读取清单失败: {} ({})", manifest_source.display(), e))?;

    let mut local_sources = Vec::new();
    let mut localized_manifest_lines = Vec::new();
    let mut runtime_seed_source = None;
    let mut canonical_source_map: BTreeMap<PathBuf, String> = BTreeMap::new();

    for raw in content.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }

        if line == EXTERNAL_RUNTIME_SEED_MANIFEST_REL {
            let source = kernel_root.join(line);
            if !source.is_file() {
                return Err(format!(
                    "外部运行时种子不存在: {} -> {}",
                    line,
                    source.display()
                ));
            }
            let source = canonicalize_existing_path(source, "外部运行时种子")?;
            if let Some(previous) = canonical_source_map.insert(source.clone(), line.to_string()) {
                return Err(format!(
                    "清单重复映射到同一真实文件: {} 与 {}",
                    previous,
                    line
                ));
            }
            runtime_seed_source = Some(source);
            localized_manifest_lines.push(LOCAL_RUNTIME_SEED_REL.to_string());
            continue;
        }

        let relative = PathBuf::from(line);
        let absolute = kernel_root.join(&relative);
        if !absolute.is_file() {
            return Err(format!(
                "清单文件不存在: {} -> {}",
                line,
                absolute.display()
            ));
        }
        let absolute = canonicalize_existing_path(absolute, "清单文件")?;
        if let Some(previous) = canonical_source_map.insert(absolute.clone(), line.to_string()) {
            return Err(format!(
                "清单重复映射到同一真实文件: {} 与 {}",
                previous,
                line
            ));
        }

        if relative == Path::new(LOCAL_RUNTIME_SEED_REL) {
            runtime_seed_source = Some(absolute.clone());
        }

        local_sources.push(relative);
        localized_manifest_lines.push(line.to_string());
    }

    let runtime_seed_source = canonicalize_existing_path(runtime_seed_source.ok_or_else(|| {
        format!(
            "当前内核清单未声明运行时种子: {}",
            manifest_source.display()
        )
    })?, "运行时种子")?;

    let entry_module_rel = local_sources
        .iter()
        .find(|path| path.as_path() == Path::new(NOVA_ENTRY_REL))
        .cloned()
        .ok_or_else(|| format!("清单缺少入口文件: {}", NOVA_ENTRY_REL))?;

    if !local_sources
        .iter()
        .any(|path| path == Path::new(FRONTEND_ENTRY_REL))
    {
        return Err(format!("清单缺少前端入口文件: {}", FRONTEND_ENTRY_REL));
    }

    Ok(KernelProject {
        workspace_root,
        kernel_root,
        manifest_source,
        local_sources,
        entry_module_rel,
        localized_manifest_lines,
        runtime_seed_source,
        // 6.1: AI基因系统项目合同默认值
        profile: String::from("linux-user"),
        target: String::from("x86-64-linux"),
        entry_symbol: String::from("none"),
        image_format: String::from("elf64"),
        runtime_mode: String::from("interpreter"),
        debug_contract: String::from("none"),
        emit: String::new(),
        link_layout: String::new(),
        subsystem_tags: Vec::new(),
        module_tags: BTreeMap::new(),
    })
}

fn detect_default_kernel_root(workspace_root: &Path) -> Result<PathBuf, String> {
    let parent = workspace_root
        .parent()
        .ok_or_else(|| format!("无法从工作区推导中文原生目录: {}", workspace_root.display()))?;
    let candidates = [
        parent.join("原生编译器"),
        parent.join("内核"),
        parent.join("中文原生"),
    ];
    let manifest_names = [
        "_manifest.txt",
        "nova_manifest.txt",
        ".__nova_files_tmp.txt",
        "nova_files_tmp.txt",
    ];
    let mut tried = Vec::new();

    for candidate in candidates {
        tried.push(candidate.display().to_string());
        if !candidate.is_dir() {
            continue;
        }
        let has_entry = candidate.join(NOVA_ENTRY_REL).is_file();
        let has_manifest = !collect_existing_manifests(&candidate, &manifest_names).is_empty();
        if has_entry && has_manifest {
            return Ok(candidate);
        }
    }

    Err(format!(
        "未找到原生编译器项目目录: {}",
        tried.join(" | ")
    ))
}

fn pick_manifest(kernel_root: &Path) -> Result<PathBuf, String> {
    let authority = collect_existing_manifests(kernel_root, &["_manifest.txt", "nova_manifest.txt"]);
    if !authority.is_empty() {
        ensure_manifest_group_consistency("权威清单", &authority)?;
        return Ok(authority[0].clone());
    }

    let temporary = collect_existing_manifests(
        kernel_root,
        &[".__nova_files_tmp.txt", "nova_files_tmp.txt"],
    );
    if !temporary.is_empty() {
        ensure_manifest_group_consistency("临时清单", &temporary)?;
        return Ok(temporary[0].clone());
    }

    Err(format!(
        "未找到内核清单: {}",
        kernel_root.display()
    ))
}

fn collect_existing_manifests(kernel_root: &Path, names: &[&str]) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for name in names {
        let candidate = kernel_root.join(name);
        if candidate.is_file() {
            paths.push(candidate);
        }
    }
    paths
}

fn ensure_manifest_group_consistency(label: &str, manifests: &[PathBuf]) -> Result<(), String> {
    if manifests.len() < 2 {
        return Ok(());
    }
    let baseline = normalized_manifest_lines(&manifests[0])?;
    for candidate in manifests.iter().skip(1) {
        let current = normalized_manifest_lines(candidate)?;
        if current != baseline {
            return Err(format!(
                "{}存在不一致文件: {} 与 {}",
                label,
                manifests[0].display(),
                candidate.display()
            ));
        }
    }
    Ok(())
}

fn normalized_manifest_lines(path: &Path) -> Result<Vec<String>, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("读取清单失败: {} ({})", path.display(), e))?;
    Ok(content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.to_string())
        .collect())
}

fn canonicalize_existing_path(path: PathBuf, label: &str) -> Result<PathBuf, String> {
    fs::canonicalize(&path)
        .map_err(|e| format!("{}规范化失败: {} ({})", label, path.display(), e))
}

pub fn sync_runtime_seed_to_authority(
    workspace_root: PathBuf,
    kernel_root_override: Option<PathBuf>,
) -> Result<RuntimeSeedSyncReport, String> {
    let project = load_kernel_project(workspace_root, kernel_root_override)?;
    let runtime_seed_target = project.kernel_root.join(LOCAL_RUNTIME_SEED_REL);
    if let Some(parent) = runtime_seed_target.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "创建权威运行时种子目录失败: {} ({})",
                parent.display(),
                e
            )
        })?;
    }

    let runtime_seed_bytes = fs::read(&project.runtime_seed_source).map_err(|e| {
        format!(
            "读取运行时种子源码失败: {} ({})",
            project.runtime_seed_source.display(),
            e
        )
    })?;
    fs::write(&runtime_seed_target, &runtime_seed_bytes).map_err(|e| {
        format!(
            "写入权威运行时种子失败: {} ({})",
            runtime_seed_target.display(),
            e
        )
    })?;

    let mut manifest_paths = Vec::new();
    for name in ["_manifest.txt", "nova_manifest.txt"] {
        let path = project.kernel_root.join(name);
        if path.is_file() {
            localize_authority_manifest_runtime_seed(&path)?;
            manifest_paths.push(path);
        }
    }

    let frontend_entry_path = project.kernel_root.join(FRONTEND_ENTRY_REL);
    localize_authority_frontend_entry_import(&frontend_entry_path)?;

    Ok(RuntimeSeedSyncReport {
        kernel_root: project.kernel_root,
        runtime_seed_source: project.runtime_seed_source,
        runtime_seed_target,
        frontend_entry_path,
        manifest_paths,
    })
}

fn localize_authority_manifest_runtime_seed(path: &Path) -> Result<(), String> {
    let source = fs::read_to_string(path)
        .map_err(|e| format!("读取权威清单失败: {} ({})", path.display(), e))?;
    let (updated, hits) = rewrite_unique_trimmed_line(&source, LOCAL_RUNTIME_SEED_REL, |line| {
        line == EXTERNAL_RUNTIME_SEED_MANIFEST_REL || line == LOCAL_RUNTIME_SEED_REL
    });

    if hits != 1 || !updated.lines().any(|line| line.trim() == LOCAL_RUNTIME_SEED_REL) {
        return Err(format!(
            "权威清单未能本地化运行时种子契约: {}",
            path.display()
        ));
    }

    fs::write(path, updated)
        .map_err(|e| format!("写回权威清单失败: {} ({})", path.display(), e))?;
    Ok(())
}

fn localize_authority_frontend_entry_import(path: &Path) -> Result<(), String> {
    let source = fs::read_to_string(path)
        .map_err(|e| format!("读取权威前端入口失败: {} ({})", path.display(), e))?;
    let local_import = format!("导入(\"{}\")", LOCAL_RUNTIME_SEED_IMPORT_REL);
    let external_import = format!("导入(\"{}\")", EXTERNAL_RUNTIME_SEED_IMPORT_REL);
    let (updated, hits) = rewrite_unique_trimmed_line(&source, &local_import, |line| {
        line == external_import.as_str() || line == local_import.as_str()
    });

    if hits > 1 {
        return Err(format!(
            "权威前端入口存在多处运行时种子导入 ({}处), 无法安全本地化: {}",
            hits,
            path.display()
        ));
    }
    if hits == 0 {
        return Ok(());
    }

    fs::write(path, updated)
        .map_err(|e| format!("写回权威前端入口失败: {} ({})", path.display(), e))?;
    Ok(())
}

fn rewrite_unique_trimmed_line<F>(source: &str, replacement_trimmed: &str, mut predicate: F) -> (String, usize)
where
    F: FnMut(&str) -> bool,
{
    let mut hits = 0usize;
    let had_trailing_newline = source.ends_with('\n');
    let mut lines = Vec::new();

    for line in source.lines() {
        if predicate(line.trim()) {
            lines.push(replacement_trimmed.to_string());
            hits += 1;
        } else {
            lines.push(line.to_string());
        }
    }

    let mut updated = lines.join("\n");
    if had_trailing_newline {
        updated.push('\n');
    }
    (updated, hits)
}
