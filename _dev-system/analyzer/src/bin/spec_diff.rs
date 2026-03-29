use anyhow::{bail, Context, Result};
use chrono::{DateTime, Duration, Utc};
use regex::Regex;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct BaselineReport {
    pub task: Option<String>,
    pub category: Option<String>,
    pub bundles: Option<Vec<BaselineBundle>>,
    pub snapshots: Option<Vec<BaselineSnapshot>>, // Support flattened format
    pub timestamp: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct BaselineBundle {
    pub headline: String,
    pub snapshots: Vec<BaselineSnapshot>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
struct BaselineSnapshot {
    pub path: String,
    pub functions: Vec<BaselineFunction>,
    pub fingerprint: String,
}

#[derive(Debug, Deserialize, Clone)]
struct BaselineFunction {
    pub name: String,
    pub signature: String,
}

#[derive(Debug)]
#[allow(dead_code)]
struct TargetSnapshot {
    path: PathBuf,
    functions: Vec<FunctionRecord>,
    fingerprint: String,
}

#[derive(Debug)]
struct FunctionRecord {
    name: String,
    signature: String,
    line: usize,
}

fn main() -> Result<()> {
    let (baseline_path, mut targets) = parse_args()?;
    let report = load_baseline(&baseline_path)?;

    // Expiry Check (30 days)
    if let Some(timestamp) = report.timestamp {
        let now = Utc::now();
        let age = now - timestamp;
        if age > Duration::days(30) {
            println!(
                "\n❌ Verification baseline EXPIRED ({} days old).",
                age.num_days()
            );
            println!(
                "Please regenerate the baseline artifacts before proceeding with the refactor."
            );
            process::exit(1);
        } else {
            println!("✅ Baseline verified (Age: {} days).", age.num_days());
        }
    } else {
        println!("⚠️  Baseline has no timestamp. Proceeding with caution (Legacy mode).");
    }

    let baseline_snapshots = if let Some(bundles) = &report.bundles {
        bundles
            .iter()
            .flat_map(|b| b.snapshots.clone())
            .collect::<Vec<_>>()
    } else if let Some(snapshots) = &report.snapshots {
        snapshots.clone()
    } else {
        bail!("No snapshots found in baseline report.");
    };

    if targets.is_empty() {
        targets = baseline_snapshots
            .iter()
            .map(|snapshot| PathBuf::from(&snapshot.path))
            .collect();
    }

    let baseline_keys = accumulate_baseline_keys(&baseline_snapshots);
    let target_snapshots = parse_targets(&targets)?;
    let target_keys = accumulate_target_keys(&target_snapshots);

    print_summary(&report, baseline_snapshots.len(), &target_snapshots);

    let mut missing = Vec::new();
    for (key, _sources) in &baseline_keys {
        let (name, baseline_sig) = key;
        let mut found = false;

        for (t_name, t_sig) in &target_keys {
            if t_name == name && (t_sig.contains(baseline_sig) || baseline_sig.contains(t_sig)) {
                found = true;
                break;
            }
        }

        if !found {
            missing.push(key.clone());
        }
    }

    if !missing.is_empty() {
        println!("\nMissing functions (present in baseline but not targets):");
        let mut missing_sorted = missing.clone();
        missing_sorted.sort();
        for (name, signature) in missing_sorted {
            println!("  - {} — {}", name, signature);
        }
        println!("\n❌ Function surface mismatch detected. Adjust the refactor or ensure all targets are provided.");
        process::exit(1);
    } else {
        println!("\n✅ Function surface matches baseline snapshots (Semantic AST Verified).");
    }

    Ok(())
}

fn parse_args() -> Result<(PathBuf, Vec<PathBuf>)> {
    let raw_args: Vec<String> = env::args().skip(1).collect();
    let mut idx = 0;
    let mut baseline = None;
    let mut targets = Vec::new();

    while idx < raw_args.len() {
        match raw_args[idx].as_str() {
            "--baseline" => {
                idx += 1;
                if idx >= raw_args.len() {
                    bail!("Missing value for --baseline");
                }
                baseline = Some(PathBuf::from(&raw_args[idx]));
            }
            "--targets" => {
                idx += 1;
                while idx < raw_args.len() && !raw_args[idx].starts_with("--") {
                    targets.push(PathBuf::from(&raw_args[idx]));
                    idx += 1;
                }
                idx -= 1;
            }
            other => {
                bail!("Unknown argument: {}", other);
            }
        }
        idx += 1;
    }

    let baseline = baseline.ok_or_else(|| anyhow::anyhow!("--baseline is required"))?;
    Ok((baseline, targets))
}

fn load_baseline(path: &Path) -> Result<BaselineReport> {
    let data = fs::read_to_string(path)
        .with_context(|| format!("Failed to read baseline: {}", path.display()))?;
    let report: BaselineReport = serde_json::from_str(&data)
        .with_context(|| format!("Failed to parse baseline JSON: {}", path.display()))?;
    Ok(report)
}

fn accumulate_baseline_keys(
    snapshots: &[BaselineSnapshot],
) -> HashMap<(String, String), HashSet<String>> {
    let mut map: HashMap<(String, String), HashSet<String>> = HashMap::new();
    for snapshot in snapshots {
        for function in &snapshot.functions {
            let key = (function.name.clone(), function.signature.clone());
            map.entry(key).or_default().insert(snapshot.path.clone());
        }
    }
    map
}

fn parse_targets(targets: &[PathBuf]) -> Result<Vec<TargetSnapshot>> {
    let mut snapshots = Vec::new();
    for path in targets {
        if !path.exists() {
            bail!("Target file missing: {}", path.display());
        }
        snapshots.push(parse_target(path)?);
    }
    Ok(snapshots)
}

fn parse_target(path: &Path) -> Result<TargetSnapshot> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read target file: {}", path.display()))?;
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    let functions = parse_functions(&content, &ext);
    let fingerprint = fingerprint_functions(&functions);
    Ok(TargetSnapshot {
        path: path.to_path_buf(),
        functions,
        fingerprint,
    })
}

fn accumulate_target_keys(snapshots: &[TargetSnapshot]) -> HashSet<(String, String)> {
    let mut set = HashSet::new();
    for snapshot in snapshots {
        for function in &snapshot.functions {
            set.insert((function.name.clone(), function.signature.clone()));
        }
    }
    set
}

fn print_summary(report: &BaselineReport, snapshot_count: usize, targets: &[TargetSnapshot]) {
    println!("Baseline report:");
    if let Some(task) = &report.task {
        println!("  - Task: {}", task);
    }
    println!("  - Snapshots: {} files", snapshot_count);
    println!("Targets:");
    for snapshot in targets {
        println!(
            "  - {}: {} functions detected",
            snapshot.path.display(),
            snapshot.functions.len()
        );
    }
}

fn parse_functions(content: &str, ext: &str) -> Vec<FunctionRecord> {
    match ext {
        "rs" => parse_rust_functions(content),
        _ => Vec::new(),
    }
}

fn parse_rust_functions(content: &str) -> Vec<FunctionRecord> {
    use syn::visit::Visit;

    if let Ok(file) = syn::parse_file(content) {
        struct FnVisitor<'a> {
            source: &'a str,
            functions: Vec<FunctionRecord>,
        }

        impl<'a> FnVisitor<'a> {
            fn push_signature(&mut self, name: String, line: usize) {
                let start = line_start_offset(self.source, line);
                self.functions.push(FunctionRecord {
                    name,
                    signature: extract_line(self.source, start),
                    line,
                });
            }
        }

        impl<'ast, 'a> Visit<'ast> for FnVisitor<'a> {
            fn visit_item_fn(&mut self, i: &'ast syn::ItemFn) {
                let name = i.sig.ident.to_string();
                let line = i.sig.fn_token.span.start().line;
                self.push_signature(name, line);
                syn::visit::visit_item_fn(self, i);
            }

            fn visit_impl_item_fn(&mut self, i: &'ast syn::ImplItemFn) {
                let name = i.sig.ident.to_string();
                let line = i.sig.fn_token.span.start().line;
                self.push_signature(name, line);
                syn::visit::visit_impl_item_fn(self, i);
            }
        }

        let mut visitor = FnVisitor {
            source: content,
            functions: Vec::new(),
        };
        visitor.visit_file(&file);
        visitor.functions.sort_by_key(|f| f.line);
        visitor
            .functions
            .dedup_by(|a, b| a.name == b.name && a.signature == b.signature && a.line == b.line);
        return visitor.functions;
    }

    let regex = Regex::new(r"\b(?:pub\s+)?(?:async\s+)?fn\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap();
    let mut functions = Vec::new();
    for cap in regex.captures_iter(content) {
        let name = cap[1].to_string();
        if let Some(mat) = cap.get(0) {
            let line = line_number(content, mat.start());
            functions.push(FunctionRecord {
                name,
                signature: extract_line(content, mat.start()),
                line,
            });
        }
    }
    functions.sort_by_key(|f| f.line);
    functions
}

fn fingerprint_functions(functions: &[FunctionRecord]) -> String {
    let mut hasher = Sha256::new();
    for func in functions {
        hasher.update(func.name.as_bytes());
        hasher.update(b"\n");
    }
    format!("{:x}", hasher.finalize())
}

fn line_number(content: &str, pos: usize) -> usize {
    content[..pos].chars().filter(|&c| c == '\n').count() + 1
}

fn line_start_offset(content: &str, line_1_based: usize) -> usize {
    if line_1_based <= 1 {
        return 0;
    }

    let mut line = 1usize;
    for (idx, ch) in content.char_indices() {
        if ch == '\n' {
            line += 1;
            if line == line_1_based {
                return idx + 1;
            }
        }
    }
    0
}

fn extract_line(content: &str, start: usize) -> String {
    let snippet = &content[start..];
    let end = snippet.find('\n').unwrap_or(snippet.len());
    snippet[..end].trim().to_string()
}
