// 语义事实: 统一骨架解析结果

#[derive(Clone, Debug)]
pub struct SkeletonResult {
    pub public_apis: Vec<String>,
    pub types: Vec<String>,
    pub functions: Vec<String>,
    pub imports: Vec<String>,
    pub high_risk: Vec<String>,
}

impl SkeletonResult {
    pub fn empty() -> Self {
        Self {
            public_apis: Vec::new(),
            types: Vec::new(),
            functions: Vec::new(),
            imports: Vec::new(),
            high_risk: Vec::new(),
        }
    }

    #[allow(dead_code)]
    pub fn summary(&self) -> String {
        format!(
            "APIs:{} Types:{} Functions:{} Imports:{} HighRisk:{}",
            self.public_apis.len(),
            self.types.len(),
            self.functions.len(),
            self.imports.len(),
            self.high_risk.len()
        )
    }
}
