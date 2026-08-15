//! Repository tasks: `cargo xtask <command>`.
//!
//! The content tree has no compiler watching over it, so these commands are
//! what keep 250 problems honest — `lint` runs every trace the way the app
//! would, `stats` reports coverage, `trace` dumps a single animation to the
//! terminal, and `package` assembles a per-OS distributable.

use anyhow::{bail, Context, Result};
use dsa_content::{lint, Library};
use dsa_core::problem::{InputMap, InputValue, Tier};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(|s| s.as_str()).unwrap_or("help");
    let rest = &args[args.len().min(1)..];

    match cmd {
        "lint" => cmd_lint(rest),
        "stats" => cmd_stats(rest),
        "trace" => cmd_trace(rest),
        "new" => cmd_new(rest),
        "package" => cmd_package(rest),
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        other => {
            print_help();
            bail!("unknown command \"{other}\"")
        }
    }
}

fn print_help() {
    eprintln!(
        r#"cargo xtask <command>

  lint [--quick]        check every content pack; --quick skips running traces
  stats                 coverage per category and tier
  trace <slug> [k=v]    run one problem's animation and print its steps
  new <slug>            scaffold a content pack
  package [--out DIR]   build release and assemble a distributable tree

Content root is found automatically; override with DSA_CONTENT."#
    );
}

// ─────────────────────────────────────────────────────────────────────────────

fn repo_root() -> PathBuf {
    // crates/xtask/ -> crates/ -> rust/ -> repo
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .unwrap_or(Path::new("."))
        .to_path_buf()
}

fn content_root() -> Result<PathBuf> {
    let extra = vec![repo_root().join("content")];
    dsa_content::find_content_root(&extra).map_err(|tried| {
        anyhow::anyhow!(
            "no content root found (looked for catalog.toml in):\n  {}",
            tried
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join("\n  ")
        )
    })
}

fn load() -> Result<Library> {
    let root = content_root()?;
    println!("content: {}", root.display());
    Ok(Library::load(root))
}

// ─────────────────────────────────────────────────────────────────────────────

fn cmd_lint(args: &[String]) -> Result<()> {
    let deep = !args.iter().any(|a| a == "--quick");
    let lib = load()?;
    let report = lint(&lib, deep);

    for issue in &report.issues {
        println!("{issue}");
    }

    println!(
        "\n{} pack(s) checked, {} trace(s) run, {} step(s) recorded",
        report.packs_checked, report.traces_run, report.total_steps
    );
    println!(
        "{} error(s), {} warning(s)",
        report.errors(),
        report.warnings()
    );

    if report.errors() > 0 {
        bail!("content has errors");
    }
    Ok(())
}

fn cmd_stats(_args: &[String]) -> Result<()> {
    let lib = load()?;

    let mut by_cat: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    for item in &lib.catalog {
        let e = by_cat.entry(item.category.as_str()).or_default();
        e.1 += 1;
        if item.viz {
            e.0 += 1;
        }
    }

    println!("\n{:<32} {:>10}", "category", "animated");
    println!("{}", "-".repeat(64));
    // Catalog order, not alphabetical — it is a learning path.
    for cat in &lib.categories {
        let (viz, total) = by_cat.get(cat.as_str()).copied().unwrap_or((0, 0));
        let width = 20;
        let filled = if total == 0 { 0 } else { viz * width / total };
        println!(
            "{:<32} {:>4}/{:<4}  [{}{}]",
            cat,
            viz,
            total,
            "#".repeat(filled),
            ".".repeat(width - filled)
        );
    }

    println!("\n{:<12} {:>10}", "tier", "animated");
    println!("{}", "-".repeat(64));
    for tier in Tier::ALL {
        let (viz, total) = lib.tier_progress(tier);
        println!("{:<18} {:>4}/{:<4}", tier.title(), viz, total);
    }

    let packs = lib.packs().count();
    let scripted = lib.packs().filter(|(_, p)| p.has_script()).count();
    println!(
        "\n{packs} content pack(s) on disk, {scripted} with a trace script, {} language source(s)",
        lib.packs().map(|(_, p)| p.sources.len()).sum::<usize>()
    );
    if !lib.errors.is_empty() {
        println!(
            "\n{} load error(s) — run `cargo xtask lint`",
            lib.errors.len()
        );
    }
    Ok(())
}

fn cmd_trace(args: &[String]) -> Result<()> {
    let slug = args
        .first()
        .context("usage: cargo xtask trace <slug> [key=value ...]")?;
    let lib = load()?;
    let pack = lib
        .pack(slug)
        .with_context(|| format!("no pack named \"{slug}\""))?;

    let mut input: InputMap = pack.meta.default_input.clone();
    for kv in &args[1..] {
        let Some((k, v)) = kv.split_once('=') else {
            continue;
        };
        let field = pack.meta.inputs.iter().find(|f| f.name == k);
        let parsed = match field {
            Some(f) => f.parse(v).map_err(anyhow::Error::msg)?,
            None => InputValue::Str(v.to_string()),
        };
        input.insert(k.to_string(), parsed);
    }

    let trace = lib
        .trace(slug, &input)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("\n{} — {} step(s)\n", pack.meta.title, trace.len());
    for (i, s) in trace.steps.iter().enumerate() {
        let views: Vec<&str> = s.views.iter().map(|v| v.label()).collect();
        println!(
            "{:>4}  {}{:<12} d{}  {}",
            i,
            "  ".repeat(s.depth.saturating_sub(1)),
            s.tag,
            s.depth,
            s.note
        );
        if !views.is_empty() {
            println!(
                "      {}[{}]",
                "  ".repeat(s.depth.saturating_sub(1)),
                views.join(" | ")
            );
        }
    }
    if let Some(r) = &trace.result {
        println!("\nresult: {r}");
    }
    Ok(())
}

fn cmd_new(args: &[String]) -> Result<()> {
    let slug = args.first().context("usage: cargo xtask new <slug>")?;
    let root = content_root()?;
    let dir = root.join("problems").join(slug);
    if dir.exists() {
        bail!("{} already exists", dir.display());
    }
    std::fs::create_dir_all(dir.join("code"))?;

    let title: String = slug
        .split('-')
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    std::fs::write(
        dir.join("problem.toml"),
        format!(
            r#"slug = "{slug}"
title = "{title}"
category = "Arrays & Hashing"
difficulty = "Easy"
tier = "250"
complexity = "O(n) time · O(1) space"

description = "TODO: what the problem asks for."
approach = "TODO: the one-line idea behind the solution."

[[inputs]]
name = "nums"
label = "nums"
type = "int-array"
max_len = 40

[default_input]
nums = [1, 2, 3]

[[tests]]
expected = "TODO"
[tests.input]
nums = [1, 2, 3]
"#
        ),
    )?;

    std::fs::write(
        dir.join("code").join("go.txt"),
        "func solve(nums []int) int {\n    total := 0 //@init\n    for _, x := range nums { //@loop\n        total += x //@add\n    }\n    return total //@ret\n}\n",
    )?;

    std::fs::write(
        dir.join("trace.rhai"),
        r#"// Each step() names a //@tag from the sources, explains *why* in plain
// English, and hands the renderer the picture to draw.

fn trace(input) {
    let nums = input.nums;
    let total = 0;

    enter("solve", #{ nums: A(nums) });
    set(#{ total: N(total) });
    step("init", "Start the running total at 0.", [ array("nums", nums) ]);

    for i in 0..nums.len() {
        total += nums[i];
        set(#{ i: N(i), total: N(total) });
        step("loop", `i = ${i}: take nums[${i}] = ${nums[i]}.`,
            [ array("nums", nums).ptr("i", i).done(before(i)) ]);
        step("add", `Running total is now ${total}.`,
            [ array("nums", nums).ptr("i", i).hl([i]).done(before(i)) ]);
    }

    result(`${total}`);
    step("ret", `Every element folded in — the answer is ${total}.`,
        [ array("nums", nums).done(seq(0, nums.len() - 1)) ], "return");
    leave(`${total}`);
}
"#,
    )?;

    println!("scaffolded {}", dir.display());
    println!("next: add it to catalog.toml, then `cargo xtask trace {slug}`");
    Ok(())
}

fn cmd_package(args: &[String]) -> Result<()> {
    let out = args
        .iter()
        .position(|a| a == "--out")
        .and_then(|i| args.get(i + 1))
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_root().join("dist").join(target_dir_name()));

    let rust_dir = repo_root().join("rust");
    println!("building release binary...");
    let status = Command::new(env!("CARGO"))
        .current_dir(&rust_dir)
        .args(["build", "--release", "-p", "dsa-app"])
        .status()
        .context("failed to run cargo")?;
    if !status.success() {
        bail!("cargo build failed");
    }

    let exe_name = if cfg!(windows) {
        "dsa-visualized.exe"
    } else {
        "dsa-visualized"
    };
    let built = rust_dir.join("target").join("release").join(exe_name);
    if !built.exists() {
        bail!("expected binary at {}", built.display());
    }

    if out.exists() {
        std::fs::remove_dir_all(&out).ok();
    }
    std::fs::create_dir_all(&out)?;
    std::fs::copy(&built, out.join(exe_name))?;
    copy_dir(&content_root()?, &out.join("content"))?;
    for f in ["README.md", "LICENSE"] {
        let src = repo_root().join(f);
        if src.exists() {
            std::fs::copy(&src, out.join(f)).ok();
        }
    }

    let archive = args.iter().any(|a| a == "--archive");
    if archive {
        match make_archive(&out) {
            Ok(path) => println!("archive: {}", path.display()),
            Err(e) => println!("(archive skipped: {e})"),
        }
    }

    println!("packaged into {}", out.display());
    println!("  {exe_name} + content/ — the binary reads content next to itself");
    Ok(())
}

/// Zip on Windows, tar.gz elsewhere — both using tools that ship with the OS,
/// so packaging needs no extra crates or installs.
fn make_archive(dir: &Path) -> Result<PathBuf> {
    let parent = dir.parent().unwrap_or(Path::new("."));
    let name = dir.file_name().and_then(|s| s.to_str()).unwrap_or("dist");

    if cfg!(windows) {
        let out = parent.join(format!("{name}.zip"));
        let status = Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!(
                    "Compress-Archive -Path '{}\\*' -DestinationPath '{}' -Force",
                    dir.display(),
                    out.display()
                ),
            ])
            .status()?;
        if !status.success() {
            bail!("Compress-Archive failed");
        }
        Ok(out)
    } else {
        let out = parent.join(format!("{name}.tar.gz"));
        let status = Command::new("tar")
            .arg("-czf")
            .arg(&out)
            .arg("-C")
            .arg(parent)
            .arg(name)
            .status()?;
        if !status.success() {
            bail!("tar failed");
        }
        Ok(out)
    }
}

fn target_dir_name() -> String {
    let os = if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    };
    let arch = std::env::consts::ARCH;
    format!("{os}-{arch}")
}

fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest = to.join(entry.file_name());
        if ty.is_dir() {
            copy_dir(&entry.path(), &dest)?;
        } else if ty.is_file() {
            std::fs::copy(entry.path(), &dest)?;
        }
    }
    Ok(())
}
