//! Command safety policy and privacy-safe audit logging.

use anyhow::Result;
use serde_json::{json, Value};
use std::fs::OpenOptions;
use std::io::Write;

use super::runtime;

const BLOCKED_ERROR: &str = "PROGRAMMER_COMMAND_BLOCKED: command rejected by safety policy";

const ALWAYS_BLOCKED: &[(&str, &str)] = &[
    (":(){:|:&};:", "fork_bomb"),
    ("while(1){start powershell}", "fork_bomb"),
    ("format c:", "system_disk_format"),
    ("reg delete hklm", "critical_registry_delete"),
    (
        "reg delete hkcu\\software\\microsoft",
        "critical_registry_delete",
    ),
    ("cipher /w:", "secure_wipe"),
    ("bootrec /fixmbr", "boot_record_change"),
    ("bcdedit /delete", "boot_configuration_delete"),
];

const DISPOSABLE_COMPONENTS: &[&str] = &[
    "target",
    "build",
    "dist",
    "tmp",
    "temp",
    "node_modules",
    "__pycache__",
    ".cache",
];

#[derive(Debug, Clone, PartialEq, Eq)]
struct SafetyAssessment {
    safe: bool,
    reason_code: &'static str,
    message: Option<String>,
    severity: &'static str,
}

impl SafetyAssessment {
    fn allowed() -> Self {
        Self {
            safe: true,
            reason_code: "allowed",
            message: None,
            severity: "safe",
        }
    }

    fn blocked(reason_code: &'static str, message: &'static str) -> Self {
        Self {
            safe: false,
            reason_code,
            message: Some(format!("BLOCKED: {}", message)),
            severity: "critical",
        }
    }
}

/// Enforce the policy at an actual command execution boundary.
pub fn enforce_command_safety(command: &str, entrypoint: &str) -> Result<()> {
    enforce_with_auditor(command, entrypoint, |surface, reason| {
        append_blocked_audit(surface, reason)
    })
}

fn enforce_with_auditor<F>(command: &str, entrypoint: &str, mut audit: F) -> Result<()>
where
    F: FnMut(&str, &str),
{
    let assessment = assess_command(command);
    if assessment.safe {
        return Ok(());
    }

    audit(entrypoint, assessment.reason_code);
    anyhow::bail!(BLOCKED_ERROR)
}

fn assess_command(command: &str) -> SafetyAssessment {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return SafetyAssessment::allowed();
    }

    let lower = trimmed.to_ascii_lowercase();
    if has_encoded_command_switch(&lower) {
        return SafetyAssessment::blocked(
            "encoded_command",
            "encoded command payloads are not accepted",
        );
    }
    if lower.contains("frombase64string")
        && (lower.contains("iex") || lower.contains("invoke-expression"))
    {
        return SafetyAssessment::blocked(
            "encoded_command",
            "encoded command payloads are not accepted",
        );
    }

    for (pattern, reason_code) in ALWAYS_BLOCKED {
        if lower.contains(*pattern) {
            return SafetyAssessment::blocked(*reason_code, "critical system operation");
        }
    }

    if lower.contains("invoke-webrequest")
        && (lower.contains("|iex")
            || lower.contains("| iex")
            || lower.contains("invoke-expression"))
    {
        return SafetyAssessment::blocked("download_and_execute", "download-and-execute pipeline");
    }

    let deobfuscated_lower = lower.replace('^', "").replace('`', "");
    if !contains_destructive_verb(&lower) && !contains_destructive_verb(&deobfuscated_lower) {
        return SafetyAssessment::allowed();
    }

    if has_destructive_obfuscation(trimmed) {
        return SafetyAssessment::blocked(
            "obfuscated_destructive_command",
            "obfuscated destructive command",
        );
    }

    let clauses = match split_clauses(trimmed) {
        Some(clauses) => clauses,
        None => {
            return SafetyAssessment::blocked(
                "ambiguous_destructive_syntax",
                "ambiguous destructive command syntax",
            )
        }
    };

    let mut found_destructive_clause = false;
    for clause in clauses {
        let tokens = match tokenize(&clause) {
            Some(tokens) => tokens,
            None => {
                return SafetyAssessment::blocked(
                    "ambiguous_destructive_syntax",
                    "ambiguous destructive command syntax",
                )
            }
        };

        let Some((verb_index, verb)) = destructive_verb(&tokens) else {
            continue;
        };
        found_destructive_clause = true;
        let targets = extract_targets(&tokens[(verb_index + 1)..], verb);
        if targets.is_empty() {
            return SafetyAssessment::blocked(
                "ambiguous_destructive_target",
                "destructive target could not be determined",
            );
        }
        if targets.iter().any(|target| !is_disposable_target(target)) {
            return SafetyAssessment::blocked(
                "unsafe_destructive_target",
                "every destructive target must be inside an explicitly disposable directory",
            );
        }
    }

    if !found_destructive_clause {
        return SafetyAssessment::blocked(
            "ambiguous_destructive_syntax",
            "destructive command could not be parsed safely",
        );
    }
    SafetyAssessment::allowed()
}

fn has_encoded_command_switch(command: &str) -> bool {
    command.split_whitespace().any(|token| {
        let token = token.trim_matches(|ch: char| ch == '"' || ch == '\'');
        matches!(
            token,
            "-enc" | "-encodedcommand" | "/encodedcommand" | "--encoded-command"
        )
    })
}

fn contains_destructive_verb(command: &str) -> bool {
    command
        .split(|ch: char| ch.is_whitespace() || ";&|()\"'".contains(ch))
        .any(|token| {
            matches!(
                token.trim_matches(|ch: char| ch == ',' || ch == '.'),
                "rm" | "del" | "erase" | "rd" | "rmdir" | "remove-item"
            )
        })
}

fn has_destructive_obfuscation(command: &str) -> bool {
    let lower = command.to_ascii_lowercase();
    command.contains('`')
        || command.contains('^')
        || command.contains("$(")
        || command.contains('%')
        || command.contains('!')
        || lower.contains("frombase64string")
        || lower.contains("[char]")
        || lower.contains(" -join ")
}

fn split_clauses(command: &str) -> Option<Vec<String>> {
    let mut clauses = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let chars: Vec<char> = command.chars().collect();
    let mut index = 0;

    while index < chars.len() {
        let ch = chars[index];
        if let Some(active_quote) = quote {
            current.push(ch);
            if ch == active_quote {
                quote = None;
            }
            index += 1;
            continue;
        }
        if ch == '\'' || ch == '"' {
            quote = Some(ch);
            current.push(ch);
            index += 1;
            continue;
        }
        if ch == '|' {
            return None;
        }
        if ch == ';' || ch == '&' || ch == '\n' || ch == '\r' {
            if !current.trim().is_empty() {
                clauses.push(current.trim().to_string());
            }
            current.clear();
            if index + 1 < chars.len() && chars[index + 1] == ch {
                index += 1;
            }
        } else {
            current.push(ch);
        }
        index += 1;
    }

    if quote.is_some() {
        return None;
    }
    if !current.trim().is_empty() {
        clauses.push(current.trim().to_string());
    }
    Some(clauses)
}

fn tokenize(clause: &str) -> Option<Vec<String>> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    for ch in clause.chars() {
        if let Some(active_quote) = quote {
            if ch == active_quote {
                quote = None;
            } else {
                current.push(ch);
            }
            continue;
        }
        if ch == '\'' || ch == '"' {
            quote = Some(ch);
        } else if ch.is_whitespace() {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
        } else {
            current.push(ch);
        }
    }
    if quote.is_some() {
        return None;
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    Some(tokens)
}

fn destructive_verb(tokens: &[String]) -> Option<(usize, &str)> {
    for (index, token) in tokens.iter().enumerate() {
        let normalized = token
            .trim_matches(|ch: char| ch == ',' || ch == '.')
            .to_ascii_lowercase();
        if !matches!(
            normalized.as_str(),
            "rm" | "del" | "erase" | "rd" | "rmdir" | "remove-item"
        ) {
            continue;
        }
        if index == 0
            || tokens[..index]
                .iter()
                .all(|prefix| is_wrapper_token(prefix))
        {
            let verb = match normalized.as_str() {
                "remove-item" => "remove-item",
                "del" | "erase" => "del",
                "rd" | "rmdir" => "rmdir",
                _ => "rm",
            };
            return Some((index, verb));
        }
    }
    None
}

fn is_wrapper_token(token: &str) -> bool {
    matches!(
        token.to_ascii_lowercase().as_str(),
        "cmd"
            | "cmd.exe"
            | "/c"
            | "call"
            | "powershell"
            | "powershell.exe"
            | "pwsh"
            | "pwsh.exe"
            | "-command"
            | "bash"
            | "sh"
            | "-c"
            | "wsl"
            | "wsl.exe"
            | "--"
            | "sudo"
    )
}

fn extract_targets(tokens: &[String], verb: &str) -> Vec<String> {
    let mut targets = Vec::new();
    for token in tokens {
        if token == "--" || is_delete_option(token, verb) {
            continue;
        }
        for candidate in token.split(',') {
            let candidate = candidate.trim();
            if !candidate.is_empty() {
                targets.push(candidate.to_string());
            }
        }
    }
    targets
}

fn is_delete_option(token: &str, verb: &str) -> bool {
    let lower = token.to_ascii_lowercase();
    match verb {
        "rm" | "remove-item" => lower.starts_with('-'),
        "del" | "rmdir" => {
            matches!(lower.as_str(), "/s" | "/q" | "/f" | "/p" | "/?") || lower.starts_with("/a")
        }
        _ => false,
    }
}

fn is_disposable_target(raw_target: &str) -> bool {
    let target = raw_target
        .trim()
        .trim_matches(|ch: char| ch == '"' || ch == '\'' || ch == ',' || ch == ';');
    if target.is_empty()
        || matches!(target, "." | ".." | "/" | "\\")
        || target.contains('$')
        || target.contains('%')
        || target.contains('!')
        || target.contains('`')
        || target.contains('{')
        || target.contains('}')
        || target.contains("$(")
    {
        return false;
    }

    let normalized = target.replace('\\', "/");
    if normalized == "*"
        || normalized == "/*"
        || (normalized.len() == 3 && normalized.as_bytes()[1] == b':' && normalized.ends_with('/'))
    {
        return false;
    }
    let components: Vec<&str> = normalized
        .split('/')
        .filter(|component| !component.is_empty() && *component != ".")
        .collect();
    if components.is_empty() || components.iter().any(|component| *component == "..") {
        return false;
    }
    components.iter().any(|component| {
        let clean = component.trim_end_matches('*').to_ascii_lowercase();
        DISPOSABLE_COMPONENTS.contains(&clean.as_str())
    })
}

fn audit_log_path() -> std::path::PathBuf {
    runtime::state_path("security").join("audit.jsonl")
}

fn append_blocked_audit(entrypoint: &str, reason_code: &str) {
    let path = audit_log_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let safe_entrypoint: String = entrypoint
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == '-')
        .take(64)
        .collect();
    let record = json!({
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "event": "command_blocked",
        "entrypoint": safe_entrypoint,
        "reason_code": reason_code,
    });
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{}", record);
    }
}

pub async fn check_command(args: Value) -> Result<Value> {
    let command = args["command"].as_str().unwrap_or("");
    let assessment = assess_command(command);
    Ok(json!({
        "safe": assessment.safe,
        "severity": assessment.severity,
        "warning": assessment.message,
        "reason_code": assessment.reason_code,
        "command": command
    }))
}

pub async fn audit_log(args: Value) -> Result<Value> {
    let lines = args["lines"].as_u64().unwrap_or(20) as usize;
    let path = audit_log_path();
    match std::fs::read_to_string(&path) {
        Ok(content) => {
            let entries: Vec<&str> = content.lines().rev().take(lines).collect();
            Ok(json!({ "entries": entries, "count": entries.len(), "log_path": path }))
        }
        Err(_) => {
            Ok(json!({ "entries": [], "count": 0, "log_path": path, "note": "No audit log yet" }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{assess_command, enforce_with_auditor, BLOCKED_ERROR};

    #[test]
    fn allows_each_disposable_target() {
        for command in [
            "rm -rf target",
            "rm -rf ./build/output",
            "rmdir /s /q dist",
            "del /s /q tmp\\*",
            "Remove-Item -Recurse -Force .cache",
            "rm -rf node_modules __pycache__ temp",
        ] {
            assert!(assess_command(command).safe, "should allow: {command}");
        }
    }

    #[test]
    fn blocks_mixed_safe_and_unsafe_targets() {
        for command in [
            "rm -rf target important",
            "rmdir /s /q build C:\\Users",
            "Remove-Item -Recurse dist,documents",
        ] {
            let assessment = assess_command(command);
            assert!(!assessment.safe, "should block: {command}");
            assert_eq!(assessment.reason_code, "unsafe_destructive_target");
        }
    }

    #[test]
    fn blocks_roots_parents_ambiguous_and_obfuscated_deletes() {
        for command in [
            "rm -rf /",
            "rm -rf .",
            "rm -rf ../target",
            "rd /s /q C:\\",
            "rm -rf $TARGET",
            "rm -rf target | echo hidden",
            "cmd /c r^mdir /s /q target",
            "powershell -EncodedCommand ZABlAGwA",
        ] {
            assert!(!assess_command(command).safe, "should block: {command}");
        }
    }

    #[test]
    fn ordinary_non_destructive_commands_remain_allowed() {
        for command in ["cargo test", "git status", "echo target", "Get-ChildItem ."] {
            assert!(assess_command(command).safe, "should allow: {command}");
        }
    }

    #[test]
    fn enforcement_returns_deterministic_error_and_invokes_auditor() {
        let mut audit = None;
        let error = enforce_with_auditor("rm -rf target documents", "cmd", |surface, reason| {
            audit = Some((surface.to_string(), reason.to_string()));
        })
        .expect_err("mixed target must be blocked");

        assert_eq!(error.to_string(), BLOCKED_ERROR);
        assert_eq!(
            audit,
            Some(("cmd".to_string(), "unsafe_destructive_target".to_string()))
        );
    }
}
