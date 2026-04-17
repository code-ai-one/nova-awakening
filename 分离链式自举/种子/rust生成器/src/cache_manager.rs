#![allow(dead_code, unused_imports, unused_variables, unused_mut)]
/// Nova 增量编译缓存管理
/// 基于文件哈希的增量编译：只重新编译变更的模块
/// 缓存格式：.nova_cache/<module_hash>.cache

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub module_path: String,
    pub source_hash: u64,
    pub binary_path: PathBuf,
    pub compiled_at: u64,  // Unix timestamp
    pub file_size:   usize,
}

impl CacheEntry {
    pub fn is_valid(&self, source_hash: u64) -> bool {
        self.source_hash == source_hash
    }
}

/// 增量编译缓存
pub struct BuildCache {
    cache_dir: PathBuf,
    entries:   HashMap<String, CacheEntry>,
    hits:      usize,
    misses:    usize,
}

impl BuildCache {
    pub fn new(cache_dir: impl Into<PathBuf>) -> Self {
        BuildCache {
            cache_dir: cache_dir.into(),
            entries: HashMap::new(),
            hits: 0,
            misses: 0,
        }
    }

    /// 从缓存目录加载已有缓存索引
    pub fn load(&mut self) -> Result<usize, String> {
        let index_path = self.cache_dir.join("index.cache");
        if !index_path.exists() {
            return Ok(0);
        }
        let content = std::fs::read_to_string(&index_path)
            .map_err(|e| format!("读取缓存索引失败: {}", e))?;
        let mut count = 0;
        for line in content.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 4 {
                let entry = CacheEntry {
                    module_path: parts[0].to_string(),
                    source_hash: parts[1].parse().unwrap_or(0),
                    binary_path: PathBuf::from(parts[2]),
                    compiled_at: parts[3].parse().unwrap_or(0),
                    file_size:   parts.get(4).and_then(|s| s.parse().ok()).unwrap_or(0),
                };
                self.entries.insert(entry.module_path.clone(), entry);
                count += 1;
            }
        }
        Ok(count)
    }

    /// 保存缓存索引到磁盘
    pub fn save(&self) -> Result<(), String> {
        std::fs::create_dir_all(&self.cache_dir)
            .map_err(|e| format!("创建缓存目录失败: {}", e))?;
        let index_path = self.cache_dir.join("index.cache");
        let mut content = String::new();
        for (_, entry) in &self.entries {
            content += &format!("{}\t{}\t{}\t{}\t{}\n",
                entry.module_path,
                entry.source_hash,
                entry.binary_path.display(),
                entry.compiled_at,
                entry.file_size
            );
        }
        std::fs::write(&index_path, content)
            .map_err(|e| format!("写入缓存索引失败: {}", e))
    }

    /// 检查模块是否缓存命中
    pub fn check_hit(&mut self, module_path: &str, source_content: &[u8]) -> Option<&CacheEntry> {
        let hash = Self::compute_hash(source_content);
        if let Some(entry) = self.entries.get(module_path) {
            if entry.is_valid(hash) && entry.binary_path.exists() {
                self.hits += 1;
                return Some(entry);
            }
        }
        self.misses += 1;
        None
    }

    /// 记录编译结果到缓存
    pub fn record(&mut self, module_path: &str, source_content: &[u8], binary_path: PathBuf) {
        let hash = Self::compute_hash(source_content);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let file_size = binary_path.metadata().map(|m| m.len() as usize).unwrap_or(0);
        self.entries.insert(module_path.to_string(), CacheEntry {
            module_path: module_path.to_string(),
            source_hash: hash,
            binary_path,
            compiled_at: now,
            file_size,
        });
    }

    /// 清理过期缓存（超过 max_age_secs 秒的）
    pub fn clean_expired(&mut self, max_age_secs: u64) -> usize {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let before = self.entries.len();
        self.entries.retain(|_, e| {
            now.saturating_sub(e.compiled_at) < max_age_secs
        });
        before - self.entries.len()
    }

    /// 清理所有缓存
    pub fn clear_all(&mut self) {
        self.entries.clear();
        self.hits = 0;
        self.misses = 0;
    }

    /// 统计
    pub fn stats(&self) -> CacheStats {
        let total = self.entries.len();
        let total_size: usize = self.entries.values().map(|e| e.file_size).sum();
        CacheStats {
            entries: total,
            hits: self.hits,
            misses: self.misses,
            hit_rate: if self.hits + self.misses > 0 {
                self.hits * 100 / (self.hits + self.misses)
            } else { 0 },
            total_bytes: total_size,
        }
    }

    /// 判断是否需要重新编译某个模块（基于内容哈希）
    pub fn needs_rebuild(&self, module_path: &str, source_content: &[u8]) -> bool {
        let hash = Self::compute_hash(source_content);
        match self.entries.get(module_path) {
            Some(entry) => !entry.is_valid(hash) || !entry.binary_path.exists(),
            None => true,
        }
    }

    /// 计算内容哈希（FNV-1a 64位）
    pub fn compute_hash(data: &[u8]) -> u64 {
        const FNV_OFFSET: u64 = 14695981039346656037;
        const FNV_PRIME:  u64 = 1099511628211;
        let mut hash = FNV_OFFSET;
        for &byte in data {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        hash
    }

    /// 批量检查一组文件是否需要重编译
    pub fn batch_check<'a>(&self, modules: &'a [(String, Vec<u8>)]) -> Vec<&'a str> {
        modules.iter()
            .filter(|(path, content)| self.needs_rebuild(path, content))
            .map(|(path, _)| path.as_str())
            .collect()
    }
}

#[derive(Debug, Default)]
pub struct CacheStats {
    pub entries:     usize,
    pub hits:        usize,
    pub misses:      usize,
    pub hit_rate:    usize,   // 0-100
    pub total_bytes: usize,
}

impl CacheStats {
    pub fn format(&self) -> String {
        format!(
            "缓存: {}条 | 命中率: {}% ({}/{}) | 磁盘: {:.1}KB",
            self.entries,
            self.hit_rate,
            self.hits,
            self.hits + self.misses,
            self.total_bytes as f64 / 1024.0
        )
    }
}

/// 文件系统级变更检测（更快，用于快速判断）
pub fn file_is_newer(file: &Path, than: &Path) -> bool {
    let file_time = file.metadata().and_then(|m| m.modified()).ok();
    let than_time = than.metadata().and_then(|m| m.modified()).ok();
    match (file_time, than_time) {
        (Some(f), Some(t)) => f > t,
        _ => true,  // 无法比较时保守选择重编译
    }
}

/// 检查一组源文件是否有任何一个比输出文件更新
pub fn any_source_newer(sources: &[&Path], output: &Path) -> bool {
    if !output.exists() { return true; }
    sources.iter().any(|s| file_is_newer(s, output))
}
