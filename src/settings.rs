use std::{
    fs::File,
    io::{BufRead, BufReader, Write},
    path::Path,
};

use crate::models::LoadedSettings;

/// 設定ファイルの読み込み
pub fn load_settings(base_dir: &str) -> LoadedSettings {
    let settings_path = Path::new(base_dir).join("text-read-settings.txt");
    let mut patterns_include = Vec::new();
    let mut patterns_exclude = Vec::new();
    let mut output_path: Option<String> = None;
    let mut dev_memo = Vec::new();
    let mut llm_note = Vec::new(); // ← LLM補足

    if settings_path.exists() {
        if let Ok(file) = File::open(settings_path) {
            for line in BufReader::new(file).lines() {
                if let Ok(raw_line) = line {
                    let trimmed = raw_line.trim();
                    if trimmed.is_empty() || trimmed.starts_with('#') {
                        continue;
                    }
                    if let Some(rest) = trimmed.strip_prefix("OUTPUT_PATH=") {
                        let val = rest.trim();
                        if !val.is_empty() {
                            output_path = Some(val.to_string());
                        }
                    }
                    else if let Some(rest) = trimmed.strip_prefix("EXCLUDE:") {
                        let val = rest.trim();
                        if !val.is_empty() {
                            patterns_exclude.push(val.to_string());
                        }
                    }
                    else if let Some(rest) = trimmed.strip_prefix("DEVNOTE:") {
                        dev_memo.push(rest.to_string());
                    }
                    // ★ LLM補足行
                    else if let Some(rest) = trimmed.strip_prefix("LLMNOTE:") {
                        llm_note.push(rest.to_string());
                    }
                    else {
                        // 上記以外はinclude扱い
                        patterns_include.push(trimmed.to_string());
                    }
                }
            }
        }
    }

    LoadedSettings {
        patterns_include,
        patterns_exclude,
        output_path,
        dev_memo,
        llm_note,
    }
}

/// 設定ファイルの書き込み
pub fn write_settings(
    project_dir: &str,
    include_patterns: &[&str],
    exclude_patterns: &[&str],
    output_path: &Option<String>,
    dev_memo: &[&str],
    llm_note: &[&str], // ← LLM補足
) -> Result<(), String> {
    let settings_path = Path::new(project_dir).join("text-read-settings.txt");

    let mut file = File::create(&settings_path)
        .map_err(|e| format!("設定ファイル作成に失敗: {}", e))?;

    // 1) OUTPUT_PATH
    if let Some(op) = output_path {
        let line = format!("OUTPUT_PATH={}\n\n", op);
        file.write_all(line.as_bytes())
            .map_err(|e| format!("OUTPUT_PATH書き込み失敗: {}", e))?;
    }

    // 2) includeパターン
    for pat in include_patterns {
        if let Err(e) = writeln!(file, "{}", pat) {
            return Err(format!("Includeパターン書き込みに失敗: {}", e));
        }
    }

    // 3) excludeパターン（行頭に "EXCLUDE:" を付加）
    for pat in exclude_patterns {
        if let Err(e) = writeln!(file, "EXCLUDE:{}", pat) {
            return Err(format!("Excludeパターン書き込みに失敗: {}", e));
        }
    }

    // 4) 開発メモ
    for line in dev_memo {
        if let Err(e) = writeln!(file, "DEVNOTE:{}", line) {
            return Err(format!("開発メモ書き込みに失敗: {}", e));
        }
    }

    // 5) LLM補足
    for line in llm_note {
        if let Err(e) = writeln!(file, "LLMNOTE:{}", line) {
            return Err(format!("LLM補足書き込みに失敗: {}", e));
        }
    }

    Ok(())
}
