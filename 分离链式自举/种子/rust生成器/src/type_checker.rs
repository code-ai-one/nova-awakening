#![allow(dead_code, unused_imports, unused_variables, unused_mut)]
/// Nova 类型检查验证器
/// 验证编译中间状态的类型正确性
/// 应用：调试编译器 / 确保类型安全 / SSA 类型一致性检查

use std::collections::HashMap;

/// Nova 类型（与 Nova 运行时类型系统对齐）
#[derive(Debug, Clone, PartialEq)]
pub enum NovaType {
    Int,         // 整数（64位）
    Float,       // 浮点（64位）
    Bool,
    String,
    Nil,
    List(Box<NovaType>),          // 同类型列表
    Dict(Box<NovaType>, Box<NovaType>),  // 字典
    Function(Vec<NovaType>, Box<NovaType>),  // (参数类型, 返回类型)
    Any,         // 动态类型（未知）
    Never,       // 不可达类型（return/panic后）
    Tuple(Vec<NovaType>),
}

impl NovaType {
    pub fn is_numeric(&self) -> bool { matches!(self, NovaType::Int | NovaType::Float) }
    pub fn is_comparable(&self) -> bool { matches!(self, NovaType::Int | NovaType::Float | NovaType::String | NovaType::Bool) }
    pub fn is_iterable(&self) -> bool { matches!(self, NovaType::List(_) | NovaType::String) }
    pub fn is_callable(&self) -> bool { matches!(self, NovaType::Function(_, _)) }
    pub fn is_compatible_with(&self, other: &NovaType) -> bool {
        if self == other { return true; }
        if matches!(self, NovaType::Any) || matches!(other, NovaType::Any) { return true; }
        if matches!(self, NovaType::Never) || matches!(other, NovaType::Never) { return true; }
        false
    }

    pub fn name(&self) -> String {
        match self {
            NovaType::Int => "整数".into(),
            NovaType::Float => "浮点".into(),
            NovaType::Bool => "布尔".into(),
            NovaType::String => "字符串".into(),
            NovaType::Nil => "空".into(),
            NovaType::List(t) => format!("列表[{}]", t.name()),
            NovaType::Dict(k, v) => format!("字典[{} → {}]", k.name(), v.name()),
            NovaType::Function(params, ret) => {
                let p = params.iter().map(|t| t.name()).collect::<Vec<_>>().join(", ");
                format!("函数({}) → {}", p, ret.name())
            }
            NovaType::Any => "动态".into(),
            NovaType::Never => "永不".into(),
            NovaType::Tuple(ts) => format!("({},)", ts.iter().map(|t| t.name()).collect::<Vec<_>>().join(", ")),
        }
    }
}

/// 类型错误
#[derive(Debug, Clone)]
pub struct TypeError {
    pub location: String,
    pub expected: NovaType,
    pub got:      NovaType,
    pub context:  String,
}

impl TypeError {
    pub fn new(loc: impl Into<String>, expected: NovaType, got: NovaType, ctx: String) -> Self {
        TypeError { location: loc.into(), expected, got, context: ctx }
    }
    pub fn format(&self) -> String {
        format!("[类型错误 @ {}] {}: 期望 `{}` 但得到 `{}`",
            self.location, self.context, self.expected.name(), self.got.name())
    }
}

/// 类型环境（变量名 → 类型的映射）
#[derive(Debug, Clone, Default)]
pub struct TypeEnv {
    scopes: Vec<HashMap<String, NovaType>>,
}

impl TypeEnv {
    pub fn new() -> Self { TypeEnv { scopes: vec![HashMap::new()] } }
    pub fn push_scope(&mut self) { self.scopes.push(HashMap::new()); }
    pub fn pop_scope(&mut self) { if self.scopes.len() > 1 { self.scopes.pop(); } }

    pub fn define(&mut self, name: impl Into<String>, ty: NovaType) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.into(), ty);
        }
    }

    pub fn lookup(&self, name: &str) -> Option<&NovaType> {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.get(name) { return Some(ty); }
        }
        None
    }

    pub fn assign(&mut self, name: &str, ty: NovaType) -> bool {
        for scope in self.scopes.iter_mut().rev() {
            if scope.contains_key(name) { scope.insert(name.to_string(), ty); return true; }
        }
        false
    }
}

/// 函数类型签名表
#[derive(Default)]
pub struct FuncTypeTable {
    sigs: HashMap<String, (Vec<NovaType>, NovaType)>,
}

impl FuncTypeTable {
    pub fn new() -> Self { FuncTypeTable::default() }
    pub fn register(&mut self, name: impl Into<String>, params: Vec<NovaType>, ret: NovaType) {
        self.sigs.insert(name.into(), (params, ret));
    }
    pub fn lookup(&self, name: &str) -> Option<(&Vec<NovaType>, &NovaType)> {
        self.sigs.get(name).map(|(p, r)| (p, r))
    }
    pub fn register_builtins(&mut self) {
        // 注册内置函数类型
        self.register("计数",  vec![NovaType::Any], NovaType::Int);
        self.register("追加",  vec![NovaType::List(Box::new(NovaType::Any)), NovaType::Any], NovaType::Nil);
        self.register("删除",  vec![NovaType::List(Box::new(NovaType::Any)), NovaType::Int], NovaType::Nil);
        self.register("字符串拼接", vec![NovaType::String, NovaType::String], NovaType::String);
        self.register("转字符串", vec![NovaType::Any], NovaType::String);
        self.register("字典取", vec![NovaType::Dict(Box::new(NovaType::String), Box::new(NovaType::Any)), NovaType::String], NovaType::Any);
        self.register("字典设", vec![NovaType::Dict(Box::new(NovaType::String), Box::new(NovaType::Any)), NovaType::String, NovaType::Any], NovaType::Nil);
        self.register("打印文本", vec![NovaType::String], NovaType::Nil);
    }
}

/// 类型检查器
pub struct TypeChecker {
    pub env:    TypeEnv,
    pub funcs:  FuncTypeTable,
    pub errors: Vec<TypeError>,
    depth:      usize,  // 当前递归深度
}

impl TypeChecker {
    pub fn new() -> Self {
        let mut checker = TypeChecker { env: TypeEnv::new(), funcs: FuncTypeTable::new(), errors: vec![], depth: 0 };
        checker.funcs.register_builtins();
        checker
    }

    pub fn has_errors(&self) -> bool { !self.errors.is_empty() }

    /// 检查二元运算的类型合法性
    pub fn check_binop(&mut self, op: u8, lhs: &NovaType, rhs: &NovaType, loc: &str) -> NovaType {
        match op {
            1 | 2 | 3 | 4 | 5 => {  // + - * / %
                if lhs.is_numeric() && rhs.is_numeric() {
                    if matches!(lhs, NovaType::Float) || matches!(rhs, NovaType::Float) {
                        return NovaType::Float;
                    }
                    return NovaType::Int;
                }
                // 字符串加法（拼接）
                if op == 1 && matches!(lhs, NovaType::String) && matches!(rhs, NovaType::String) {
                    return NovaType::String;
                }
                self.errors.push(TypeError::new(loc, lhs.clone(), rhs.clone(), format!("操作符 {} 的类型不兼容", op)));
                NovaType::Any
            }
            6 | 7 | 8 | 9 | 10 | 11 => {  // == != < <= > >=
                if !lhs.is_compatible_with(rhs) {
                    self.errors.push(TypeError::new(loc, lhs.clone(), rhs.clone(), "比较操作数类型不兼容".to_string()));
                }
                NovaType::Bool
            }
            12 | 13 => NovaType::Bool,  // && ||
            _ => NovaType::Any,
        }
    }

    /// 检查函数调用的参数类型
    pub fn check_call(&mut self, func_name: &str, args: &[NovaType], loc: &str) -> NovaType {
        if let Some((params, ret)) = self.funcs.lookup(func_name) {
            let params = params.clone();
            let ret = ret.clone();
            if args.len() != params.len() {
                self.errors.push(TypeError::new(loc, NovaType::Any, NovaType::Any,
                    format!("函数 `{}` 期望{}个参数但得到{}个", func_name, params.len(), args.len())));
                return ret;
            }
            for (i, (arg, param)) in args.iter().zip(params.iter()).enumerate() {
                if !arg.is_compatible_with(param) {
                    self.errors.push(TypeError::new(loc, param.clone(), arg.clone(),
                        format!("函数 `{}` 第{}个参数类型不匹配", func_name, i + 1)));
                }
            }
            ret
        } else {
            NovaType::Any  // 未知函数：动态类型
        }
    }

    pub fn format_errors(&self) -> String {
        self.errors.iter().map(|e| e.format()).collect::<Vec<_>>().join("\n")
    }

    pub fn stats(&self) -> TypeCheckerStats {
        TypeCheckerStats { errors: self.errors.len(), registered_funcs: self.funcs.sigs.len() }
    }
}

impl Default for TypeChecker { fn default() -> Self { Self::new() } }

#[derive(Debug)]
pub struct TypeCheckerStats {
    pub errors:           usize,
    pub registered_funcs: usize,
}
impl TypeCheckerStats {
    pub fn format(&self) -> String {
        format!("类型检查: {}个错误 {}个已知函数签名",
            self.errors, self.registered_funcs)
    }
}
