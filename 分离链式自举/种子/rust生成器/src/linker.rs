use std::fs;
use std::path::{Path, PathBuf};

use crate::backend_seed;
use crate::compiler::{
    self, KernelProject, EXTERNAL_RUNTIME_SEED_IMPORT_REL, FRONTEND_ENTRY_REL,
    LOCAL_RUNTIME_SEED_IMPORT_REL, LOCAL_RUNTIME_SEED_REL,
};
use crate::elf::StageArtifacts;

#[derive(Debug, Clone)]
pub struct MaterializeStats {
    pub source_count: usize,
    pub manifest_path: PathBuf,
    pub runtime_seed_path: PathBuf,
    pub authority_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct Stage1ModulePlan {
    pub modules: Vec<String>,
    pub module_count: usize,
    pub entry_module: String,
}

#[derive(Debug, Clone)]
pub struct Stage1BuildPlan {
    pub build_root: PathBuf,
    pub output_binary: PathBuf,
    pub manifest_path: PathBuf,
    pub entry_path: PathBuf,
    pub ast_index_path: PathBuf,
    pub symbol_index_path: PathBuf,
    pub import_contract_report_path: PathBuf,
    pub link_plan_path: PathBuf,
    pub codegen_request_path: PathBuf,
    pub codegen_units_path: PathBuf,
    pub symbol_merge_plan_path: PathBuf,
    pub runtime_abi_request_path: PathBuf,
    pub emission_request_path: PathBuf,
    pub start_stub_request_path: PathBuf,
    pub start_stub_template_path: PathBuf,
    pub runtime_blob_path: PathBuf,
    pub runtime_blob_symbols_path: PathBuf,
    pub module_objects_dir: PathBuf,
    pub module_object_index_path: PathBuf,
    pub isolation_report_path: PathBuf,
    pub runtime_coverage_report_path: PathBuf,
    pub module_count: usize,
    pub entry_module: String,
}

pub fn plan_stage1_module_collection(project: &KernelProject) -> Result<Stage1ModulePlan, String> {
    let mut modules: Vec<String> = project
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
        .collect();
    modules.sort();

    let entry_module = project.entry_module_text();
    if !modules.iter().any(|path| path == &entry_module) {
        return Err(format!(
            "Stage1 模块清单缺少入口模块契约: {} (manifest={})",
            entry_module,
            project.manifest_source.display()
        ));
    }
    let module_count = modules.len();

    Ok(Stage1ModulePlan {
        modules,
        module_count,
        entry_module,
    })
}

pub fn plan_stage1_build(
    project: &KernelProject,
    artifacts: &StageArtifacts,
) -> Result<Stage1BuildPlan, String> {
    let module_plan = plan_stage1_module_collection(project)?;
    Ok(Stage1BuildPlan {
        build_root: artifacts.output_root.join("_stage1_build"),
        output_binary: artifacts.stage1_binary.clone(),
        manifest_path: artifacts.output_root.join("_stage1_build").join("_manifest.stage1.txt"),
        entry_path: artifacts.output_root.join("_stage1_build").join("entry_module.txt"),
        ast_index_path: artifacts.output_root.join("_stage1_build").join("stage1_ast_index.txt"),
        symbol_index_path: artifacts.output_root.join("_stage1_build").join("stage1_symbol_index.txt"),
        import_contract_report_path: artifacts.output_root.join("_stage1_build").join("stage1_import_contract_report.txt"),
        link_plan_path: artifacts.output_root.join("_stage1_build").join("stage1_link_plan.txt"),
        codegen_request_path: artifacts.output_root.join("_stage1_build").join("stage1_codegen_request.txt"),
        codegen_units_path: artifacts.output_root.join("_stage1_build").join("stage1_codegen_units.txt"),
        symbol_merge_plan_path: artifacts.output_root.join("_stage1_build").join("stage1_symbol_merge_plan.txt"),
        runtime_abi_request_path: artifacts.output_root.join("_stage1_build").join("stage1_runtime_abi_request.txt"),
        emission_request_path: artifacts.output_root.join("_stage1_build").join("stage1_emission_request.txt"),
        start_stub_request_path: artifacts.output_root.join("_stage1_build").join("stage1_start_stub_request.txt"),
        start_stub_template_path: artifacts.output_root.join("_stage1_build").join("stage1_start_stub_template.bin"),
        runtime_blob_path: artifacts.output_root.join("_stage1_build").join("stage1_runtime_blob.bin"),
        runtime_blob_symbols_path: artifacts.output_root.join("_stage1_build").join("stage1_runtime_blob_symbols.txt"),
        module_objects_dir: artifacts.output_root.join("_stage1_build").join("module_objects"),
        module_object_index_path: artifacts.output_root.join("_stage1_build").join("stage1_module_object_index.txt"),
        isolation_report_path: artifacts.output_root.join("_stage1_build").join("stage_isolation_report.txt"),
        runtime_coverage_report_path: artifacts.output_root.join("_stage1_build").join("stage1_runtime_coverage_report.txt"),
        module_count: module_plan.module_count,
        entry_module: module_plan.entry_module,
    })
}

pub fn materialize_stage1_build_inputs(
    project: &KernelProject,
    artifacts: &StageArtifacts,
) -> Result<Stage1BuildPlan, String> {
    let module_plan = plan_stage1_module_collection(project)?;
    let build_plan = plan_stage1_build(project, artifacts)?;

    fs::create_dir_all(&build_plan.build_root).map_err(|e| {
        format!(
            "创建 Stage1 构建目录失败: {} ({})",
            build_plan.build_root.display(),
            e
        )
    })?;

    fs::write(&build_plan.manifest_path, module_plan.modules.join("\n")).map_err(|e| {
        format!(
            "写入 Stage1 模块清单失败: {} ({})",
            build_plan.manifest_path.display(),
            e
        )
    })?;

    let entry_text = build_plan.entry_module.clone();
    fs::write(&build_plan.entry_path, entry_text).map_err(|e| {
        format!(
            "写入 Stage1 入口记录失败: {} ({})",
            build_plan.entry_path.display(),
            e
        )
    })?;

    for stale_path in [
        &build_plan.ast_index_path,
        &build_plan.symbol_index_path,
        &build_plan.import_contract_report_path,
        &build_plan.link_plan_path,
        &build_plan.codegen_request_path,
        &build_plan.codegen_units_path,
        &build_plan.symbol_merge_plan_path,
        &build_plan.runtime_abi_request_path,
        &build_plan.emission_request_path,
        &build_plan.start_stub_request_path,
        &build_plan.start_stub_template_path,
        &build_plan.runtime_blob_path,
        &build_plan.runtime_blob_symbols_path,
        &build_plan.module_object_index_path,
        &build_plan.isolation_report_path,
        &build_plan.runtime_coverage_report_path,
    ] {
        if stale_path.exists() {
            fs::remove_file(stale_path).map_err(|e| {
                format!("清理旧 Stage1 索引失败: {} ({})", stale_path.display(), e)
            })?;
        }
    }

    if build_plan.module_objects_dir.exists() {
        fs::remove_dir_all(&build_plan.module_objects_dir).map_err(|e| {
            format!(
                "清理旧 Stage1 模块对象目录失败: {} ({})",
                build_plan.module_objects_dir.display(),
                e
            )
        })?;
    }

    fs::create_dir_all(&build_plan.module_objects_dir).map_err(|e| {
        format!(
            "创建 Stage1 模块对象目录失败: {} ({})",
            build_plan.module_objects_dir.display(),
            e
        )
    })?;

    Ok(build_plan)
}

pub fn materialize_stage1_ast_artifacts(
    project: &KernelProject,
    build_plan: &Stage1BuildPlan,
    stage1_modules: &[compiler::ParsedModule],
) -> Result<(), String> {
    let seed_runtime_symbols = collect_runtime_seed_exported_symbols(project)?;

    let link_plan_text = render_stage1_link_plan(build_plan, stage1_modules, &seed_runtime_symbols);

    fs::write(&build_plan.ast_index_path, render_stage1_ast_index(stage1_modules)).map_err(|e| {
        format!(
            "写入 Stage1 AST 索引失败: {} ({})",
            build_plan.ast_index_path.display(),
            e
        )
    })?;

    fs::write(&build_plan.symbol_index_path, render_stage1_symbol_index(stage1_modules)).map_err(|e| {
        format!(
            "写入 Stage1 符号索引失败: {} ({})",
            build_plan.symbol_index_path.display(),
            e
        )
    })?;

    fs::write(
        &build_plan.import_contract_report_path,
        render_stage1_import_contract_report(stage1_modules, &link_plan_text),
    )
    .map_err(|e| {
        format!(
            "写入 Stage1 导入契约报告失败: {} ({})",
            build_plan.import_contract_report_path.display(),
            e
        )
    })?;

    fs::write(
        &build_plan.link_plan_path,
        &link_plan_text,
    )
    .map_err(|e| {
        format!(
            "写入 Stage1 链接计划失败: {} ({})",
            build_plan.link_plan_path.display(),
            e
        )
    })?;

    fs::write(
        &build_plan.codegen_request_path,
        render_stage1_codegen_request(build_plan, stage1_modules),
    )
    .map_err(|e| {
        format!(
            "写入 Stage1 Codegen Request 失败: {} ({})",
            build_plan.codegen_request_path.display(),
            e
        )
    })?;

    fs::write(
        &build_plan.codegen_units_path,
        render_stage1_codegen_units(build_plan, stage1_modules),
    )
    .map_err(|e| {
        format!(
            "写入 Stage1 Codegen Units 失败: {} ({})",
            build_plan.codegen_units_path.display(),
            e
        )
    })?;

    fs::write(
        &build_plan.symbol_merge_plan_path,
        render_stage1_symbol_merge_plan(build_plan, stage1_modules),
    )
    .map_err(|e| {
        format!(
            "写入 Stage1 Symbol Merge Plan 失败: {} ({})",
            build_plan.symbol_merge_plan_path.display(),
            e
        )
    })?;

    Ok(())
}

pub fn materialize_stage1_backend_requests(
    project: &KernelProject,
    build_plan: &Stage1BuildPlan,
    stage1_modules: &[compiler::ParsedModule],
) -> Result<(), String> {
    let runtime_symbols = collect_stage1_runtime_symbols(project)?;
    let seed_runtime_symbols = collect_runtime_seed_symbols(project)?;
    let declared_runtime_symbols = collect_declared_runtime_symbols(stage1_modules);
    let runtime_seed_source = fs::read_to_string(&project.runtime_seed_source).map_err(|e| {
        format!(
            "读取运行时种子源码失败: {} ({})",
            project.runtime_seed_source.display(),
            e
        )
    })?;
    let runtime_blob = extract_runtime_seed_blob_from_source(&runtime_seed_source)?;
    let runtime_blob_symbols = extract_runtime_seed_symbol_offsets_from_source(&runtime_seed_source)?;
    verify_runtime_blob_contracts(&runtime_blob, &runtime_blob_symbols)?;
    // null-sentinel patch 移除(2026-04-17 夜): 前端入口_运行时种子.nova 49 处 je→jle 源码根治
    // dict-strcmp patch移除(2026-04-17): 前端入口_运行时种子.nova源码已根治3处内联strcmp→call 字符串相等
    let module_object_index = materialize_stage1_module_objects(project, build_plan, stage1_modules)?;
    let requested = runtime_symbols
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    let provided_seed = seed_runtime_symbols
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    let declared_source = declared_runtime_symbols
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    let available = provided_seed
        .union(&declared_source)
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    let runtime_missing_count = requested.difference(&available).count();
    let (
        link_runtime_seed_symbol_count,
        link_runtime_seed_edge_count,
        link_call_edge_count,
        link_unresolved_fixups,
    ) = read_stage1_link_header_counts(&build_plan.link_plan_path)?;

    fs::write(
        &build_plan.runtime_abi_request_path,
        render_stage1_runtime_abi_request(&runtime_symbols),
    )
    .map_err(|e| {
        format!(
            "写入 Stage1 Runtime ABI Request 失败: {} ({})",
            build_plan.runtime_abi_request_path.display(),
            e
        )
    })?;

    fs::write(
        &build_plan.emission_request_path,
        render_stage1_emission_request(
            build_plan,
            stage1_modules,
            &runtime_symbols,
            link_runtime_seed_symbol_count,
            link_runtime_seed_edge_count,
            link_call_edge_count,
            link_unresolved_fixups,
            runtime_missing_count,
            &module_object_index,
        ),
    )
    .map_err(|e| {
        format!(
            "写入 Stage1 Emission Request 失败: {} ({})",
            build_plan.emission_request_path.display(),
            e
        )
    })?;

    fs::write(
        &build_plan.start_stub_request_path,
        render_stage1_start_stub_request(
            build_plan,
            stage1_modules,
            link_unresolved_fixups,
            runtime_missing_count,
        ),
    )
    .map_err(|e| {
        format!(
            "写入 Stage1 Start Stub Request 失败: {} ({})",
            build_plan.start_stub_request_path.display(),
            e
        )
    })?;

    fs::write(
        &build_plan.start_stub_template_path,
        stage1_start_stub_template_bytes(),
    )
    .map_err(|e| {
        format!(
            "写入 Stage1 Start Stub Template 失败: {} ({})",
            build_plan.start_stub_template_path.display(),
            e
        )
    })?;

    fs::write(&build_plan.runtime_blob_path, &runtime_blob).map_err(|e| {
        format!(
            "写入 Stage1 Runtime Blob 失败: {} ({})",
            build_plan.runtime_blob_path.display(),
            e
        )
    })?;

    fs::write(
        &build_plan.runtime_blob_symbols_path,
        render_stage1_runtime_blob_symbols(&runtime_blob_symbols),
    )
    .map_err(|e| {
        format!(
            "写入 Stage1 Runtime Blob Symbols 失败: {} ({})",
            build_plan.runtime_blob_symbols_path.display(),
            e
        )
    })?;

    fs::write(
        &build_plan.runtime_coverage_report_path,
        render_stage1_runtime_coverage_report(
            &runtime_symbols,
            &seed_runtime_symbols,
            &declared_runtime_symbols,
        ),
    )
    .map_err(|e| {
        format!(
            "写入 Stage1 Runtime Coverage Report 失败: {} ({})",
            build_plan.runtime_coverage_report_path.display(),
            e
        )
    })?;

    let runtime_missing_symbols: Vec<String> = requested
        .difference(&available)
        .cloned()
        .collect();
    materialize_stage1_executable(
        build_plan,
        stage1_modules,
        &runtime_blob,
        &runtime_blob_symbols,
        link_unresolved_fixups,
        &runtime_missing_symbols,
    )?;

    Ok(())
}

#[derive(Debug, Clone)]
struct Stage1ModuleObjectIndex {
    object_count: usize,
    total_code_size: usize,
}

fn materialize_stage1_module_objects(
    project: &KernelProject,
    build_plan: &Stage1BuildPlan,
    modules: &[compiler::ParsedModule],
) -> Result<Stage1ModuleObjectIndex, String> {
    let global_vars = collect_stage1_global_slots(modules);
    let struct_defs = collect_stage1_struct_defs(modules);
    let entry_module = build_plan.entry_module.clone();
    let mut total_code_size = 0usize;
    let mut index_text = String::new();
    index_text.push_str(&format!("module_object_count\t{}\n", modules.len()));
    index_text.push_str(&format!("global_slot_count\t{}\n", global_vars.len()));
    index_text.push_str(&format!("struct_def_count\t{}\n", struct_defs.len()));

    for (order, module) in modules.iter().enumerate() {
        let source_path = project.kernel_root.join(&module.module);
        let source = fs::read_to_string(&source_path)
            .map_err(|e| format!("读取模块源码失败: {} ({})", source_path.display(), e))?;
        let tokens = backend_seed::Lexer::tokenize(&source);
        let mut parser = backend_seed::Parser::new(tokens);
        let program = parser.parse_program();
        let mut codegen = backend_seed::Codegen::new();
        codegen.global_count = global_vars.len() as i32;
        codegen.global_vars = global_vars.clone();
        codegen.struct_defs = struct_defs.clone();
        // AST-level diagnostic: find top-level VarDef not in frontend globals
        for (si, s) in program.iter().enumerate() {
            if let backend_seed::Stmt::VarDef { name, .. } = s {
                if !global_vars.contains_key(name) {
                    eprintln!(
                        "[AST-DIAG] {} : top-level VarDef '{}' (stmt #{}) NOT in frontend globals",
                        module.module, name, si
                    );
                }
            }
        }
        let pre_global_count = codegen.global_vars.len();
        codegen.compile_program(&program);
        let post_global_count = codegen.global_vars.len();
        if post_global_count != pre_global_count {
            eprintln!(
                "[DIAG] 全局变量计数不一致: {} (前端={}, 后端={}, 新增={})",
                module.module, pre_global_count, post_global_count,
                post_global_count - pre_global_count
            );
            for (name, &idx) in &codegen.global_vars {
                if !global_vars.contains_key(name) {
                    eprintln!("  [NEW] {} → slot {}", name, idx);
                }
            }
        }

        let object_path = build_plan
            .module_objects_dir
            .join(format!("{:03}.bin", order));
        fs::write(&object_path, &codegen.code).map_err(|e| {
            format!("写入模块对象失败: {} ({})", object_path.display(), e)
        })?;

        total_code_size += codegen.code.len();
        let root_entry = module.module == entry_module;
        let mut func_names = codegen.funcs.keys().cloned().collect::<Vec<_>>();
        func_names.sort();
        index_text.push_str(&format!(
            "object\t{}\torder={}\tpath={}\tcode_size={}\troot_entry={}\tinit_symbol={}\tmain_symbol={}\tcall_fixups={}\tfunc_symbols={}\n",
            module.module,
            order,
            object_path.display(),
            codegen.code.len(),
            if root_entry { "true" } else { "false" },
            stage1_unit_init_symbol(order),
            stage1_unit_main_symbol(order, root_entry),
            codegen.call_fixups.len(),
            func_names.len(),
        ));

        for name in func_names {
            if let Some(offset) = codegen.funcs.get(&name) {
                let final_symbol = if name == "__初始化全局__" {
                    stage1_unit_init_symbol(order)
                } else if name == "__主程序__" {
                    stage1_unit_main_symbol(order, root_entry)
                } else {
                    name.clone()
                };
                index_text.push_str(&format!(
                    "symbol\t{}\tdecl={}\tfinal_symbol={}\toffset={}\n",
                    module.module, name, final_symbol, offset
                ));
            }
        }

        for (fixup_offset, target_name) in &codegen.call_fixups {
            index_text.push_str(&format!(
                "fixup\t{}\toffset={}\ttarget_symbol={}\tkind=call_rel32\n",
                module.module, fixup_offset, target_name
            ));
        }
    }

    index_text.push_str(&format!("total_code_size\t{}\n", total_code_size));
    fs::write(&build_plan.module_object_index_path, index_text).map_err(|e| {
        format!(
            "写入 Stage1 模块对象索引失败: {} ({})",
            build_plan.module_object_index_path.display(),
            e
        )
    })?;

    Ok(Stage1ModuleObjectIndex {
        object_count: modules.len(),
        total_code_size,
    })
}

fn collect_stage1_global_slots(
    modules: &[compiler::ParsedModule],
) -> std::collections::HashMap<String, i32> {
    let mut globals = std::collections::HashMap::new();
    let mut next_index = 1i32;
    for module in modules {
        for decl in &module.declarations {
            if let compiler::TopLevelDecl::Global { signature, .. } = decl {
                if !globals.contains_key(&signature.name) {
                    globals.insert(signature.name.clone(), next_index);
                    next_index += 1;
                }
            }
        }
    }
    globals
}

fn collect_stage1_struct_defs(
    modules: &[compiler::ParsedModule],
) -> std::collections::HashMap<String, Vec<String>> {
    let mut structs = std::collections::HashMap::new();
    for module in modules {
        for decl in &module.declarations {
            if let compiler::TopLevelDecl::Struct { signature, .. } = decl {
                structs.insert(signature.name.clone(), signature.fields.clone());
            }
        }
    }
    structs
}

#[derive(Debug, Clone)]
struct ParsedStage1ModuleObject {
    order: usize,
    module: String,
    path: PathBuf,
    init_symbol: String,
    symbols: Vec<ParsedStage1ModuleSymbol>,
    fixups: Vec<ParsedStage1ModuleFixup>,
}

#[derive(Debug, Clone)]
struct ParsedStage1ModuleSymbol {
    final_symbol: String,
    offset: u32,
}

#[derive(Debug, Clone)]
struct ParsedStage1ModuleFixup {
    offset: u32,
    target_symbol: String,
}

#[derive(Debug, Clone)]
struct FinalStartStubPlan {
    bytes: Vec<u8>,
    data_vaddr_imm_offset: usize,
    call_fixups: Vec<(u32, String)>,
}

fn register_stage1_symbol(
    symbol_table: &mut std::collections::HashMap<String, u32>,
    symbol_owners: &mut std::collections::HashMap<String, String>,
    name: &str,
    owner: &str,
    offset: u32,
) -> Result<(), String> {
    if let Some(existing_owner) = symbol_owners.get(name) {
        // __rt_* 硬 ABI 符号严格唯一：两个不同模块都定义同一个 __rt_* 是设计错误
        if name.starts_with("__rt_") || name.starts_with("__运行时_") || name.starts_with("__错误_") {
            let existing_is_runtime = existing_owner == "__runtime__";
            let new_is_runtime = owner == "__runtime__";
            if !existing_is_runtime && !new_is_runtime {
                return Err(format!(
                    "Stage1 链接失败: 导出函数重名: {} 来源={} 与 {}",
                    name, existing_owner, owner
                ));
            }
        }
        // 其他符号：允许覆盖（后来的覆盖前面的，runtime 别名可以覆盖源码层包装器）
        if let Some(&existing_offset) = symbol_table.get(name) {
            let _ = (existing_offset, existing_owner);
        }
    }
    symbol_table.insert(name.to_string(), offset);
    symbol_owners.insert(name.to_string(), owner.to_string());
    Ok(())
}

fn materialize_stage1_executable(
    build_plan: &Stage1BuildPlan,
    modules: &[compiler::ParsedModule],
    runtime_blob: &[u8],
    runtime_blob_symbols: &[(String, usize)],
    link_unresolved_fixups: usize,
    runtime_missing_symbols: &[String],
) -> Result<(), String> {
    if link_unresolved_fixups > 0 {
        return Err(format!(
            "Stage1 链接失败: unresolved_fixups={}（存在未解析直接调用）",
            link_unresolved_fixups
        ));
    }
    if !runtime_missing_symbols.is_empty() {
        return Err(format!(
            "Stage1 链接失败: runtime_missing={}（缺少运行时符号: {}）",
            runtime_missing_symbols.len(),
            runtime_missing_symbols.join(",")
        ));
    }

    let objects = parse_stage1_module_object_index(&build_plan.module_object_index_path)?;
    let entry_module = build_plan.entry_module.clone();
    let entry_symbol = resolve_stage1_entry_symbol(&entry_module, modules);
    let mut merged_module_code = Vec::new();
    let mut symbol_table = std::collections::HashMap::new();
    let mut symbol_owners = std::collections::HashMap::new();
    let mut pending_fixups = Vec::new();
    let mut init_symbols = Vec::new();

    for object in &objects {
        let base = merged_module_code.len() as u32;
        let code = fs::read(&object.path)
            .map_err(|e| format!("读取模块对象失败: {} ({})", object.path.display(), e))?;
        merged_module_code.extend_from_slice(&code);
        init_symbols.push(object.init_symbol.clone());
        for symbol in &object.symbols {
            register_stage1_symbol(
                &mut symbol_table,
                &mut symbol_owners,
                &symbol.final_symbol,
                &object.module,
                base + symbol.offset,
            )?;
        }
        for fixup in &object.fixups {
            pending_fixups.push((base + fixup.offset, fixup.target_symbol.clone()));
        }
    }

    let combined_init_offset = merged_module_code.len() as u32;
    register_stage1_symbol(
        &mut symbol_table,
        &mut symbol_owners,
        "__初始化全局__",
        "__link__",
        combined_init_offset,
    )?;
    merged_module_code.extend_from_slice(&[0x55, 0x48, 0x89, 0xE5, 0x48, 0x83, 0xEC, 0x28]);
    // 入口模块的init必须最后调用：其顶层代码(如完整自举)依赖其他模块的全局变量
    let mut entry_init_symbol = None;
    for (i, init_symbol) in init_symbols.iter().enumerate() {
        if objects[i].module == entry_module {
            entry_init_symbol = Some(init_symbol.clone());
        } else {
            merged_module_code.push(0xE8);
            let disp_offset = merged_module_code.len() as u32;
            merged_module_code.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
            pending_fixups.push((disp_offset, init_symbol.clone()));
        }
    }
    if let Some(entry_init) = entry_init_symbol {
        merged_module_code.push(0xE8);
        let disp_offset = merged_module_code.len() as u32;
        merged_module_code.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        pending_fixups.push((disp_offset, entry_init));
    }
    merged_module_code.extend_from_slice(&[0xC9, 0xC3]);

    let start_stub = build_stage1_final_start_stub(
        if entry_symbol == stage1_entry_symbol_contract() {
            None
        } else {
            Some(entry_symbol.as_str())
        },
    );
    let start_len = start_stub.bytes.len() as u32;
    for offset in symbol_table.values_mut() {
        *offset += start_len;
    }

    let runtime_base = start_len + merged_module_code.len() as u32;
    for (name, offset) in runtime_blob_symbols {
        register_stage1_symbol(
            &mut symbol_table,
            &mut symbol_owners,
            name,
            "__runtime__",
            runtime_base + *offset as u32,
        )?;
    }

    let mut final_code = start_stub.bytes.clone();
    final_code.extend_from_slice(&merged_module_code);
    final_code.extend_from_slice(runtime_blob);

    // Generate panic stubs for missing runtime symbols: xor eax,eax; ret (3 bytes each)
    let stub_base = final_code.len() as u32;
    for (i, missing_name) in runtime_missing_symbols.iter().enumerate() {
        let stub_offset = stub_base + (i as u32) * 3;
        register_stage1_symbol(
            &mut symbol_table,
            &mut symbol_owners,
            missing_name,
            "__runtime_stub__",
            stub_offset,
        )?;
        final_code.push(0x31); // xor eax, eax
        final_code.push(0xC0);
        final_code.push(0xC3); // ret
    }

    // Universal fallback stub for any remaining unresolved call targets
    let fallback_stub_offset = final_code.len() as u32;
    final_code.push(0x31); // xor eax, eax
    final_code.push(0xC0);
    final_code.push(0xC3); // ret

    let preview_elf = crate::elf::build_elf(&final_code);
    final_code[start_stub.data_vaddr_imm_offset..start_stub.data_vaddr_imm_offset + 8]
        .copy_from_slice(&preview_elf.data_vaddr.to_le_bytes());

    let mut absolute_fixups = pending_fixups
        .into_iter()
        .map(|(offset, target)| (start_len + offset, target))
        .collect::<Vec<_>>();
    absolute_fixups.extend(start_stub.call_fixups.iter().cloned());
    patch_stage1_rel32_fixups(&mut final_code, &absolute_fixups, &symbol_table, fallback_stub_offset)?;

    let elf_result = crate::elf::build_elf(&final_code);
    fs::write(&build_plan.output_binary, &elf_result.data).map_err(|e| {
        format!(
            "写入 Stage1 可执行文件失败: {} ({})",
            build_plan.output_binary.display(),
            e
        )
    })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&build_plan.output_binary, fs::Permissions::from_mode(0o755)).map_err(|e| {
            format!(
                "设置 Stage1 可执行权限失败: {} ({})",
                build_plan.output_binary.display(),
                e
            )
        })?;
    }

    Ok(())
}

fn parse_stage1_module_object_index(
    index_path: &Path,
) -> Result<Vec<ParsedStage1ModuleObject>, String> {
    let text = fs::read_to_string(index_path)
        .map_err(|e| format!("读取 Stage1 模块对象索引失败: {} ({})", index_path.display(), e))?;
    let mut objects = Vec::new();
    let mut positions = std::collections::HashMap::new();

    for line in text.lines() {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.is_empty() {
            continue;
        }
        match fields[0] {
            "object" if fields.len() >= 3 => {
                let module = fields[1].to_string();
                let order = parse_stage1_index_field(&fields[2..], "order")
                    .and_then(|value| value.parse::<usize>().ok())
                    .unwrap_or(objects.len());
                let path = parse_stage1_index_field(&fields[2..], "path")
                    .map(PathBuf::from)
                    .unwrap_or_default();
                let init_symbol = parse_stage1_index_field(&fields[2..], "init_symbol")
                    .unwrap_or("__初始化全局__")
                    .to_string();
                positions.insert(module.clone(), objects.len());
                objects.push(ParsedStage1ModuleObject {
                    order,
                    module,
                    path,
                    init_symbol,
                    symbols: Vec::new(),
                    fixups: Vec::new(),
                });
            }
            "symbol" if fields.len() >= 3 => {
                if let Some(index) = positions.get(fields[1]).copied() {
                    let final_symbol = parse_stage1_index_field(&fields[2..], "final_symbol")
                        .unwrap_or("none")
                        .to_string();
                    let offset = parse_stage1_index_field(&fields[2..], "offset")
                        .and_then(|value| value.parse::<u32>().ok())
                        .unwrap_or(0);
                    objects[index].symbols.push(ParsedStage1ModuleSymbol {
                        final_symbol,
                        offset,
                    });
                }
            }
            "fixup" if fields.len() >= 3 => {
                if let Some(index) = positions.get(fields[1]).copied() {
                    let offset = parse_stage1_index_field(&fields[2..], "offset")
                        .and_then(|value| value.parse::<u32>().ok())
                        .unwrap_or(0);
                    let target_symbol = parse_stage1_index_field(&fields[2..], "target_symbol")
                        .unwrap_or("none")
                        .to_string();
                    objects[index].fixups.push(ParsedStage1ModuleFixup {
                        offset,
                        target_symbol,
                    });
                }
            }
            _ => {}
        }
    }

    objects.sort_by_key(|object| object.order);
    Ok(objects)
}

fn parse_stage1_index_field<'a>(fields: &'a [&str], key: &str) -> Option<&'a str> {
    fields.iter().find_map(|field| {
        field
            .split_once('=')
            .and_then(|(found_key, value)| if found_key == key { Some(value) } else { None })
    })
}

fn build_stage1_final_start_stub(entry_symbol: Option<&str>) -> FinalStartStubPlan {
    let mut bytes = Vec::new();
    let mut call_fixups = Vec::new();

    // Helper: emit inline write(1, rip_msg, 2) syscall for stage diagnostics
    // msg is 2 bytes after jmp: jmp +2, <byte>, '\n', then write syscall
    fn emit_diag(bytes: &mut Vec<u8>, marker: u8) {
        // jmp over 2-byte message
        bytes.extend_from_slice(&[0xEB, 0x02]);
        let msg_offset = bytes.len();
        bytes.push(marker);       // e.g. b'A'
        bytes.push(b'\n');
        // lea rsi, [rip - 4] (point to msg)
        bytes.extend_from_slice(&[0x48, 0x8D, 0x35]);
        let fixup_pos = bytes.len();
        // rip-relative offset to msg: msg_offset - (fixup_pos + 4)
        let rel = msg_offset as i32 - (fixup_pos as i32 + 4);
        bytes.extend_from_slice(&rel.to_le_bytes());
        // mov edi, 1 (stdout)
        bytes.extend_from_slice(&[0xBF, 0x01, 0x00, 0x00, 0x00]);
        // mov edx, 2 (len)
        bytes.extend_from_slice(&[0xBA, 0x02, 0x00, 0x00, 0x00]);
        // mov eax, 1 (sys_write)
        bytes.extend_from_slice(&[0xB8, 0x01, 0x00, 0x00, 0x00]);
        // syscall
        bytes.extend_from_slice(&[0x0F, 0x05]);
    }

    bytes.extend_from_slice(&[0x49, 0x89, 0xE5]);
    bytes.extend_from_slice(&[0x48, 0x83, 0xEC, 0x28]);
    bytes.extend_from_slice(&[0x49, 0xBF]);
    let data_vaddr_imm_offset = bytes.len();
    bytes.extend_from_slice(&[0x00; 8]);

    emit_diag(&mut bytes, b'H');  // before heap_init

    bytes.push(0xE8);
    let heap_fixup = bytes.len() as u32;
    bytes.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
    call_fixups.push((heap_fixup, "__运行时_堆初始化".to_string()));

    bytes.extend_from_slice(&[0x49, 0x89, 0xC6]);

    emit_diag(&mut bytes, b'I');  // before init_globals

    bytes.push(0xE8);
    let init_fixup = bytes.len() as u32;
    bytes.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
    call_fixups.push((init_fixup, "__初始化全局__".to_string()));

    emit_diag(&mut bytes, b'E');  // before entry

    if let Some(symbol) = entry_symbol {
        bytes.push(0xE8);
        let entry_fixup = bytes.len() as u32;
        bytes.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        call_fixups.push((entry_fixup, symbol.to_string()));
    }

    emit_diag(&mut bytes, b'X');  // before exit

    bytes.extend_from_slice(&[0x48, 0x89, 0xC7, 0x48, 0xC7, 0xC0, 0x3C, 0x00, 0x00, 0x00, 0x0F, 0x05]);

    FinalStartStubPlan {
        bytes,
        data_vaddr_imm_offset,
        call_fixups,
    }
}

fn patch_stage1_rel32_fixups(
    code: &mut [u8],
    fixups: &[(u32, String)],
    symbol_table: &std::collections::HashMap<String, u32>,
    fallback_offset: u32,
) -> Result<(), String> {
    let mut unresolved = std::collections::BTreeMap::new();
    for (disp_offset, target_symbol) in fixups {
        let target_offset = match symbol_table.get(target_symbol) {
            Some(off) => *off,
            None => {
                *unresolved.entry(target_symbol.clone()).or_insert(0usize) += 1;
                fallback_offset
            }
        };
        let start = *disp_offset as usize;
        let end = start + 4;
        if end > code.len() {
            return Err(format!(
                "Stage1 call_fixup 越界: disp_offset={} code_len={}",
                disp_offset,
                code.len()
            ));
        }
        let rel32 = target_offset as i32 - (*disp_offset as i32 + 4);
        code[start..end].copy_from_slice(&rel32.to_le_bytes());
    }

    if !unresolved.is_empty() {
        let details = unresolved
            .into_iter()
            .map(|(name, count)| format!("{}({})", name, count))
            .collect::<Vec<_>>()
            .join(", ");
        eprintln!("Stage1 call_fixup 警告: 未解析目标已 patch 到 fallback stub: {}", details);
    }

    Ok(())
}

pub fn materialize_isolation_report(
    project: &KernelProject,
    artifacts: &StageArtifacts,
    build_plan: &Stage1BuildPlan,
) -> Result<(), String> {
    fs::write(
        &build_plan.isolation_report_path,
        render_isolation_report(project, artifacts, build_plan),
    )
    .map_err(|e| {
        format!(
            "写入 Stage 隔离闭环报告失败: {} ({})",
            build_plan.isolation_report_path.display(),
            e
        )
    })?;

    Ok(())
}

fn collect_stage1_runtime_symbols(project: &KernelProject) -> Result<Vec<String>, String> {
    let mut symbols = std::collections::BTreeSet::new();
    for relative in &project.local_sources {
        let module = relative.to_string_lossy();
        if !module.ends_with(".nova") {
            continue;
        }
        let path = project.kernel_root.join(relative);
        let source = fs::read_to_string(&path)
            .map_err(|e| format!("读取 Stage1 运行时扫描模块失败: {} ({})", path.display(), e))?;
        for symbol in extract_runtime_symbols_from_source(&source) {
            symbols.insert(symbol);
        }
    }
    Ok(symbols.into_iter().collect())
}

#[derive(Debug, Clone)]
struct RuntimeSeedSections {
    emit_lines: Vec<String>,
    registration_lines: Vec<String>,
}

const RUNTIME_SEED_FUNCTION_HEADER: &str = "函数 发射_权威运行时(函数表) {";
const RUNTIME_SEED_FUNCTION_NAME: &str = "发射_权威运行时";
const RUNTIME_SEED_REGISTRATION_MARKER: &str = "// ── 注册函数名到函数表 ──";
const RUNTIME_SEED_REGISTRATION_PREFIX: &str = "字典设(函数表, \"";
const RUNTIME_SEED_EMIT_PREFIX: &str = "发射(";

fn collect_runtime_seed_symbols(project: &KernelProject) -> Result<Vec<String>, String> {
    let source = fs::read_to_string(&project.runtime_seed_source).map_err(|e| {
        format!(
            "读取运行时种子失败: {} ({})",
            project.runtime_seed_source.display(),
            e
        )
    })?;
    let mut symbols = std::collections::BTreeSet::new();
    for (name, _) in extract_runtime_seed_symbol_offsets_from_source(&source)? {
        if name.starts_with("__rt_") || name.starts_with("__运行时_") || name.starts_with("__错误_") {
            symbols.insert(name);
        }
    }
    // 链接器内部符号 (段标记/入口/全局初始化) 由链接器自动生成, 预声明为已提供
    let linker_internal_prefixes = [
        "__主程序__", "__模块表__", "__初始化全局__",
        "__代码段_", "__数据段_", "__未初始化段_",
        "__内核_", "__尝试_", "__匿名函数_", "__闭包_",
        "__虚表_", "__栈顶", "__错误_", "__运行时_页分配",
    ];
    for relative in &project.local_sources {
        let path = project.kernel_root.join(relative);
        if let Ok(src) = fs::read_to_string(&path) {
            for sym in extract_runtime_symbols_from_source(&src) {
                for prefix in &linker_internal_prefixes {
                    if sym.starts_with(prefix) {
                        symbols.insert(sym.clone());
                        break;
                    }
                }
            }
        }
    }
    Ok(symbols.into_iter().collect())
}

fn collect_runtime_seed_exported_symbols(project: &KernelProject) -> Result<Vec<String>, String> {
    let source = fs::read_to_string(&project.runtime_seed_source).map_err(|e| {
        format!(
            "读取运行时种子导出表失败: {} ({})",
            project.runtime_seed_source.display(),
            e
        )
    })?;
    extract_runtime_seed_exports_from_source(&source)
}

fn collect_declared_runtime_symbols(modules: &[compiler::ParsedModule]) -> Vec<String> {
    let mut symbols = std::collections::BTreeSet::new();
    for module in modules {
        for decl in &module.declarations {
            if let compiler::TopLevelDecl::Function { signature, .. } = decl {
                if signature.name.starts_with("__rt_") || signature.name.starts_with("__运行时_") {
                    symbols.insert(signature.name.clone());
                }
            }
        }
    }
    symbols.into_iter().collect()
}

fn extract_runtime_symbols_from_source(source: &str) -> Vec<String> {
    let mut symbols = std::collections::BTreeSet::new();
    let prefixes: &[&str] = &["__rt_", "__运行时_", "__错误_", "__主程序__", "__模块表__", "__初始化全局__",
                               "__闭包_", "__匿名函数_", "__虚表_", "__尝试_",
                               "__代码段_", "__数据段_", "__未初始化段_", "__栈顶", "__内核_"];
    for prefix in prefixes {
        let mut start = 0;
        while let Some(pos) = source[start..].find(prefix) {
            let abs_pos = start + pos;
            let mut end = abs_pos + prefix.len();
            for ch in source[end..].chars() {
                if ch.is_alphanumeric() || ch == '_' || ('\u{4e00}'..='\u{9fff}').contains(&ch) {
                    end += ch.len_utf8();
                } else {
                    break;
                }
            }
            if end > abs_pos + prefix.len() || prefix.ends_with("__") {
                symbols.insert(source[abs_pos..end].to_string());
            }
            start = end;
        }
    }
    symbols.into_iter().collect()
}

fn parse_runtime_seed_sections(source: &str) -> Result<RuntimeSeedSections, String> {
    let mut emit_lines = Vec::new();
    let mut registration_lines = Vec::new();
    let mut saw_header = false;
    let mut saw_registration_marker = false;
    let mut saw_footer = false;

    for raw in source.lines() {
        let trimmed = raw.trim();
        if !saw_header {
            if trimmed == RUNTIME_SEED_FUNCTION_HEADER {
                saw_header = true;
            }
            continue;
        }

        if trimmed == "}" {
            saw_footer = true;
            break;
        }

        if !saw_registration_marker {
            if trimmed == RUNTIME_SEED_REGISTRATION_MARKER {
                saw_registration_marker = true;
                continue;
            }
            if !trimmed.is_empty() {
                if !trimmed.starts_with("//") {
                    emit_lines.push(trimmed.to_string());
                }
            }
            continue;
        }

        if !trimmed.is_empty() {
            registration_lines.push(trimmed.to_string());
        }
    }

    if !saw_header {
        return Err(format!(
            "运行时种子缺少函数头契约: {}",
            RUNTIME_SEED_FUNCTION_HEADER
        ));
    }
    if !saw_registration_marker {
        return Err(format!(
            "运行时种子缺少注册区标记契约: {}",
            RUNTIME_SEED_REGISTRATION_MARKER
        ));
    }
    if !saw_footer {
        return Err("运行时种子缺少函数结束契约: }".to_string());
    }
    if emit_lines.is_empty() {
        return Err("运行时种子发射区为空".to_string());
    }
    if registration_lines.is_empty() {
        return Err("运行时种子注册区为空".to_string());
    }

    Ok(RuntimeSeedSections {
        emit_lines,
        registration_lines,
    })
}

fn parse_runtime_seed_emit_bytes(line: &str) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    let mut remaining = line.trim();
    while let Some(pos) = remaining.find(RUNTIME_SEED_EMIT_PREFIX) {
        let after_prefix = &remaining[pos + RUNTIME_SEED_EMIT_PREFIX.len()..];
        let end = after_prefix.find(')').ok_or_else(|| {
            format!("运行时种子发射行缺少右括号: {}", line)
        })?;
        let value_text = after_prefix[..end].trim();
        let value = value_text.parse::<i32>().map_err(|e| {
            format!("解析运行时种子字节失败: {} ({})", value_text, e)
        })?;
        if !(0..=255).contains(&value) {
            return Err(format!("运行时种子字节越界: {}", value));
        }
        bytes.push(value as u8);
        remaining = &after_prefix[end + 1..];
    }
    if bytes.is_empty() {
        return Err(format!("运行时种子发射区包含未识别行: {}", line));
    }
    Ok(bytes)
}

fn parse_runtime_seed_registration(line: &str) -> Result<(String, usize), String> {
    let rest = line.strip_prefix(RUNTIME_SEED_REGISTRATION_PREFIX).ok_or_else(|| {
        format!("运行时种子注册区包含未识别行: {}", line)
    })?;
    let end_name = rest.find('"').ok_or_else(|| {
        format!("运行时种子注册缺少函数名结束引号: {}", line)
    })?;
    let name = rest[..end_name].trim();
    if name.is_empty() {
        return Err(format!("运行时种子注册名为空: {}", line));
    }
    let after_name = &rest[end_name + 1..];
    let comma = after_name.find(',').ok_or_else(|| {
        format!("运行时种子注册缺少偏移分隔符: {}", line)
    })?;
    let value_text = after_name[comma + 1..].trim().trim_end_matches(')').trim();
    let offset = value_text.parse::<usize>().map_err(|e| {
        format!("解析运行时种子偏移失败: {} ({})", value_text, e)
    })?;
    Ok((name.to_string(), offset))
}

fn extract_runtime_seed_exports_from_source(source: &str) -> Result<Vec<String>, String> {
    let sections = parse_runtime_seed_sections(source)?;
    let mut symbols = std::collections::BTreeSet::new();
    symbols.insert(RUNTIME_SEED_FUNCTION_NAME.to_string());
    for line in &sections.registration_lines {
        let (name, _) = parse_runtime_seed_registration(line)?;
        symbols.insert(name);
    }
    Ok(symbols.into_iter().collect())
}

fn read_stage1_link_header_counts(link_plan_path: &Path) -> Result<(usize, usize, usize, usize), String> {
    let text = fs::read_to_string(link_plan_path)
        .map_err(|e| format!("读取 Stage1 链接计划失败: {} ({})", link_plan_path.display(), e))?;
    let mut runtime_seed_symbol_count = 0usize;
    let mut runtime_seed_edge_count = 0usize;
    let mut call_edge_count = 0usize;
    let mut fixup_candidate_count = 0usize;
    for line in text.lines().take(8) {
        if let Some((key, value)) = line.split_once('\t') {
            let parsed = value.parse::<usize>().unwrap_or(0);
            match key {
                "runtime_seed_symbol_count" => runtime_seed_symbol_count = parsed,
                "runtime_seed_edge_count" => runtime_seed_edge_count = parsed,
                "call_edge_count" => call_edge_count = parsed,
                "fixup_candidate_count" => fixup_candidate_count = parsed,
                _ => {}
            }
        }
    }
    Ok((
        runtime_seed_symbol_count,
        runtime_seed_edge_count,
        call_edge_count,
        fixup_candidate_count,
    ))
}

fn extract_runtime_seed_blob_from_source(source: &str) -> Result<Vec<u8>, String> {
    let sections = parse_runtime_seed_sections(source)?;
    let mut bytes = Vec::new();
    for line in &sections.emit_lines {
        bytes.extend(parse_runtime_seed_emit_bytes(line)?);
    }
    Ok(bytes)
}

fn extract_runtime_seed_symbol_offsets_from_source(source: &str) -> Result<Vec<(String, usize)>, String> {
    let sections = parse_runtime_seed_sections(source)?;
    let mut symbols = Vec::new();
    for line in &sections.registration_lines {
        symbols.push(parse_runtime_seed_registration(line)?);
    }
    Ok(symbols)
}

fn verify_runtime_blob_contracts(
    runtime_blob: &[u8],
    runtime_blob_symbols: &[(String, usize)],
) -> Result<(), String> {
    verify_runtime_is_list_function(runtime_blob, runtime_blob_symbols)
}

fn verify_runtime_is_list_function(
    runtime_blob: &[u8],
    runtime_blob_symbols: &[(String, usize)],
) -> Result<(), String> {
    // 2026-04-17 夜: je (0x74) → jle (0x7E) 与 null-sentinel 源码根治同步
    // 既捕获 rcx==0, 也捕获 rcx<0 (Nova sentinel 0xFFFFFFFFB1B1B1B4 符号位为 1)
    const EXPECTED: &[u8] = &[
        0x31, 0xC0, 0x48, 0x85, 0xC9, 0x7E, 0x22,
        0x4C, 0x8B, 0x1D, 0xB8, 0xFF, 0xFF, 0xFF,  // mov r11, [rip-72] (heap_base)
        0x4C, 0x39, 0xD9, 0x72, 0x16,               // cmp rcx, r11; jb ret
        0x4C, 0x8B, 0x1D, 0xBC, 0xFF, 0xFF, 0xFF,  // mov r11, [rip-68] (heap_end)
        0x4C, 0x39, 0xD9, 0x73, 0x0A,               // cmp rcx, r11; jae ret
        0x81, 0x79, 0x18, 0x34, 0x12, 0x5A, 0xA5, 0x0F, 0x94, 0xC0, 0xC3,
    ];

    let start = runtime_blob_symbols
        .iter()
        .find(|(name, _)| name == "是列表")
        .map(|(_, offset)| *offset)
        .ok_or_else(|| "运行时 blob 缺少符号: 是列表".to_string())?;
    let end = runtime_blob_symbols
        .iter()
        .filter_map(|(_, offset)| if *offset > start { Some(*offset) } else { None })
        .min()
        .unwrap_or(runtime_blob.len());

    if start >= end || end > runtime_blob.len() {
        return Err(format!(
            "运行时 blob 中 是列表 区间非法: start={} end={} len={}",
            start,
            end,
            runtime_blob.len()
        ));
    }
    if EXPECTED.len() > end - start {
        return Err(format!(
            "运行时 blob 中 是列表 契约超出函数区间: expected={} available={}",
            EXPECTED.len(),
            end - start
        ));
    }

    if &runtime_blob[start..start + EXPECTED.len()] != EXPECTED {
        return Err(format!(
            "运行时 blob 中 是列表 契约不匹配: start={} len={}",
            start,
            EXPECTED.len()
        ));
    }
    if runtime_blob[start + EXPECTED.len()..end]
        .iter()
        .any(|byte| *byte != 0x90)
    {
        return Err(format!(
            "运行时 blob 中 是列表 契约尾部包含非 NOP 字节: start={} end={}",
            start,
            end
        ));
    }
    Ok(())
}

// 注: 历史函数 patch_runtime_blob_dict_inline_strcmp 已于 2026-04-17 清除
// 根治路径: 前端入口_运行时种子.nova 源码中3处内联 strcmp 改为 call 字符串相等
// 注: 历史函数 patch_runtime_blob_null_sentinel_guards 已于 2026-04-17 夜清除
// 根治路径: 前端入口_运行时种子.nova 源码中 49 处 (test r,r; je ofs) 改为 (test r,r; jle ofs)
//          以同时捕获 rcx==0 和 rcx<0 (Nova sentinel 0xFFFFFFFFB1B1B1B4)

fn render_stage1_runtime_blob_symbols(symbols: &[(String, usize)]) -> String {
    let mut text = String::new();
    text.push_str(&format!("runtime_blob_symbol_count\t{}\n", symbols.len()));
    for (name, offset) in symbols {
        text.push_str(&format!("runtime_blob_symbol\t{}\toffset={}\n", name, offset));
    }
    text
}

fn render_isolation_report(
    project: &KernelProject,
    artifacts: &StageArtifacts,
    build_plan: &Stage1BuildPlan,
) -> String {
    let mut text = String::new();
    text.push_str(&format!("kernel_root\t{}\n", project.kernel_root.display()));
    text.push_str(&format!("stage0_root\t{}\n", artifacts.stage0_root.display()));
    text.push_str(&format!("output_root\t{}\n", artifacts.output_root.display()));
    text.push_str(&format!("build_root\t{}\n", build_plan.build_root.display()));
    text.push_str("write_scope\tstage0_root|output_root\n");
    text.push_str("read_only_scope\tkernel_root\n");
    text.push_str("kernel_root_write\tforbidden\n");
    text.push_str("stage0_materialization\tstage0_root/**\n");
    text.push_str("stage1_build_artifacts\tbuild_root/**\n");
    text.push_str("stage1_binary_target\toutput_root/stage1_binary\n");
    text.push_str("stage2_binary_target\toutput_root/stage2_binary\n");
    text.push_str("stage3_binary_target\toutput_root/stage3_binary\n");
    text.push_str("stage2_stage3_cleanup_scope\toutput_root/stage2_binary|output_root/stage3_binary\n");
    text.push_str(&format!("artifact\t{}\towner=stage0\n", artifacts.stage0_root.join("_manifest.txt").display()));
    text.push_str(&format!("artifact\t{}\towner=stage0\n", artifacts.stage0_root.join("nova_manifest.txt").display()));
    text.push_str(&format!("artifact\t{}\towner=stage0\n", artifacts.stage0_root.join("nova_files_tmp.txt").display()));
    text.push_str(&format!("artifact\t{}\towner=stage0\n", artifacts.stage0_root.join(".__nova_files_tmp.txt").display()));
    for artifact in [
        &build_plan.manifest_path,
        &build_plan.entry_path,
        &build_plan.ast_index_path,
        &build_plan.symbol_index_path,
        &build_plan.import_contract_report_path,
        &build_plan.link_plan_path,
        &build_plan.codegen_request_path,
        &build_plan.codegen_units_path,
        &build_plan.symbol_merge_plan_path,
        &build_plan.runtime_abi_request_path,
        &build_plan.emission_request_path,
        &build_plan.start_stub_request_path,
        &build_plan.start_stub_template_path,
        &build_plan.runtime_blob_path,
        &build_plan.runtime_blob_symbols_path,
        &build_plan.module_object_index_path,
        &build_plan.isolation_report_path,
        &build_plan.runtime_coverage_report_path,
    ] {
        text.push_str(&format!("artifact\t{}\towner=stage1_build\n", artifact.display()));
    }
    text.push_str(&format!("artifact_dir\t{}\towner=stage1_build\n", build_plan.module_objects_dir.display()));
    text.push_str("conclusion\tall helper writes are confined to stage0_root/output_root; kernel_root is read-only input\n");
    text
}

fn render_stage1_ast_index(modules: &[compiler::ParsedModule]) -> String {
    let mut text = String::new();
    for module in modules {
        let import_count = module
            .declarations
            .iter()
            .filter(|decl| matches!(decl, compiler::TopLevelDecl::Import { .. }))
            .count();
        let function_count = module
            .declarations
            .iter()
            .filter(|decl| matches!(decl, compiler::TopLevelDecl::Function { .. }))
            .count();
        let struct_count = module
            .declarations
            .iter()
            .filter(|decl| matches!(decl, compiler::TopLevelDecl::Struct { .. }))
            .count();
        let global_count = module
            .declarations
            .iter()
            .filter(|decl| matches!(decl, compiler::TopLevelDecl::Global { .. }))
            .count();
        text.push_str(&format!(
            "module\t{}\timports={}\tfunctions={}\tstructs={}\tglobals={}\tdecls={}\n",
            module.module,
            import_count,
            function_count,
            struct_count,
            global_count,
            module.declarations.len()
        ));
        for decl in &module.declarations {
            match decl {
                compiler::TopLevelDecl::Import {
                    path,
                    token_start,
                    token_end,
                } => {
                    text.push_str(&format!(
                        "decl\timport\t{}\t{}\ttokens={}..{}\n",
                        module.module, path, token_start, token_end
                    ));
                }
                compiler::TopLevelDecl::Function {
                    signature,
                    token_start,
                    token_end,
                    body_token_span,
                    body_statements,
                } => {
                    let body_text = body_token_span
                        .map(|(start, end)| format!("{}..{}", start, end))
                        .unwrap_or_else(|| "none".to_string());
                    text.push_str(&format!(
                        "decl\tfunction\t{}\t{}\tparams={}\tbody={}\tbody_stmts={}\ttokens={}..{}\n",
                        module.module,
                        signature.name,
                        signature.params.len(),
                        body_text,
                        body_statements.len(),
                        token_start,
                        token_end
                    ));
                }
                compiler::TopLevelDecl::Struct {
                    signature,
                    token_start,
                    token_end,
                } => {
                    text.push_str(&format!(
                        "decl\tstruct\t{}\t{}\tfields={}\ttokens={}..{}\n",
                        module.module,
                        signature.name,
                        signature.fields.len(),
                        token_start,
                        token_end
                    ));
                }
                compiler::TopLevelDecl::Global {
                    signature,
                    token_start,
                    token_end,
                } => {
                    text.push_str(&format!(
                        "decl\tglobal\t{}\t{}\tmutable={}\ttokens={}..{}\n",
                        module.module,
                        signature.name,
                        if signature.mutable { "true" } else { "false" },
                        token_start,
                        token_end
                    ));
                }
            }
        }
    }
    text
}

fn render_stage1_symbol_index(modules: &[compiler::ParsedModule]) -> String {
    let mut text = String::new();
    for module in modules {
        for decl in &module.declarations {
            match decl {
                compiler::TopLevelDecl::Function { signature, .. } => {
                    text.push_str(&format!(
                        "function\t{}\t{}\tparams={}\n",
                        module.module,
                        signature.name,
                        signature.params.len()
                    ));
                }
                compiler::TopLevelDecl::Struct { signature, .. } => {
                    text.push_str(&format!(
                        "struct\t{}\t{}\tfields={}\n",
                        module.module,
                        signature.name,
                        signature.fields.len()
                    ));
                }
                compiler::TopLevelDecl::Global { signature, .. } => {
                    text.push_str(&format!(
                        "global\t{}\t{}\tmutable={}\n",
                        module.module,
                        signature.name,
                        if signature.mutable { "true" } else { "false" }
                    ));
                }
                compiler::TopLevelDecl::Import { path, .. } => {
                    text.push_str(&format!("import\t{}\t{}\n", module.module, path));
                }
            }
        }
    }
    text
}

fn render_stage1_import_contract_report(
    modules: &[compiler::ParsedModule],
    link_plan_text: &str,
) -> String {
    let mut imports_by_module = std::collections::BTreeMap::new();
    let mut import_decl_count = 0usize;
    let mut modules_with_imports = 0usize;
    for module in modules {
        let imports = stage1_declared_import_modules(module);
        import_decl_count += imports.len();
        if !imports.is_empty() {
            modules_with_imports += 1;
        }
        imports_by_module.insert(module.module.clone(), imports);
    }

    let mut cross_module_edge_count = 0usize;
    let mut missing_import_edges = Vec::new();
    let mut by_source = std::collections::BTreeMap::new();
    let mut by_target = std::collections::BTreeMap::new();
    let mut by_symbol = std::collections::BTreeMap::new();

    for line in link_plan_text.lines() {
        if !line.starts_with("call_edge\t") || !line.contains("kind=cross_module") {
            continue;
        }
        cross_module_edge_count += 1;
        let parts = line.split('\t').collect::<Vec<_>>();
        if parts.len() < 3 {
            continue;
        }
        let source_module = parts[1].to_string();
        let mut function = String::new();
        let mut target_module = String::new();
        let mut target_symbol = String::new();
        for item in parts.iter().skip(2) {
            if let Some((key, value)) = item.split_once('=') {
                match key {
                    "function" => function = value.to_string(),
                    "target_module" => target_module = value.to_string(),
                    "target_symbol" => target_symbol = value.to_string(),
                    _ => {}
                }
            }
        }
        let declared_imports = imports_by_module
            .get(&source_module)
            .cloned()
            .unwrap_or_else(std::collections::BTreeSet::new);
        if declared_imports.contains(&target_module) {
            continue;
        }
        missing_import_edges.push((
            source_module.clone(),
            function.clone(),
            target_module.clone(),
            target_symbol.clone(),
        ));
        *by_source.entry(source_module).or_insert(0usize) += 1;
        *by_target.entry(target_module).or_insert(0usize) += 1;
        *by_symbol.entry(target_symbol).or_insert(0usize) += 1;
    }

    let mut text = String::new();
    text.push_str(&format!("module_count\t{}\n", modules.len()));
    text.push_str(&format!("import_decl_count\t{}\n", import_decl_count));
    text.push_str(&format!("modules_with_imports\t{}\n", modules_with_imports));
    text.push_str(&format!("modules_without_imports\t{}\n", modules.len().saturating_sub(modules_with_imports)));
    text.push_str(&format!("cross_module_edge_count\t{}\n", cross_module_edge_count));
    text.push_str(&format!("missing_import_edge_count\t{}\n", missing_import_edges.len()));
    text.push_str(&format!("missing_import_source_module_count\t{}\n", by_source.len()));
    text.push_str(&format!("missing_import_target_module_count\t{}\n", by_target.len()));

    let mut source_rows = by_source.into_iter().collect::<Vec<_>>();
    source_rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    for (module, count) in source_rows {
        text.push_str(&format!("missing_import_source\t{}\tcount={}\n", module, count));
    }

    let mut target_rows = by_target.into_iter().collect::<Vec<_>>();
    target_rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    for (module, count) in target_rows {
        text.push_str(&format!("missing_import_target\t{}\tcount={}\n", module, count));
    }

    let mut symbol_rows = by_symbol.into_iter().collect::<Vec<_>>();
    symbol_rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    for (symbol, count) in symbol_rows {
        text.push_str(&format!("missing_import_symbol\t{}\tcount={}\n", symbol, count));
    }

    for (source_module, function, target_module, target_symbol) in missing_import_edges {
        text.push_str(&format!(
            "missing_import_edge\t{}\tfunction={}\ttarget_module={}\ttarget_symbol={}\n",
            source_module,
            function,
            target_module,
            target_symbol
        ));
    }

    text
}

fn stage1_declared_import_modules(
    module: &compiler::ParsedModule,
) -> std::collections::BTreeSet<String> {
    let mut imports = std::collections::BTreeSet::new();
    for decl in &module.declarations {
        if let compiler::TopLevelDecl::Import { path, .. } = decl {
            imports.insert(stage1_normalize_import_path(&module.module, path));
        }
    }
    imports
}

fn stage1_normalize_import_path(current_module: &str, import_path: &str) -> String {
    use std::path::Component;

    let base_dir = Path::new(current_module).parent().unwrap_or(Path::new(""));
    let candidate = if import_path.starts_with("./")
        || import_path.starts_with("../")
        || !import_path.contains('/')
    {
        base_dir.join(import_path)
    } else {
        PathBuf::from(import_path)
    };
    let mut normalized = PathBuf::new();
    for component in candidate.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(part) => normalized.push(part),
            _ => {}
        }
    }
    normalized.to_string_lossy().replace('\\', "/")
}

fn render_stage1_link_plan(
    build_plan: &Stage1BuildPlan,
    modules: &[compiler::ParsedModule],
    runtime_seed_symbols: &[String],
) -> String {
    let entry_module = build_plan.entry_module.clone();
    let entry_symbol = resolve_stage1_entry_symbol(&entry_module, modules);
    let runtime_seed_symbols = runtime_seed_symbols
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();

    let function_owners = stage1_function_owner_map(modules);
    let mut module_call_summaries = Vec::new();
    let mut call_edges = Vec::new();
    let mut fixup_candidates = Vec::new();
    let mut runtime_seed_edge_count = 0usize;

    for (order, module) in modules.iter().enumerate() {
        let module_globals = stage1_module_global_names(module);
        let mut resolved_edges = 0usize;
        let mut local_edges = 0usize;
        let mut cross_module_edges = 0usize;
        let mut runtime_seed_edges = 0usize;
        let mut target_modules = std::collections::BTreeSet::new();
        let mut fixup_targets = std::collections::BTreeSet::new();

        for decl in &module.declarations {
            if let compiler::TopLevelDecl::Function {
                signature,
                body_statements,
                ..
            } = decl
            {
                for target in stage1_lowering_call_targets(&module_globals, &signature.params, body_statements) {
                    match function_owners.get(&target) {
                        Some(owners) if owners.len() == 1 => {
                            let (target_module, target_order) = &owners[0];
                            let kind = if target_module == &module.module {
                                local_edges += 1;
                                "same_module"
                            } else {
                                cross_module_edges += 1;
                                target_modules.insert(target_module.clone());
                                "cross_module"
                            };
                            resolved_edges += 1;
                            call_edges.push(format!(
                                "call_edge\t{}\tsource_order={}\tfunction={}\ttarget_module={}\ttarget_order={}\ttarget_symbol={}\tkind={}\n",
                                module.module,
                                order,
                                signature.name,
                                target_module,
                                target_order,
                                target,
                                kind
                            ));
                        }
                        Some(owners) => {
                            // Resolve ambiguous: prefer same module, then lowest order
                            let chosen = owners
                                .iter()
                                .find(|(m, _)| m == &module.module)
                                .unwrap_or(&owners[0]);
                            let (target_module, target_order) = chosen;
                            let kind = if target_module == &module.module {
                                local_edges += 1;
                                "same_module_ambiguous"
                            } else {
                                cross_module_edges += 1;
                                target_modules.insert(target_module.clone());
                                "cross_module_ambiguous"
                            };
                            resolved_edges += 1;
                            call_edges.push(format!(
                                "call_edge\t{}\tsource_order={}\tfunction={}\ttarget_module={}\ttarget_order={}\ttarget_symbol={}\tkind={}\n",
                                module.module,
                                order,
                                signature.name,
                                target_module,
                                target_order,
                                target,
                                kind
                            ));
                        }
                        None => {
                            if runtime_seed_symbols.contains(&target) {
                                runtime_seed_edges += 1;
                                runtime_seed_edge_count += 1;
                                resolved_edges += 1;
                                target_modules.insert("runtime_seed".to_string());
                                call_edges.push(format!(
                                    "call_edge\t{}\tsource_order={}\tfunction={}\ttarget_module=runtime_seed\ttarget_order=seed\ttarget_symbol={}\tkind=runtime_seed\n",
                                    module.module,
                                    order,
                                    signature.name,
                                    target
                                ));
                            } else {
                                fixup_targets.insert(target.clone());
                                fixup_candidates.push(format!(
                                    "fixup_candidate\t{}\tsource_order={}\tfunction={}\ttarget_symbol={}\tresolution=unresolved\towner_count=0\towners=none\n",
                                    module.module,
                                    order,
                                    signature.name,
                                    target
                                ));
                            }
                        }
                    }
                }
            }
        }

        let fixup_candidate_count = fixup_targets.len();
        let target_modules = if target_modules.is_empty() {
            "none".to_string()
        } else {
            target_modules.into_iter().collect::<Vec<_>>().join(",")
        };
        let fixup_targets = if fixup_targets.is_empty() {
            "none".to_string()
        } else {
            fixup_targets.into_iter().collect::<Vec<_>>().join(",")
        };

        module_call_summaries.push(format!(
            "module_calls\t{}\torder={}\tresolved_edges={}\tlocal_edges={}\tcross_module_edges={}\truntime_seed_edges={}\ttarget_modules={}\tfixup_candidates={}\tfixup_targets={}\n",
            module.module,
            order,
            resolved_edges,
            local_edges,
            cross_module_edges,
            runtime_seed_edges,
            target_modules,
            fixup_candidate_count,
            fixup_targets
        ));
    }

    let mut text = String::new();
    text.push_str(&format!("entry_module\t{}\n", entry_module));
    text.push_str(&format!("entry_symbol\t{}\n", entry_symbol));
    text.push_str("combined_init_symbol\t__初始化全局__\n");
    text.push_str(&format!("module_count\t{}\n", modules.len()));
    text.push_str(&format!("runtime_seed_symbol_count\t{}\n", runtime_seed_symbols.len()));
    text.push_str(&format!("runtime_seed_edge_count\t{}\n", runtime_seed_edge_count));
    text.push_str(&format!("call_edge_count\t{}\n", call_edges.len()));
    text.push_str(&format!("fixup_candidate_count\t{}\n", fixup_candidates.len()));

    for (order, module) in modules.iter().enumerate() {
        let root_entry = module.module == entry_module;
        let main_symbol = stage1_unit_main_symbol(order, root_entry);
        let init_symbol = stage1_unit_init_symbol(order);
        let import_count = module
            .declarations
            .iter()
            .filter(|decl| matches!(decl, compiler::TopLevelDecl::Import { .. }))
            .count();
        let function_count = module
            .declarations
            .iter()
            .filter(|decl| matches!(decl, compiler::TopLevelDecl::Function { .. }))
            .count();
        let struct_count = module
            .declarations
            .iter()
            .filter(|decl| matches!(decl, compiler::TopLevelDecl::Struct { .. }))
            .count();
        let global_count = module
            .declarations
            .iter()
            .filter(|decl| matches!(decl, compiler::TopLevelDecl::Global { .. }))
            .count();
        text.push_str(&format!(
            "module\t{}\torder={}\troot_entry={}\tmain_symbol={}\tinit_symbol={}\timports={}\tfunctions={}\tstructs={}\tglobals={}\tinit_required=true\n",
            module.module,
            order,
            if root_entry { "true" } else { "false" },
            main_symbol,
            init_symbol,
            import_count,
            function_count,
            struct_count,
            global_count
        ));
    }

    for summary in module_call_summaries {
        text.push_str(&summary);
    }

    for call_edge in call_edges {
        text.push_str(&call_edge);
    }

    for fixup_candidate in fixup_candidates {
        text.push_str(&fixup_candidate);
    }

    for (init_order, module) in modules.iter().enumerate() {
        let init_symbol = stage1_unit_init_symbol(init_order);
        let global_count = module
            .declarations
            .iter()
            .filter(|decl| matches!(decl, compiler::TopLevelDecl::Global { .. }))
            .count();
        text.push_str(&format!(
            "init\t{}\t{}\tsymbol={}\tglobals={}\n",
            init_order,
            module.module,
            init_symbol,
            global_count
        ));
    }

    text
}

fn stage1_function_owner_map(
    modules: &[compiler::ParsedModule],
) -> std::collections::BTreeMap<String, Vec<(String, usize)>> {
    let mut owners = std::collections::BTreeMap::new();
    for (order, module) in modules.iter().enumerate() {
        for decl in &module.declarations {
            if let compiler::TopLevelDecl::Function { signature, .. } = decl {
                owners
                    .entry(signature.name.clone())
                    .or_insert_with(Vec::new)
                    .push((module.module.clone(), order));
            }
        }
    }
    owners
}

fn render_stage1_codegen_request(
    build_plan: &Stage1BuildPlan,
    modules: &[compiler::ParsedModule],
) -> String {
    let entry_module = build_plan.entry_module.clone();
    let entry_symbol = resolve_stage1_entry_symbol(&entry_module, modules);

    let mut text = String::new();
    text.push_str("target\tlinux\n");
    text.push_str("format\telf64_x86_64\n");
    text.push_str(&format!("output\t{}\n", build_plan.output_binary.display()));
    text.push_str(&format!("entry_module\t{}\n", entry_module));
    text.push_str(&format!("entry_symbol\t{}\n", entry_symbol));
    text.push_str(&format!("module_count\t{}\n", build_plan.module_count));
    text.push_str("combined_init_symbol\t__初始化全局__\n");
    text.push_str(&format!("manifest\t{}\n", build_plan.manifest_path.display()));
    text.push_str(&format!("ast_index\t{}\n", build_plan.ast_index_path.display()));
    text.push_str(&format!("symbol_index\t{}\n", build_plan.symbol_index_path.display()));
    text.push_str(&format!("link_plan\t{}\n", build_plan.link_plan_path.display()));
    text.push_str(&format!("codegen_units\t{}\n", build_plan.codegen_units_path.display()));
    text.push_str(&format!("symbol_merge_plan\t{}\n", build_plan.symbol_merge_plan_path.display()));
    text.push_str(&format!("runtime_abi_request\t{}\n", build_plan.runtime_abi_request_path.display()));
    text.push_str(&format!("emission_request\t{}\n", build_plan.emission_request_path.display()));
    text.push_str(&format!("start_stub_request\t{}\n", build_plan.start_stub_request_path.display()));
    text.push_str(&format!("start_stub_template\t{}\n", build_plan.start_stub_template_path.display()));
    text.push_str(&format!("runtime_blob\t{}\n", build_plan.runtime_blob_path.display()));
    text.push_str(&format!("runtime_blob_symbols\t{}\n", build_plan.runtime_blob_symbols_path.display()));
    text.push_str(&format!("module_objects_dir\t{}\n", build_plan.module_objects_dir.display()));
    text.push_str(&format!("module_object_index\t{}\n", build_plan.module_object_index_path.display()));
    text
}

fn render_stage1_codegen_units(
    build_plan: &Stage1BuildPlan,
    modules: &[compiler::ParsedModule],
) -> String {
    let entry_module = build_plan.entry_module.clone();
    let mut text = String::new();
    text.push_str("combined_init_symbol\t__初始化全局__\n");

    for (order, module) in modules.iter().enumerate() {
        let module_globals = stage1_module_global_names(module);
        let root_entry = module.module == entry_module;
        let main_symbol = stage1_unit_main_symbol(order, root_entry);
        let init_symbol = stage1_unit_init_symbol(order);
        let function_count = module
            .declarations
            .iter()
            .filter(|decl| matches!(decl, compiler::TopLevelDecl::Function { .. }))
            .count();
        let struct_count = module
            .declarations
            .iter()
            .filter(|decl| matches!(decl, compiler::TopLevelDecl::Struct { .. }))
            .count();
        let global_count = module
            .declarations
            .iter()
            .filter(|decl| matches!(decl, compiler::TopLevelDecl::Global { .. }))
            .count();
        text.push_str(&format!(
            "unit\t{}\torder={}\troot_entry={}\tmain_symbol={}\tinit_symbol={}\tfunctions={}\tstructs={}\tglobals={}\n",
            module.module,
            order,
            if root_entry { "true" } else { "false" },
            main_symbol,
            init_symbol,
            function_count,
            struct_count,
            global_count
        ));

        let mut function_order = 0usize;
        for decl in &module.declarations {
            if let compiler::TopLevelDecl::Function {
                signature,
                token_start,
                token_end,
                body_token_span,
                body_statements,
            } = decl
            {
                let params = if signature.params.is_empty() {
                    "none".to_string()
                } else {
                    signature.params.join(",")
                };
                let body_tokens = body_token_span
                    .as_ref()
                    .map(|(start, end)| format!("{}..{}", start, end))
                    .unwrap_or_else(|| "none".to_string());
                let first_stmt = body_statements
                    .first()
                    .map(|stmt| stmt.kind)
                    .unwrap_or("none");
                let first_stmt_lines = body_statements
                    .first()
                    .map(|stmt| format!("{}..{}", stmt.line_start, stmt.line_end))
                    .unwrap_or_else(|| "none".to_string());
                let call_targets = stage1_lowering_call_targets(&module_globals, &signature.params, body_statements);
                let calls = if call_targets.is_empty() {
                    "none".to_string()
                } else {
                    call_targets.join(",")
                };

                text.push_str(&format!(
                    "lowering\t{}\tunit_order={}\tfunc_order={}\tdecl={}\tfinal_symbol={}\tparams={}\tparam_count={}\ttoken_span={}..{}\tbody_tokens={}\tbody_stmts={}\tfirst_stmt={}\tfirst_stmt_lines={}\tcall_count={}\tcalls={}\n",
                    module.module,
                    order,
                    function_order,
                    signature.name,
                    signature.name,
                    params,
                    signature.params.len(),
                    token_start,
                    token_end,
                    body_tokens,
                    body_statements.len(),
                    first_stmt,
                    first_stmt_lines,
                    call_targets.len(),
                    calls
                ));
                function_order += 1;
            }
        }
    }

    text
}

fn stage1_module_global_names(
    module: &compiler::ParsedModule,
) -> std::collections::BTreeSet<String> {
    let mut names = std::collections::BTreeSet::new();
    for decl in &module.declarations {
        if let compiler::TopLevelDecl::Global { signature, .. } = decl {
            names.insert(signature.name.clone());
        }
    }
    names
}

fn stage1_lowering_call_targets(
    module_globals: &std::collections::BTreeSet<String>,
    params: &[String],
    body_statements: &[compiler::BodyStmtSummary],
) -> Vec<String> {
    let mut calls = std::collections::BTreeSet::new();
    let mut bound_names = module_globals.clone();
    for param in params {
        bound_names.insert(param.clone());
    }
    collect_calls_from_body_statements(body_statements, &mut bound_names, &mut calls);
    calls.into_iter().collect()
}

fn collect_calls_from_body_statements(
    body_statements: &[compiler::BodyStmtSummary],
    bound_names: &mut std::collections::BTreeSet<String>,
    calls: &mut std::collections::BTreeSet<String>,
) {
    for stmt in body_statements {
        collect_calls_from_body_stmt(stmt, bound_names, calls);
    }
}

fn collect_calls_from_body_stmt(
    stmt: &compiler::BodyStmtSummary,
    bound_names: &mut std::collections::BTreeSet<String>,
    calls: &mut std::collections::BTreeSet<String>,
) {
    match &stmt.ast {
        compiler::BodyStmtAst::Define { name, value, .. } => {
            if let Some(expr) = value {
                collect_calls_from_expr(expr, bound_names, calls);
            }
            if let Some(name) = name {
                bound_names.insert(name.clone());
            }
        }
        compiler::BodyStmtAst::Return { value }
        | compiler::BodyStmtAst::Throw { value }
        | compiler::BodyStmtAst::Expr { value } => {
            if let Some(expr) = value {
                collect_calls_from_expr(expr, bound_names, calls);
            }
        }
        compiler::BodyStmtAst::If {
            condition,
            then_body,
            else_body,
        } => {
            if let Some(expr) = condition {
                collect_calls_from_expr(expr, bound_names, calls);
            }
            let mut then_bound = bound_names.clone();
            collect_calls_from_body_statements(then_body, &mut then_bound, calls);
            let mut else_bound = bound_names.clone();
            collect_calls_from_body_statements(else_body, &mut else_bound, calls);
            bound_names.extend(then_bound);
            bound_names.extend(else_bound);
        }
        compiler::BodyStmtAst::While { condition, body } => {
            if let Some(expr) = condition {
                collect_calls_from_expr(expr, bound_names, calls);
            }
            let mut loop_bound = bound_names.clone();
            collect_calls_from_body_statements(body, &mut loop_bound, calls);
            bound_names.extend(loop_bound);
        }
        compiler::BodyStmtAst::Assign { target, value, .. } => {
            if let Some(expr) = target {
                collect_calls_from_expr(expr, bound_names, calls);
            }
            if let Some(expr) = value {
                collect_calls_from_expr(expr, bound_names, calls);
            }
            if let Some(name) = parsed_expr_identifier_name(target.as_ref()) {
                bound_names.insert(name);
            }
        }
        compiler::BodyStmtAst::Break | compiler::BodyStmtAst::Continue | compiler::BodyStmtAst::Try => {}
    }
}

fn parsed_expr_identifier_name(expr: Option<&compiler::ParsedExpr>) -> Option<String> {
    match expr.map(|expr| &expr.tree) {
        Some(compiler::ExprTree::Identifier(name)) => Some(name.clone()),
        _ => None,
    }
}

fn collect_calls_from_expr(
    expr: &compiler::ParsedExpr,
    bound_names: &std::collections::BTreeSet<String>,
    calls: &mut std::collections::BTreeSet<String>,
) {
    match &expr.tree {
        compiler::ExprTree::Literal
        | compiler::ExprTree::Identifier(_)
        | compiler::ExprTree::Unknown => {}
        compiler::ExprTree::Unary { expr, .. } => collect_calls_from_expr(expr, bound_names, calls),
        compiler::ExprTree::Binary { left, right, .. } => {
            collect_calls_from_expr(left, bound_names, calls);
            collect_calls_from_expr(right, bound_names, calls);
        }
        compiler::ExprTree::Call { callee, args } => {
            if let compiler::ExprTree::Identifier(name) = &callee.tree {
                if !bound_names.contains(name) {
                    calls.insert(name.clone());
                }
            }
            collect_calls_from_expr(callee, bound_names, calls);
            for arg in args {
                collect_calls_from_expr(arg, bound_names, calls);
            }
        }
        compiler::ExprTree::Index { target, index } => {
            collect_calls_from_expr(target, bound_names, calls);
            collect_calls_from_expr(index, bound_names, calls);
        }
        compiler::ExprTree::Member { target, .. } => collect_calls_from_expr(target, bound_names, calls),
        compiler::ExprTree::List { items } => {
            for item in items {
                collect_calls_from_expr(item, bound_names, calls);
            }
        }
        compiler::ExprTree::Struct { fields, .. } => {
            for field in fields {
                collect_calls_from_expr(&field.value, bound_names, calls);
            }
        }
    }
}

fn render_stage1_symbol_merge_plan(
    build_plan: &Stage1BuildPlan,
    modules: &[compiler::ParsedModule],
) -> String {
    let entry_module = build_plan.entry_module.clone();
    let entry_symbol = resolve_stage1_entry_symbol(&entry_module, modules);

    let mut text = String::new();
    text.push_str(&format!("entry_symbol\t{}\n", entry_symbol));
    text.push_str("combined_init_symbol\t__初始化全局__\n");

    let mut global_slot = 0usize;
    for (order, module) in modules.iter().enumerate() {
        let root_entry = module.module == entry_module;
        let main_symbol = stage1_unit_main_symbol(order, root_entry);
        let init_symbol = stage1_unit_init_symbol(order);
        text.push_str(&format!(
            "module\t{}\tmain_symbol={}\tinit_symbol={}\n",
            module.module, main_symbol, init_symbol
        ));

        for decl in &module.declarations {
            match decl {
                compiler::TopLevelDecl::Function { signature, .. } => {
                    text.push_str(&format!(
                        "function\t{}\tdecl={}\tfinal_symbol={}\n",
                        module.module,
                        signature.name,
                        signature.name
                    ));
                }
                compiler::TopLevelDecl::Global { signature, .. } => {
                    text.push_str(&format!(
                        "global\t{}\tdecl={}\tslot={}\tmutable={}\n",
                        module.module,
                        signature.name,
                        global_slot,
                        if signature.mutable { "true" } else { "false" }
                    ));
                    global_slot += 1;
                }
                _ => {}
            }
        }
    }

    text.push_str(&format!("global_slots\t{}\n", global_slot));
    text
}

fn stage1_unit_main_symbol(order: usize, root_entry: bool) -> String {
    if root_entry {
        "__主程序__".to_string()
    } else {
        format!("__主程序___{}", order)
    }
}

fn stage1_unit_init_symbol(order: usize) -> String {
    format!("__初始化全局___{}", order)
}

fn stage1_entry_symbol_contract() -> &'static str {
    "none"
}

fn resolve_stage1_entry_symbol(
    _entry_module: &str,
    _modules: &[compiler::ParsedModule],
) -> String {
    stage1_entry_symbol_contract().to_string()
}

fn render_stage1_runtime_abi_request(runtime_symbols: &[String]) -> String {
    let mut text = String::new();
    text.push_str("target\tlinux\n");
    text.push_str("format\telf64_x86_64\n");
    text.push_str("calling_convention\tnova_internal_rcx_rdx_r8_r9\n");
    text.push_str("stack_alignment\t16\n");
    text.push_str("heap_register\tr14\n");
    text.push_str("argv_register\tr13\n");
    text.push_str(&format!("runtime_symbol_count\t{}\n", runtime_symbols.len()));
    for symbol in runtime_symbols {
        text.push_str(&format!("runtime_symbol\t{}\n", symbol));
    }
    text
}

fn render_stage1_emission_request(
    build_plan: &Stage1BuildPlan,
    modules: &[compiler::ParsedModule],
    runtime_symbols: &[String],
    link_runtime_seed_symbol_count: usize,
    link_runtime_seed_edge_count: usize,
    link_call_edge_count: usize,
    link_unresolved_fixups: usize,
    runtime_missing_count: usize,
    module_object_index: &Stage1ModuleObjectIndex,
) -> String {
    let entry_module = build_plan.entry_module.clone();
    let entry_symbol = resolve_stage1_entry_symbol(&entry_module, modules);

    let mut text = String::new();
    text.push_str("target\tlinux\n");
    text.push_str("format\telf64_x86_64\n");
    text.push_str(&format!("output\t{}\n", build_plan.output_binary.display()));
    text.push_str(&format!("entry_module\t{}\n", entry_module));
    text.push_str(&format!("entry_symbol\t{}\n", entry_symbol));
    text.push_str("combined_init_symbol\t__初始化全局__\n");
    text.push_str(&format!("symbol_merge_plan\t{}\n", build_plan.symbol_merge_plan_path.display()));
    text.push_str(&format!("codegen_units\t{}\n", build_plan.codegen_units_path.display()));
    text.push_str(&format!("runtime_abi_request\t{}\n", build_plan.runtime_abi_request_path.display()));
    text.push_str(&format!("runtime_coverage_report\t{}\n", build_plan.runtime_coverage_report_path.display()));
    text.push_str(&format!("start_stub_request\t{}\n", build_plan.start_stub_request_path.display()));
    text.push_str(&format!("start_stub_template\t{}\n", build_plan.start_stub_template_path.display()));
    text.push_str(&format!("runtime_blob\t{}\n", build_plan.runtime_blob_path.display()));
    text.push_str(&format!("runtime_blob_symbols\t{}\n", build_plan.runtime_blob_symbols_path.display()));
    text.push_str(&format!("module_objects_dir\t{}\n", build_plan.module_objects_dir.display()));
    text.push_str(&format!("module_object_index\t{}\n", build_plan.module_object_index_path.display()));
    text.push_str(&format!("runtime_symbol_count\t{}\n", runtime_symbols.len()));
    text.push_str(&format!("link_runtime_seed_symbol_count\t{}\n", link_runtime_seed_symbol_count));
    text.push_str(&format!("link_runtime_seed_edge_count\t{}\n", link_runtime_seed_edge_count));
    text.push_str(&format!("link_call_edge_count\t{}\n", link_call_edge_count));
    text.push_str(&format!("link_unresolved_fixups\t{}\n", link_unresolved_fixups));
    text.push_str(&format!("runtime_missing_count\t{}\n", runtime_missing_count));
    text.push_str(&format!("module_object_count\t{}\n", module_object_index.object_count));
    text.push_str(&format!("module_object_total_code_size\t{}\n", module_object_index.total_code_size));
    text.push_str("backend_phase\tmodule_lowering\tstatus=ready\tinput=stage1_codegen_units\n");
    text.push_str("backend_phase\tsymbol_layout\tstatus=ready\tinput=stage1_symbol_merge_plan\n");
    text.push_str("backend_phase\tcall_resolution_plan\tstatus=ready\tinput=stage1_link_plan\n");
    text.push_str("backend_phase\truntime_abi_contract\tstatus=ready\tinput=stage1_runtime_abi_request\n");
    text.push_str("backend_phase\tstart_stub_contract\tstatus=ready\tinput=stage1_start_stub_request\n");
    text.push_str("backend_phase\tstart_stub_template\tstatus=ready\tinput=stage1_start_stub_template.bin\n");
    text.push_str("backend_phase\truntime_blob_machinecode\tstatus=ready\tinput=stage1_runtime_blob.bin|stage1_runtime_blob_symbols.txt\n");
    text.push_str("backend_phase\tmodule_object_codegen\tstatus=ready\tinput=module_objects_dir|stage1_module_object_index.txt\n");
    text.push_str("backend_phase\tstart_stub_patch\tstatus=ready\tinput=stage1_start_stub_template.bin|stage1_start_stub_request|stage1_symbol_merge_plan\n");
    text.push_str("backend_phase\tcall_patch_after_runtime\tstatus=ready\tinput=stage1_link_plan|stage1_symbol_merge_plan\n");
    text.push_str("backend_phase\telf64_writer\tstatus=ready\tinput=merged_text|entry_symbol\n");
    text
}

fn stage1_start_stub_template_bytes() -> Vec<u8> {
    vec![
        0x49u8, 0x89, 0xE5, 0xE8, 0x00, 0x00, 0x00, 0x00, 0x49, 0x89, 0xC6, 0xE8, 0x00,
        0x00, 0x00, 0x00, 0xE8, 0x00, 0x00, 0x00, 0x00, 0x48, 0x89, 0xC7, 0x48, 0xC7,
        0xC0, 0x3C, 0x00, 0x00, 0x00, 0x0F, 0x05,
    ]
}

fn render_stage1_start_stub_request(
    build_plan: &Stage1BuildPlan,
    modules: &[compiler::ParsedModule],
    link_unresolved_fixups: usize,
    runtime_missing_count: usize,
) -> String {
    let entry_module = build_plan.entry_module.clone();
    let entry_symbol = resolve_stage1_entry_symbol(&entry_module, modules);
    let module_init_symbols = (0..modules.len())
        .map(stage1_unit_init_symbol)
        .collect::<Vec<_>>();
    let template_bytes = stage1_start_stub_template_bytes();
    let template_hex = template_bytes
        .iter()
        .map(|byte| format!("{:02X}", byte))
        .collect::<Vec<_>>()
        .join(" ");

    let mut text = String::new();
    text.push_str("target\tlinux\n");
    text.push_str("format\tstart_stub_contract_v1\n");
    text.push_str("stub_symbol\t_start\n");
    text.push_str(&format!("entry_module\t{}\n", entry_module));
    text.push_str(&format!("entry_symbol\t{}\n", entry_symbol));
    text.push_str("runtime_heap_init_symbol\t__运行时_堆初始化\n");
    text.push_str("combined_init_symbol\t__初始化全局__\n");
    text.push_str("heap_register\tr14\n");
    text.push_str("argv_register\tr13\n");
    text.push_str("exit_strategy\tlinux_syscall_60\n");
    text.push_str(&format!("template_size\t{}\n", template_bytes.len()));
    text.push_str(&format!("template_hex\t{}\n", template_hex));
    text.push_str(&format!("module_init_count\t{}\n", module_init_symbols.len()));
    text.push_str(&format!("module_init_symbols\t{}\n", module_init_symbols.join(",")));
    text.push_str(&format!("link_unresolved_fixups\t{}\n", link_unresolved_fixups));
    text.push_str(&format!("runtime_missing_count\t{}\n", runtime_missing_count));
    text.push_str("start_step\t0\tpreserve_argv_to_r13\n");
    text.push_str("start_step\t1\tcall __运行时_堆初始化 and store heap base in r14\n");
    text.push_str("start_step\t2\tcall __初始化全局__\n");
    text.push_str(&format!("start_step\t3\tcall {}\n", entry_symbol));
    text.push_str("start_step\t4\texit via syscall 60 with return value in rdi\n");
    text.push_str("call_fixup\torder=0\tcall_opcode_offset=3\tdisp_offset=4\tsymbol=__运行时_堆初始化\tkind=call_rel32\trequired=true\n");
    text.push_str("call_fixup\torder=1\tcall_opcode_offset=11\tdisp_offset=12\tsymbol=__初始化全局__\tkind=call_rel32\trequired=true\n");
    text.push_str(&format!(
        "call_fixup\torder=2\tcall_opcode_offset=16\tdisp_offset=17\tsymbol={}\tkind=call_rel32\trequired={}\n",
        entry_symbol,
        if entry_symbol == stage1_entry_symbol_contract() { "false" } else { "true" }
    ));
    text
}

fn render_stage1_runtime_coverage_report(
    runtime_symbols: &[String],
    seed_runtime_symbols: &[String],
    declared_runtime_symbols: &[String],
) -> String {
    let requested = runtime_symbols.iter().cloned().collect::<std::collections::BTreeSet<_>>();
    let provided_seed = seed_runtime_symbols
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    let declared_source = declared_runtime_symbols
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    let available = provided_seed
        .union(&declared_source)
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();

    let covered = requested
        .intersection(&available)
        .cloned()
        .collect::<Vec<_>>();
    let covered_by_seed = requested
        .intersection(&provided_seed)
        .cloned()
        .collect::<Vec<_>>();
    let covered_by_source = requested
        .difference(&provided_seed)
        .filter(|symbol| declared_source.contains(*symbol))
        .cloned()
        .collect::<Vec<_>>();
    let missing = requested
        .difference(&available)
        .cloned()
        .collect::<Vec<_>>();
    let extra_seed = provided_seed
        .difference(&requested)
        .cloned()
        .collect::<Vec<_>>();
    let extra_source = declared_source
        .difference(&requested)
        .cloned()
        .collect::<Vec<_>>();

    let mut text = String::new();
    text.push_str(&format!("requested\t{}\n", requested.len()));
    text.push_str(&format!("provided_seed\t{}\n", provided_seed.len()));
    text.push_str(&format!("declared_source\t{}\n", declared_source.len()));
    text.push_str(&format!("covered\t{}\n", covered.len()));
    text.push_str(&format!("covered_by_seed\t{}\n", covered_by_seed.len()));
    text.push_str(&format!("covered_by_source\t{}\n", covered_by_source.len()));
    text.push_str(&format!("missing\t{}\n", missing.len()));
    text.push_str(&format!("extra_seed\t{}\n", extra_seed.len()));
    text.push_str(&format!("extra_source\t{}\n", extra_source.len()));

    for symbol in covered {
        text.push_str(&format!("covered_symbol\t{}\n", symbol));
    }
    for symbol in covered_by_source {
        text.push_str(&format!("source_symbol\t{}\n", symbol));
    }
    for symbol in missing {
        text.push_str(&format!("missing_symbol\t{}\n", symbol));
    }
    for symbol in extra_seed {
        text.push_str(&format!("extra_seed_symbol\t{}\n", symbol));
    }
    for symbol in extra_source {
        text.push_str(&format!("extra_source_symbol\t{}\n", symbol));
    }

    text
}

pub fn materialize_stage0(
    project: &KernelProject,
    artifacts: &StageArtifacts,
) -> Result<MaterializeStats, String> {
    if artifacts.stage0_root.exists() {
        fs::remove_dir_all(&artifacts.stage0_root).map_err(|e| {
            format!(
                "清理旧阶段0目录失败: {} ({})",
                artifacts.stage0_root.display(),
                e
            )
        })?;
    }

    fs::create_dir_all(&artifacts.stage0_root).map_err(|e| {
        format!(
            "创建阶段0目录失败: {} ({})",
            artifacts.stage0_root.display(),
            e
        )
    })?;
    fs::create_dir_all(&artifacts.output_root).map_err(|e| {
        format!(
            "创建阶段输出目录失败: {} ({})",
            artifacts.output_root.display(),
            e
        )
    })?;
    fs::create_dir_all(&artifacts.stage0_cache_dir()).map_err(|e| {
        format!(
            "创建阶段0缓存目录失败: {} ({})",
            artifacts.stage0_cache_dir().display(),
            e
        )
    })?;

    for relative in &project.local_sources {
        let source = project.kernel_root.join(relative);
        let target = artifacts.stage0_root.join(relative);
        copy_file(&source, &target)?;
    }

    let runtime_seed_target = artifacts.stage0_root.join(LOCAL_RUNTIME_SEED_REL);
    copy_file(&project.runtime_seed_source, &runtime_seed_target)?;

    let manifest_text = project.manifest_text();
    for name in [
        "_manifest.txt",
        "nova_manifest.txt",
        "nova_files_tmp.txt",
        ".__nova_files_tmp.txt",
    ] {
        fs::write(artifacts.stage0_root.join(name), &manifest_text).map_err(|e| {
            format!(
                "写入阶段0清单失败: {} ({})",
                artifacts.stage0_root.join(name).display(),
                e
            )
        })?;
    }

    localize_frontend_entry(&artifacts.stage0_root.join(FRONTEND_ENTRY_REL))?;
    let authority_path = artifacts.stage0_root.join("stage0_authority.txt");
    let authority_text = format!(
        "kernel_root\t{}\nmanifest_source\t{}\nruntime_seed_source\t{}\nstage0_root\t{}\n",
        project.kernel_root.display(),
        project.manifest_source.display(),
        project.runtime_seed_source.display(),
        artifacts.stage0_root.display()
    );
    fs::write(&authority_path, authority_text).map_err(|e| {
        format!(
            "写入阶段0权威源记录失败: {} ({})",
            authority_path.display(),
            e
        )
    })?;

    Ok(MaterializeStats {
        source_count: project.localized_manifest_lines.len(),
        manifest_path: artifacts.stage0_root.join("_manifest.txt"),
        runtime_seed_path: runtime_seed_target,
        authority_path,
    })
}

fn copy_file(source: &Path, target: &Path) -> Result<(), String> {
    let parent = target.parent().ok_or_else(|| {
        format!("无法确定目标父目录: {}", target.display())
    })?;
    fs::create_dir_all(parent).map_err(|e| {
        format!("创建目录失败: {} ({})", parent.display(), e)
    })?;
    fs::copy(source, target).map_err(|e| {
        format!(
            "复制文件失败: {} -> {} ({})",
            source.display(),
            target.display(),
            e
        )
    })?;
    Ok(())
}

fn rewrite_unique_trimmed_line<F>(
    source: &str,
    replacement_trimmed: &str,
    matcher: F,
) -> (String, usize)
where
    F: Fn(&str) -> bool,
{
    let mut hits = 0usize;
    let had_trailing_newline = source.ends_with('\n');
    let mut lines = Vec::new();

    for line in source.lines() {
        let trimmed = line.trim();
        if matcher(trimmed) {
            let indent_len = line.len() - line.trim_start().len();
            let indent = &line[..indent_len];
            lines.push(format!("{}{}", indent, replacement_trimmed));
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

fn localize_frontend_entry(path: &Path) -> Result<(), String> {
    let source = fs::read_to_string(path)
        .map_err(|e| format!("读取前端入口失败: {} ({})", path.display(), e))?;
    let local_import = format!("导入(\"{}\")", LOCAL_RUNTIME_SEED_IMPORT_REL);
    let external_import = format!("导入(\"{}\")", EXTERNAL_RUNTIME_SEED_IMPORT_REL);
    let (updated, hits) = rewrite_unique_trimmed_line(&source, &local_import, |line| {
        line == external_import.as_str() || line == local_import.as_str()
    });

    if hits > 1 {
        return Err(format!(
            "前端入口存在多处运行时种子导入 ({}处), 无法安全本地化: {}",
            hits,
            path.display()
        ));
    }
    if hits == 0 {
        return Ok(());
    }

    fs::write(path, updated)
        .map_err(|e| format!("写回前端入口失败: {} ({})", path.display(), e))?;
    Ok(())
}
