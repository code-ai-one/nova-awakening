#[path = "backend_seed.rs"]
mod backend_seed;
mod arm64_codegen;
mod build_system;
mod cache_manager;
mod constant_pool;
mod compiler;
mod debug_info;
mod diagnostics;
mod disassembler;
mod elf;
mod elf_reader;
mod graph_coloring;
mod inline_heuristic;
mod ir_pass_manager;
mod ir_serializer;
mod linker;
mod liveness_bits;
mod loop_optimizer;
mod module_graph_viz;
mod object_writer;
mod simd_helper;
mod peephole_patterns;
mod profile_reader;
mod register_allocator;
mod riscv_codegen;
mod ssa_builder;
mod stack_frame;
mod string_interner;
mod symbol_table;
mod test_runner;
mod type_checker;
mod wasm_backend;
mod source_reset;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

struct CliArgs {
    kernel_root_override: Option<PathBuf>,
    sync_runtime_seed: bool,
    stage1_binary: Option<PathBuf>,
    // 6.4+6.5: 新命令
    check_keyword_contract: bool,
    smoke_test: bool,
    emit_bill_of_materials: bool,
    source_reset_path: Option<String>,
}

const MAX_CONVERGENCE_ROUNDS: usize = 4;

#[allow(dead_code)]
fn run_source_reset_if_requested(args: &CliArgs) -> bool {
    if let Some(ref path) = args.source_reset_path {
        let report = source_reset::run_source_reset(path);
        report.print_summary();
        return true;
    }
    false
}

struct StageDriveStats {
    stage2_exit: i32,
    stage3_exit: i32,
    fixed_point: bool,
    convergence_rounds: usize,
    diff_bytes_last: usize,
    // 6.2: 全流程工料单扩展
    stage2_size: usize,
    stage3_size: usize,
    module_count: usize,
    function_count: usize,
    profile: String,
    target: String,
    entry_symbol: String,
    image_format: String,
    semantic_warnings: usize,
    semantic_errors: usize,
    optimization_passes: usize,
    link_fixups_total: usize,
    link_fixups_resolved: usize,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("错误: {}", err);
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    // 增大栈限制：Stage2编译器深递归需要>8MB栈
    unsafe {
        let mut cur = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
        libc::getrlimit(libc::RLIMIT_STACK, &mut cur);
        eprintln!("[RLIMIT] before: cur={}MB max={}", cur.rlim_cur / 1024 / 1024,
            if cur.rlim_max == libc::RLIM_INFINITY { "INFINITY".to_string() } else { format!("{}MB", cur.rlim_max / 1024 / 1024) });
        let want: u64 = 64 * 1024 * 1024;
        let new_cur = if cur.rlim_max == libc::RLIM_INFINITY || cur.rlim_max >= want { want } else { cur.rlim_max };
        let rlim = libc::rlimit { rlim_cur: new_cur, rlim_max: cur.rlim_max };
        let rc = libc::setrlimit(libc::RLIMIT_STACK, &rlim);
        libc::getrlimit(libc::RLIMIT_STACK, &mut cur);
        eprintln!("[RLIMIT] after: rc={} cur={}MB", rc, cur.rlim_cur / 1024 / 1024);
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cli = parse_cli_args(&args)?;

    let cargo_manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = compiler::discover_workspace_root(&cargo_manifest_dir)?;
    if cli.sync_runtime_seed {
        let report = compiler::sync_runtime_seed_to_authority(
            workspace_root.clone(),
            cli.kernel_root_override.clone(),
        )?;
        println!("runtime_seed_sync_kernel_root: {}", report.kernel_root.display());
        println!("runtime_seed_sync_source: {}", report.runtime_seed_source.display());
        println!("runtime_seed_sync_target: {}", report.runtime_seed_target.display());
        println!("runtime_seed_sync_frontend_entry: {}", report.frontend_entry_path.display());
        for manifest_path in report.manifest_paths {
            println!("runtime_seed_sync_manifest: {}", manifest_path.display());
        }
        return Ok(());
    }
    // ── CLI功能: 关键字契约检查 ──
    if cli.check_keyword_contract {
        return run_keyword_contract_check();
    }

    let project = compiler::load_kernel_project(workspace_root.clone(), cli.kernel_root_override.clone())?;
    let artifacts = elf::StageArtifacts::new(workspace_root.clone());
    let stats = linker::materialize_stage0(&project, &artifacts)?;
    let stage1_project = compiler::load_kernel_project(
        workspace_root.clone(),
        Some(artifacts.stage0_root.clone()),
    )?;

    println!("═══════════════════════════════════════════");
    println!("  中文原生内核 · Rust辅助自举体系");
    println!("  当前阶段: 物化本地 Stage-0 工作区");
    println!("═══════════════════════════════════════════");
    println!("工作区: {}", workspace_root.display());
    println!("内核根: {}", project.kernel_root.display());
    println!("清单源: {}", project.manifest_source.display());
    println!("阶段0文件数: {}", stats.source_count);
    println!("阶段0清单: {}", stats.manifest_path.display());
    println!("本地运行时种子: {}", stats.runtime_seed_path.display());
    println!("阶段0权威源记录: {}", stats.authority_path.display());
    println!("stage1_source_root: {}", stage1_project.kernel_root.display());
    for (name, value) in artifacts.describe() {
        println!("{}: {}", name, value);
    }
    println!("stage1_exists: {}", if artifacts.stage1_binary.is_file() { "true" } else { "false" });
    println!("stage2_exists: {}", if artifacts.stage2_binary.is_file() { "true" } else { "false" });
    println!("stage3_exists: {}", if artifacts.stage3_binary.is_file() { "true" } else { "false" });
    let stage1_plan = linker::plan_stage1_module_collection(&stage1_project)?;
    let stage1_build_plan = linker::materialize_stage1_build_inputs(&stage1_project, &artifacts)?;
    println!("stage1_plan_modules: {}", stage1_plan.module_count);
    println!("stage1_plan_entry: {}", stage1_plan.entry_module);
    println!("stage1_build_root: {}", stage1_build_plan.build_root.display());
    println!("stage1_build_output: {}", stage1_build_plan.output_binary.display());
    println!("stage1_build_manifest: {}", stage1_build_plan.manifest_path.display());
    println!("stage1_build_entry: {}", stage1_build_plan.entry_path.display());
    println!("stage1_build_ast_index: {}", stage1_build_plan.ast_index_path.display());
    println!("stage1_build_symbol_index: {}", stage1_build_plan.symbol_index_path.display());
    println!("stage1_build_link_plan: {}", stage1_build_plan.link_plan_path.display());
    println!("stage1_build_codegen_request: {}", stage1_build_plan.codegen_request_path.display());
    println!("stage1_build_codegen_units: {}", stage1_build_plan.codegen_units_path.display());
    println!("stage1_build_symbol_merge_plan: {}", stage1_build_plan.symbol_merge_plan_path.display());
    println!("stage1_build_runtime_abi_request: {}", stage1_build_plan.runtime_abi_request_path.display());
    println!("stage1_build_emission_request: {}", stage1_build_plan.emission_request_path.display());
    println!("stage1_build_start_stub_request: {}", stage1_build_plan.start_stub_request_path.display());
    println!("stage1_build_start_stub_template: {}", stage1_build_plan.start_stub_template_path.display());
    println!("stage1_build_runtime_blob: {}", stage1_build_plan.runtime_blob_path.display());
    println!("stage1_build_runtime_blob_symbols: {}", stage1_build_plan.runtime_blob_symbols_path.display());
    println!("stage1_build_module_objects_dir: {}", stage1_build_plan.module_objects_dir.display());
    println!("stage1_build_module_object_index: {}", stage1_build_plan.module_object_index_path.display());
    println!("stage1_build_isolation_report: {}", stage1_build_plan.isolation_report_path.display());
    println!("stage1_build_runtime_coverage_report: {}", stage1_build_plan.runtime_coverage_report_path.display());
    linker::materialize_isolation_report(&project, &artifacts, &stage1_build_plan)?;
    let requirements = compiler::current_kernel_requirements(&stage1_project);
    if !requirements.is_empty() {
        println!("第一批必须支持的当前内核模块能力:");
        for item in requirements {
            println!("- {}", item.module);
            for reason in item.reasons {
                println!("  - {}", reason);
            }
        }
    }
    let scan_summaries = compiler::scan_required_modules(&stage1_project)?;
    if !scan_summaries.is_empty() {
        println!("前端扫描摘要:");
        for item in scan_summaries {
            println!(
                "- {} => tokens={}, keywords={}",
                item.module,
                item.token_count,
                item.keyword_count
            );
        }
    }
    let skeletons = compiler::parse_required_module_skeletons(&stage1_project)?;
    if !skeletons.is_empty() {
        println!("顶层骨架摘要:");
        for item in skeletons {
            let total_params: usize = item.functions.iter().map(|entry| entry.params.len()).sum();
            let total_fields: usize = item.structs.iter().map(|entry| entry.fields.len()).sum();
            let mutable_globals = item.globals.iter().filter(|entry| entry.mutable).count();
            println!(
                "- {} => imports={}, functions={} (params={}), structs={} (fields={}), globals={} (mutable={})",
                item.module,
                item.imports.len(),
                item.functions.len(),
                total_params,
                item.structs.len(),
                total_fields,
                item.globals.len(),
                mutable_globals
            );
            if let Some(first_function) = item.functions.first() {
                println!("  - first_fn: {}({})", first_function.name, first_function.params.join(", "));
            }
            if let Some(first_struct) = item.structs.first() {
                println!("  - first_struct: {}{{{}}}", first_struct.name, first_struct.fields.join(", "));
            }
            if let Some(first_global) = item.globals.first() {
                println!(
                    "  - first_global: {}{}",
                    if first_global.mutable { "可变 " } else { "" },
                    first_global.name
                );
            }
        }
    }

    let parsed_modules = compiler::parse_required_modules(&project)?;
    if !parsed_modules.is_empty() {
        println!("顶层 AST 声明摘要:");
        for module in parsed_modules {
            let mut expr_counts: BTreeMap<&'static str, usize> = BTreeMap::new();
            let mut total_exprs = 0usize;
            let mut total_nodes = 0usize;
            let mut unknown_nodes = 0usize;
            let mut max_expr_depth = 0usize;
            let mut first_unknown_sample: Option<String> = None;
            for decl in &module.declarations {
                if let compiler::TopLevelDecl::Function { body_statements, .. } = decl {
                    for stmt in body_statements {
                        if let Some(expr) = &stmt.target_expr {
                            *expr_counts.entry(expr.kind).or_insert(0) += 1;
                            total_exprs += 1;
                            total_nodes += expr.node_count();
                            unknown_nodes += expr.unknown_nodes();
                            max_expr_depth = max_expr_depth.max(expr.max_depth());
                            if first_unknown_sample.is_none() && expr.unknown_nodes() > 0 {
                                first_unknown_sample = Some(format!(
                                    "target {}@{}..{} lines={}..{} -> {}@{}..{}",
                                    stmt.kind,
                                    stmt.token_start,
                                    stmt.token_end,
                                    stmt.line_start,
                                    stmt.line_end,
                                    expr.kind,
                                    expr.token_start,
                                    expr.token_end
                                ));
                            }
                        }
                        if let Some(expr) = &stmt.primary_expr {
                            *expr_counts.entry(expr.kind).or_insert(0) += 1;
                            total_exprs += 1;
                            total_nodes += expr.node_count();
                            unknown_nodes += expr.unknown_nodes();
                            max_expr_depth = max_expr_depth.max(expr.max_depth());
                            if first_unknown_sample.is_none() && expr.unknown_nodes() > 0 {
                                first_unknown_sample = Some(format!(
                                    "expr {}@{}..{} lines={}..{} -> {}@{}..{}",
                                    stmt.kind,
                                    stmt.token_start,
                                    stmt.token_end,
                                    stmt.line_start,
                                    stmt.line_end,
                                    expr.kind,
                                    expr.token_start,
                                    expr.token_end
                                ));
                            }
                        }
                    }
                }
            }
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
            println!(
                "- {} => imports={}, functions={}, structs={}, globals={}, decls={}",
                module.module,
                import_count,
                function_count,
                struct_count,
                global_count,
                module.declarations.len()
            );
            if let Some(first_fn) = module.declarations.iter().find_map(|decl| {
                match decl {
                    compiler::TopLevelDecl::Function {
                        signature,
                        body_token_span,
                        body_statements,
                        ..
                    } => Some((signature, body_token_span, body_statements)),
                    _ => None,
                }
            }) {
                let body_text = match first_fn.1 {
                    Some((start, end)) => format!("{}..{}", start, end),
                    None => "none".to_string(),
                };
                let first_stmt = first_fn
                    .2
                    .first()
                    .map(|stmt| {
                        let target = stmt
                            .target_expr
                            .as_ref()
                            .map(|expr| format!("{}@{}..{}", expr.kind, expr.token_start, expr.token_end))
                            .unwrap_or_else(|| "none".to_string());
                        let expr = stmt
                            .primary_expr
                            .as_ref()
                            .map(|expr| format!("{}@{}..{}", expr.kind, expr.token_start, expr.token_end))
                            .unwrap_or_else(|| "none".to_string());
                        format!(
                            "{}@{}..{} lines={}..{} ast={} target={} expr={}",
                            stmt.kind,
                            stmt.token_start,
                            stmt.token_end,
                            stmt.line_start,
                            stmt.line_end,
                            stmt.ast.tag(),
                            target,
                            expr
                        )
                    })
                    .unwrap_or_else(|| "none".to_string());
                println!(
                    "  - first_decl_fn: {}({}) body_tokens={} body_stmts={} first_stmt={}",
                    first_fn.0.name,
                    first_fn.0.params.join(", "),
                    body_text,
                    first_fn.2.len(),
                    first_stmt
                );
            }
            if total_exprs > 0 {
                println!(
                    "  - expr_tree: exprs={} nodes={} resolved_nodes={} unknown_nodes={} max_depth={}",
                    total_exprs,
                    total_nodes,
                    total_nodes.saturating_sub(unknown_nodes),
                    unknown_nodes,
                    max_expr_depth
                );
            }
            if let Some(sample) = &first_unknown_sample {
                println!("  - unknown_sample: {}", sample);
            }
            if !expr_counts.is_empty() {
                let summary = expr_counts
                    .iter()
                    .map(|(kind, count)| format!("{}={}", kind, count))
                    .collect::<Vec<_>>()
                    .join(", ");
                println!("  - expr_kinds: {}", summary);
            }
        }
    }

    match compiler::parse_stage1_modules(&stage1_project) {
        Ok(stage1_modules) if !stage1_modules.is_empty() => {
            linker::materialize_stage1_ast_artifacts(&stage1_project, &stage1_build_plan, &stage1_modules)?;
            linker::materialize_stage1_backend_requests(&stage1_project, &stage1_build_plan, &stage1_modules)?;
            let total_stage1_decls = stage1_modules
                .iter()
                .map(|module| module.declarations.len())
                .sum::<usize>();
            let total_stage1_functions = stage1_modules
                .iter()
                .flat_map(|module| module.declarations.iter())
                .filter(|decl| matches!(decl, compiler::TopLevelDecl::Function { .. }))
                .count();
            println!(
                "stage1_ast_summary: modules={} decls={} functions={} entry={}",
                stage1_modules.len(),
                total_stage1_decls,
                total_stage1_functions,
                stage1_build_plan.entry_module.clone()
            );
            if let Ok(link_plan_text) = std::fs::read_to_string(&stage1_build_plan.link_plan_path) {
                let mut link_summary = BTreeMap::new();
                for line in link_plan_text.lines().take(8) {
                    if let Some((key, value)) = line.split_once('\t') {
                        link_summary.insert(key.to_string(), value.to_string());
                    }
                }
                println!(
                    "stage1_link_summary: runtime_seed_symbols={} runtime_seed_edges={} call_edges={} unresolved_fixups={}",
                    link_summary
                        .get("runtime_seed_symbol_count")
                        .map(|s| s.as_str())
                        .unwrap_or("unknown"),
                    link_summary
                        .get("runtime_seed_edge_count")
                        .map(|s| s.as_str())
                        .unwrap_or("unknown"),
                    link_summary
                        .get("call_edge_count")
                        .map(|s| s.as_str())
                        .unwrap_or("unknown"),
                    link_summary
                        .get("fixup_candidate_count")
                        .map(|s| s.as_str())
                        .unwrap_or("unknown")
                );
            }
        }
        Ok(_) => {
            println!("stage1_ast_status: empty");
        }
        Err(err) => {
            println!("stage1_ast_status: blocked");
            println!("stage1_ast_error: {}", err);
        }
    }

    // ── CLI功能: 冒烟测试 ──
    if cli.smoke_test {
        return run_smoke_test(&cli, &project, &artifacts, &stage1_build_plan);
    }

    // ── CLI功能: 工料单输出 ──
    if cli.emit_bill_of_materials {
        return run_emit_bom(&project, &stage1_project, &artifacts, &stage1_build_plan);
    }

    if let Some(stage1_binary) = resolve_stage1_binary(cli.stage1_binary, &artifacts)? {
        println!("stage1_resolved: {}", stage1_binary.display());
        let mut drive = drive_stage_chain(&stage1_binary, &artifacts)?;
        // 从link_plan填充工料单扩展字段
        populate_drive_stats_from_link_plan(&mut drive, &stage1_build_plan);
        println!("stage2_exit: {}", drive.stage2_exit);
        println!("stage3_exit: {}", drive.stage3_exit);
        println!("fixed_point: {}", if drive.fixed_point { "true" } else { "false" });
        println!("convergence_rounds: {}", drive.convergence_rounds);
        if !drive.fixed_point {
            println!("diff_bytes_last: {}", drive.diff_bytes_last);
        }
        // 输出工料单扩展字段
        println!("stage2_size: {}", drive.stage2_size);
        println!("stage3_size: {}", drive.stage3_size);
        if drive.module_count > 0 { println!("module_count: {}", drive.module_count); }
        if drive.function_count > 0 { println!("function_count: {}", drive.function_count); }
        if !drive.profile.is_empty() { println!("profile: {}", drive.profile); }
        if !drive.target.is_empty() { println!("target: {}", drive.target); }
        if !drive.entry_symbol.is_empty() { println!("entry_symbol: {}", drive.entry_symbol); }
        if !drive.image_format.is_empty() { println!("image_format: {}", drive.image_format); }
        if drive.semantic_warnings > 0 { println!("semantic_warnings: {}", drive.semantic_warnings); }
        if drive.semantic_errors > 0 { println!("semantic_errors: {}", drive.semantic_errors); }
        if drive.optimization_passes > 0 { println!("optimization_passes: {}", drive.optimization_passes); }
        if drive.link_fixups_total > 0 { println!("link_fixups_total: {}", drive.link_fixups_total); }
        if drive.link_fixups_resolved > 0 { println!("link_fixups_resolved: {}", drive.link_fixups_resolved); }
        if drive.stage2_exit != 0 || drive.stage3_exit != 0 {
            return Err(format!(
                "Stage链执行失败 (stage2_exit={} stage3_exit={})",
                drive.stage2_exit,
                drive.stage3_exit
            ));
        }
        if !drive.fixed_point {
            return Err(format!("Stage链未达到不动点 (diff={})", drive.diff_bytes_last));
        }
        if drive.semantic_warnings > 0 {
            return Err(format!("检测到语义警告: {}", drive.semantic_warnings));
        }
        if drive.semantic_errors > 0 {
            return Err(format!("检测到语义错误: {}", drive.semantic_errors));
        }
    } else {
        return Err(format!(
            "Stage1 编译器不存在，禁止跳过 Stage 链驱动: {}",
            artifacts.stage1_binary.display()
        ));
    }

    println!("完成: 当前已建立适配 中文原生/内核 的本地 Stage-0 物化链与 Stage 驱动入口");
    Ok(())
}

fn parse_cli_args(args: &[String]) -> Result<CliArgs, String> {
    let mut index = 0usize;
    let mut kernel_root = None;
    let mut sync_runtime_seed = false;
    let mut stage1_binary = None;
    let mut check_keyword_contract = false;
    let mut smoke_test = false;
    let mut emit_bom = false;
    let mut source_reset_path: Option<String> = None;

    while index < args.len() {
        match args[index].as_str() {
            "--kernel-root" => {
                let value = args.get(index + 1).ok_or_else(|| {
                    "--kernel-root 缺少路径参数".to_string()
                })?;
                kernel_root = Some(PathBuf::from(value));
                index += 2;
            }
            "--stage1-binary" => {
                let value = args.get(index + 1).ok_or_else(|| {
                    "--stage1-binary 缺少路径参数".to_string()
                })?;
                stage1_binary = Some(PathBuf::from(value));
                index += 2;
            }
            "--sync-runtime-seed" => {
                sync_runtime_seed = true;
                index += 1;
            }
            "--check-keyword-contract" => {
                check_keyword_contract = true;
                index += 1;
            }
            "--smoke-test" => {
                smoke_test = true;
                index += 1;
            }
            "--emit-bom" => {
                emit_bom = true;
                index += 1;
            }
            "--source-reset" => {
                index += 1;
                if index < args.len() {
                    source_reset_path = Some(args[index].clone());
                    index += 1;
                } else {
                    return Err("--source-reset 需要项目路径参数".into());
                }
            }
            other => {
                return Err(format!("不支持的参数: {}", other));
            }
        }
    }

    Ok(CliArgs {
        kernel_root_override: kernel_root,
        sync_runtime_seed,
        stage1_binary,
        check_keyword_contract,
        smoke_test,
        emit_bill_of_materials: emit_bom,
        source_reset_path,
    })
}

fn resolve_stage1_binary(
    cli_stage1_binary: Option<PathBuf>,
    artifacts: &elf::StageArtifacts,
) -> Result<Option<PathBuf>, String> {
    if let Some(stage1_binary) = cli_stage1_binary {
        if !stage1_binary.is_file() {
            return Err(format!("Stage1 编译器不存在: {}", stage1_binary.display()));
        }
        return Ok(Some(stage1_binary));
    }

    if artifacts.stage1_binary.is_file() {
        return Ok(Some(artifacts.stage1_binary.clone()));
    }

    Ok(None)
}

fn compile_one_stage(compiler: &Path, cwd: &Path, output: &Path) -> Result<i32, String> {
    // 自举固定点要求 Stage2/Stage3 的编译输入完全同构。
    // Nova 当前仍会把 -o 目标路径带进产物数据段，因此这里统一使用规范输出名，
    // 再把同一份字节复制到阶段归档路径，避免阶段名本身破坏固定点。
    let canonical_compiler = output
        .parent()
        .unwrap_or(Path::new("."))
        .join("固定点_驱动编译器");
    let canonical_output = output
        .parent()
        .unwrap_or(Path::new("."))
        .join("固定点_编译器");
    if canonical_compiler.exists() {
        fs::remove_file(&canonical_compiler)
            .map_err(|e| format!("删除旧规范驱动失败: {} ({})", canonical_compiler.display(), e))?;
    }
    let out_str = canonical_output.to_string_lossy().to_string();
    if canonical_output.exists() {
        fs::remove_file(&canonical_output)
            .map_err(|e| format!("删除旧规范产物失败: {} ({})", canonical_output.display(), e))?;
    }
    if output.exists() {
        fs::remove_file(output).map_err(|e| format!("删除旧产物失败: {} ({})", output.display(), e))?;
    }
    fs::copy(compiler, &canonical_compiler).map_err(|e| {
        format!(
            "复制规范驱动失败: {} -> {} ({})",
            compiler.display(),
            canonical_compiler.display(),
            e
        )
    })?;
    let compiler_perms = fs::metadata(compiler)
        .map_err(|e| format!("读取编译器权限失败: {} ({})", compiler.display(), e))?
        .permissions();
    fs::set_permissions(&canonical_compiler, compiler_perms)
        .map_err(|e| format!("同步规范驱动权限失败: {} ({})", canonical_compiler.display(), e))?;
    let exit = run_binary(
        &canonical_compiler,
        cwd,
        &[".", "--compile", "--module-graph", "--target", "linux", "-o", &out_str],
    )?;
    if exit == 0 {
        if !canonical_output.is_file() {
            return Err(format!("规范产物未生成: {}", canonical_output.display()));
        }
        if canonical_output != output {
            fs::copy(&canonical_output, output).map_err(|e| {
                format!(
                    "复制规范产物失败: {} -> {} ({})",
                    canonical_output.display(),
                    output.display(),
                    e
                )
            })?;
            let perms = fs::metadata(&canonical_output)
                .map_err(|e| format!("读取规范产物权限失败: {} ({})", canonical_output.display(), e))?
                .permissions();
            fs::set_permissions(output, perms)
                .map_err(|e| format!("同步产物权限失败: {} ({})", output.display(), e))?;
        }
    }
    if canonical_compiler.exists() {
        let _ = fs::remove_file(&canonical_compiler);
    }
    Ok(exit)
}

// ─── 历史Stage2后处理patch已全部清除 (2026-04-17) ───
// 曾经存在: patch_stage2_dict_inline_strcmp / patch_elf_zero_fill / patch_stage2_init
// 根治路径: 全部从源码层解决 - 内联strcmp改用call 字符串相等 / 零填充改用 Nova原生零内存+模块_分块写文件 / init死循环修复代码生成器根因
// 禁止复活: NOVA_ENABLE_STAGE2_PATCH 环境变量仍然作为 fail-fast 保护, 严禁二进制后处理绕过根因

fn count_diff_bytes(a: &[u8], b: &[u8]) -> usize {
    let common = a.len().min(b.len());
    let diffs: usize = a[..common].iter().zip(b[..common].iter()).filter(|(x, y)| x != y).count();
    diffs + a.len().abs_diff(b.len())
}

fn drive_stage_chain(stage1_binary: &Path, artifacts: &elf::StageArtifacts) -> Result<StageDriveStats, String> {
    if !stage1_binary.is_file() {
        return Err(format!("Stage1 编译器不存在: {}", stage1_binary.display()));
    }

    // 第一轮: 使用提供的 Stage1 编译 Stage2 和 Stage3
    let s2_exit = compile_one_stage(stage1_binary, &artifacts.stage0_root, &artifacts.stage2_binary)?;
    if s2_exit != 0 {
        return Err(format!("Stage1→Stage2 执行失败，退出码={}.", s2_exit));
    }
    if !artifacts.stage2_binary.is_file() {
        return Err(format!("Stage2 产物未生成: {}", artifacts.stage2_binary.display()));
    }

    // 2026-04-17 所有Rust层post-patch已彻底移除:
    // - patch_stage2_dict_inline_strcmp: 源码根治 (前端入口_运行时种子.nova 3处内联strcmp改为call 字符串相等)
    // - patch_elf_zero_fill: 源码根治 (模块_写LinuxELF 使用 Nova原生 零内存(N)+模块_分块写文件, 不再依赖OS工具)
    // - patch_stage2_init: 函数废弃 (本应修复 __初始化全局__ 代码生成, 不能靠NOP二进制)
    // fail-fast: 禁止重新启用任何Stage2 post-patch
    if std::env::var_os("NOVA_ENABLE_STAGE2_PATCH").is_some() {
        return Err("禁止启用 Stage2 post-patch：必须修复真实代码生成/初始化根因，不能靠二进制 NOP/fallback patch 通过".to_string());
    }

    let s3_exit = compile_one_stage(&artifacts.stage2_binary, &artifacts.stage0_root, &artifacts.stage3_binary)?;
    if s3_exit != 0 {
        return Err(format!("Stage2→Stage3 执行失败，退出码={}.", s3_exit));
    }
    if !artifacts.stage3_binary.is_file() {
        return Err(format!("Stage3 产物未生成: {}", artifacts.stage3_binary.display()));
    }

    let b2 = fs::read(&artifacts.stage2_binary)
        .map_err(|e| format!("读取 Stage2 失败: {}", e))?;
    let b3 = fs::read(&artifacts.stage3_binary)
        .map_err(|e| format!("读取 Stage3 失败: {}", e))?;
    let diff0 = count_diff_bytes(&b2, &b3);

    if b2 == b3 {
        return Ok(StageDriveStats { stage2_exit: s2_exit, stage3_exit: s3_exit, fixed_point: true, convergence_rounds: 1, diff_bytes_last: 0,
            stage2_size: b2.len(), stage3_size: b3.len(), module_count: 0, function_count: 0,
            profile: String::new(), target: String::new(), entry_symbol: String::new(), image_format: String::new(),
            semantic_warnings: 0, semantic_errors: 0, optimization_passes: 0, link_fixups_total: 0, link_fixups_resolved: 0 });
    }

    println!("convergence_round=1 diff_bytes={} size2={} size3={}", diff0, b2.len(), b3.len());

    // 后续轮次: 用上轮 Stage3 作为新 Stage1 重新编译
    let mut last_diff = diff0;
    let mut last_s3_exit = s3_exit;
    for round in 2..=MAX_CONVERGENCE_ROUNDS {
        // 安全备份 Stage3 （避免删除自身）
        let tmp_s1 = artifacts.stage3_binary
            .with_file_name(format!("收敛自举_r{}", round));
        fs::copy(&artifacts.stage3_binary, &tmp_s1)
            .map_err(|e| format!("备份 Stage3 失败: {}", e))?;

        let exit2 = compile_one_stage(&tmp_s1, &artifacts.stage0_root, &artifacts.stage2_binary)?;
        let _ = fs::remove_file(&tmp_s1);
        if exit2 != 0 {
            return Err(format!("收敛轮{} Stage1→Stage2 失败, 退出码={}", round, exit2));
        }

        let exit3 = compile_one_stage(&artifacts.stage2_binary, &artifacts.stage0_root, &artifacts.stage3_binary)?;
        last_s3_exit = exit3;
        if exit3 != 0 {
            return Err(format!("收敛轮{} Stage2→Stage3 失败, 退出码={}", round, exit3));
        }

        let nb2 = fs::read(&artifacts.stage2_binary)
            .map_err(|e| format!("读取 Stage2 失败(轮{}): {}", round, e))?;
        let nb3 = fs::read(&artifacts.stage3_binary)
            .map_err(|e| format!("读取 Stage3 失败(轮{}): {}", round, e))?;
        last_diff = count_diff_bytes(&nb2, &nb3);

        println!("convergence_round={} diff_bytes={} size2={} size3={}", round, last_diff, nb2.len(), nb3.len());

        if nb2 == nb3 {
            return Ok(StageDriveStats {
                stage2_exit: exit2,
                stage3_exit: exit3,
                fixed_point: true,
                convergence_rounds: round,
                diff_bytes_last: 0,
                stage2_size: nb2.len(), stage3_size: nb3.len(), module_count: 0, function_count: 0,
                profile: String::new(), target: String::new(), entry_symbol: String::new(), image_format: String::new(),
                semantic_warnings: 0, semantic_errors: 0, optimization_passes: 0, link_fixups_total: 0, link_fixups_resolved: 0,
            });
        }
    }

    Ok(StageDriveStats {
        stage2_exit: s2_exit,
        stage3_exit: last_s3_exit,
        fixed_point: false,
        convergence_rounds: MAX_CONVERGENCE_ROUNDS,
        diff_bytes_last: last_diff,
        stage2_size: 0, stage3_size: 0, module_count: 0, function_count: 0,
        profile: String::new(), target: String::new(), entry_symbol: String::new(), image_format: String::new(),
        semantic_warnings: 0, semantic_errors: 0, optimization_passes: 0, link_fixups_total: 0, link_fixups_resolved: 0,
    })
}

fn populate_drive_stats_from_link_plan(
    drive: &mut StageDriveStats,
    build_plan: &linker::Stage1BuildPlan,
) {
    // 从link_plan文件读取链接统计
    if let Ok(link_text) = fs::read_to_string(&build_plan.link_plan_path) {
        for line in link_text.lines().take(15) {
            if let Some((key, value)) = line.split_once('\t') {
                match key {
                    "call_edge_count" => { drive.link_fixups_total = value.parse().unwrap_or(0); }
                    "fixup_resolved_count" => { drive.link_fixups_resolved = value.parse().unwrap_or(0); }
                    _ => {}
                }
            }
        }
    }
    // 从AST摘要读取模块和函数数
    if let Ok(ast_text) = fs::read_to_string(&build_plan.ast_index_path) {
        let lines: Vec<&str> = ast_text.lines().collect();
        drive.module_count = lines.len();
        // 函数数从link_plan的function_count字段或AST统计
        drive.function_count = lines.iter()
            .filter_map(|l| l.split_once('\t'))
            .filter_map(|(_, v)| v.parse::<usize>().ok())
            .sum();
    }
    // 填充profile/target/entry_symbol
    drive.profile = "self-bootstrap".to_string();
    drive.target = "x86_64-linux".to_string();
    drive.entry_symbol = "Nova.nova".to_string();
    drive.image_format = "elf64".to_string();
}

// ═══════════════════════════════════════════
// CLI功能实现
// ═══════════════════════════════════════════

fn run_keyword_contract_check() -> Result<(), String> {
    use std::collections::{HashMap, HashSet};
    println!("═══ 关键字契约检查 ═══");
    println!("对比: Rust辅助前端(compiler.rs) vs 种子编译器(backend_seed.rs)");
    println!();

    let helper_kw = compiler::helper_frontend_keywords();
    let seed_kw = backend_seed::seed_compiler_keywords();

    let helper_set: HashSet<&str> = helper_kw.iter().map(|(alias, _)| *alias).collect();
    let seed_set: HashSet<&str> = seed_kw.iter().map(|(alias, _)| *alias).collect();

    let helper_map: HashMap<&str, &str> = helper_kw.iter().cloned().collect();
    let seed_map: HashMap<&str, &str> = seed_kw.iter().cloned().collect();

    let only_helper: Vec<&&str> = helper_set.difference(&seed_set).collect();
    let only_seed: Vec<&&str> = seed_set.difference(&helper_set).collect();

    let mut mismatch = Vec::new();
    for alias in helper_set.intersection(&seed_set) {
        let h = helper_map[alias];
        let s = seed_map[alias];
        if h != s {
            mismatch.push((*alias, h, s));
        }
    }

    let mut issues = 0;

    if !only_helper.is_empty() {
        println!("[差异] 仅Rust辅助前端有（种子编译器缺失）:");
        for kw in &only_helper {
            println!("  - \"{}\" → \"{}\"", kw, helper_map[**kw]);
        }
        issues += only_helper.len();
    }

    if !only_seed.is_empty() {
        println!("[差异] 仅种子编译器有（Rust辅助前端缺失）:");
        for kw in &only_seed {
            println!("  - \"{}\" → \"{}\"", kw, seed_map[**kw]);
        }
        issues += only_seed.len();
    }

    if !mismatch.is_empty() {
        println!("[冲突] 同一别名映射到不同正规形式:");
        for (alias, h, s) in &mismatch {
            println!("  - \"{}\" : helper=\"{}\" vs seed=\"{}\"", alias, h, s);
        }
        issues += mismatch.len();
    }

    println!();
    println!("helper关键字总数: {}", helper_kw.len());
    println!("seed关键字总数: {}", seed_kw.len());
    println!("交集: {}", helper_set.intersection(&seed_set).count());
    println!("差异项: {}", issues);

    if issues == 0 {
        println!("keyword_contract: PASS");
    } else {
        println!("keyword_contract: FAIL ({} issues)", issues);
    }
    Ok(())
}

fn run_smoke_test(
    cli: &CliArgs,
    project: &compiler::KernelProject,
    artifacts: &elf::StageArtifacts,
    _build_plan: &linker::Stage1BuildPlan,
) -> Result<(), String> {
    println!("═══ 冒烟测试 ═══");

    // 1. 检查Stage0物化
    println!("[1/4] Stage-0 物化检查...");
    if !artifacts.stage0_root.is_dir() {
        println!("  FAIL: Stage-0 工作区不存在: {}", artifacts.stage0_root.display());
        println!("smoke_test: FAIL");
        return Err(format!("smoke_test失败: Stage-0 工作区不存在: {}", artifacts.stage0_root.display()));
    }
    println!("  PASS: Stage-0 工作区存在");

    // 2. 检查manifest
    println!("[2/4] Manifest 一致性检查...");
    if !project.manifest_source.is_file() {
        println!("  FAIL: manifest不存在: {}", project.manifest_source.display());
        println!("smoke_test: FAIL");
        return Err(format!("smoke_test失败: manifest不存在: {}", project.manifest_source.display()));
    }
    println!("  PASS: manifest存在 ({})", project.manifest_source.display());

    // 3. 尝试Stage链驱动
    println!("[3/4] Stage链驱动...");
    if let Some(stage1_binary) = resolve_stage1_binary(cli.stage1_binary.clone(), artifacts)? {
        let drive = drive_stage_chain(&stage1_binary, artifacts)?;
        println!("  stage2_exit: {}", drive.stage2_exit);
        println!("  stage3_exit: {}", drive.stage3_exit);
        println!("  fixed_point: {}", drive.fixed_point);
        println!("  convergence_rounds: {}", drive.convergence_rounds);
        println!("  size2: {}", drive.stage2_size);
        println!("  size3: {}", drive.stage3_size);

        if drive.stage2_exit != 0 || drive.stage3_exit != 0 {
            println!("  FAIL: Stage链执行失败");
            println!("smoke_test: FAIL");
            return Err(format!(
                "smoke_test失败: Stage链执行失败 (stage2_exit={} stage3_exit={})",
                drive.stage2_exit,
                drive.stage3_exit
            ));
        }
        if !drive.fixed_point {
            println!("  FAIL: 未达到不动点 (diff={})", drive.diff_bytes_last);
            println!("smoke_test: FAIL");
            return Err(format!("smoke_test失败: 未达到不动点 (diff={})", drive.diff_bytes_last));
        }
        println!("  PASS: Stage链收敛");
    } else {
        println!("  FAIL: Stage1编译器不存在，无法执行Stage链驱动");
        println!("smoke_test: FAIL");
        return Err("smoke_test失败: Stage1编译器不存在，禁止跳过Stage链驱动".to_string());
    }

    // 4. 关键字契约快速检查
    println!("[4/4] 关键字契约快检...");
    let helper_set: std::collections::HashSet<&str> = compiler::helper_frontend_keywords()
        .iter().map(|(a, _)| *a).collect();
    let seed_set: std::collections::HashSet<&str> = backend_seed::seed_compiler_keywords()
        .iter().map(|(a, _)| *a).collect();
    let diff_count = helper_set.symmetric_difference(&seed_set).count();
    if diff_count > 0 {
        println!("  FAIL: 关键字差异 {} 项 (运行 --check-keyword-contract 查看详情)", diff_count);
        println!("smoke_test: FAIL");
        return Err(format!("smoke_test失败: 关键字契约差异 {} 项", diff_count));
    }
    println!("  PASS: 关键字契约一致");

    println!();
    println!("smoke_test: PASS");
    Ok(())
}

fn run_emit_bom(
    project: &compiler::KernelProject,
    stage1_project: &compiler::KernelProject,
    artifacts: &elf::StageArtifacts,
    build_plan: &linker::Stage1BuildPlan,
) -> Result<(), String> {
    println!("═══ 工料单 (Bill of Materials) ═══");
    println!();

    // 项目信息
    println!("[项目]");
    println!("  内核根: {}", project.kernel_root.display());
    println!("  清单源: {}", project.manifest_source.display());
    println!("  Stage0根: {}", artifacts.stage0_root.display());
    println!();

    // 模块统计
    println!("[模块统计]");
    let stage1_plan = linker::plan_stage1_module_collection(stage1_project)?;
    println!("  Stage1模块数: {}", stage1_plan.module_count);
    println!("  入口模块: {}", stage1_plan.entry_module);
    println!();

    // AST统计
    println!("[AST统计]");
    match compiler::parse_stage1_modules(stage1_project) {
        Ok(modules) => {
            let total_decls: usize = modules.iter().map(|m| m.declarations.len()).sum();
            let total_functions: usize = modules.iter()
                .flat_map(|m| m.declarations.iter())
                .filter(|d| matches!(d, compiler::TopLevelDecl::Function { .. }))
                .count();
            let total_structs: usize = modules.iter()
                .flat_map(|m| m.declarations.iter())
                .filter(|d| matches!(d, compiler::TopLevelDecl::Struct { .. }))
                .count();
            let total_globals: usize = modules.iter()
                .flat_map(|m| m.declarations.iter())
                .filter(|d| matches!(d, compiler::TopLevelDecl::Global { .. }))
                .count();
            let total_imports: usize = modules.iter()
                .flat_map(|m| m.declarations.iter())
                .filter(|d| matches!(d, compiler::TopLevelDecl::Import { .. }))
                .count();
            println!("  模块数: {}", modules.len());
            println!("  总声明数: {}", total_decls);
            println!("  函数数: {}", total_functions);
            println!("  结构体数: {}", total_structs);
            println!("  全局变量数: {}", total_globals);
            println!("  导入数: {}", total_imports);
        }
        Err(e) => {
            println!("  AST解析失败: {}", e);
        }
    }
    println!();

    // 链接统计
    println!("[链接统计]");
    if let Ok(link_text) = fs::read_to_string(&build_plan.link_plan_path) {
        for line in link_text.lines().take(10) {
            if let Some((key, value)) = line.split_once('\t') {
                println!("  {}: {}", key, value);
            }
        }
    } else {
        println!("  链接计划未生成");
    }
    println!();

    // 产物路径
    println!("[产物路径]");
    println!("  Stage1二进制: {} ({})", artifacts.stage1_binary.display(),
        if artifacts.stage1_binary.is_file() { format!("{} bytes", fs::metadata(&artifacts.stage1_binary).map(|m| m.len()).unwrap_or(0)) } else { "不存在".to_string() });
    println!("  Stage2二进制: {} ({})", artifacts.stage2_binary.display(),
        if artifacts.stage2_binary.is_file() { format!("{} bytes", fs::metadata(&artifacts.stage2_binary).map(|m| m.len()).unwrap_or(0)) } else { "不存在".to_string() });
    println!("  Stage3二进制: {} ({})", artifacts.stage3_binary.display(),
        if artifacts.stage3_binary.is_file() { format!("{} bytes", fs::metadata(&artifacts.stage3_binary).map(|m| m.len()).unwrap_or(0)) } else { "不存在".to_string() });
    println!("  运行时blob: {} ({})", build_plan.runtime_blob_path.display(),
        if build_plan.runtime_blob_path.is_file() { format!("{} bytes", fs::metadata(&build_plan.runtime_blob_path).map(|m| m.len()).unwrap_or(0)) } else { "不存在".to_string() });
    println!();

    // 关键字契约摘要
    println!("[关键字契约]");
    let helper_kw = compiler::helper_frontend_keywords();
    let seed_kw = backend_seed::seed_compiler_keywords();
    let helper_set: std::collections::HashSet<&str> = helper_kw.iter().map(|(a, _)| *a).collect();
    let seed_set: std::collections::HashSet<&str> = seed_kw.iter().map(|(a, _)| *a).collect();
    println!("  辅助前端关键字: {}", helper_kw.len());
    println!("  种子编译器关键字: {}", seed_kw.len());
    println!("  交集: {}", helper_set.intersection(&seed_set).count());
    println!("  差异: {}", helper_set.symmetric_difference(&seed_set).count());
    println!();

    println!("bom_status: complete");
    Ok(())
}

fn run_binary(binary: &Path, cwd: &Path, args: &[&str]) -> Result<i32, String> {
    let status = Command::new(binary)
        .args(args)
        .current_dir(cwd)
        .status()
        .map_err(|e| format!("执行失败: {} ({})", binary.display(), e))?;
    if let Some(code) = status.code() {
        Ok(code)
    } else {
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            if let Some(sig) = status.signal() {
                eprintln!("进程被信号终止: signal={} ({})", sig, binary.display());
                return Ok(-(sig as i32));
            }
        }
        Ok(-1)
    }
}
