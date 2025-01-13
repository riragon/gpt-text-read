use std::fs;
use std::path::Path;
use walkdir::{WalkDir, DirEntry};
use regex::Regex;

use crate::models::FileInfo;

/// ファイルを集める関数（include/exclude対応）
pub fn collect_target_files(
    base_dir: &str,
    inc_patterns: &[Regex],
    exc_patterns: &[Regex],
) -> Result<Vec<FileInfo>, String> {
    let mut results = Vec::new();
    let base_path = Path::new(base_dir);

    for entry in WalkDir::new(base_path) {
        let e = entry.map_err(|e| e.to_string())?;
        if e.file_type().is_file() {
            let path = e.path();
            let filename = path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string();

            // include判定
            if is_in_patterns(&filename, inc_patterns) {
                // exclude判定
                if !is_in_patterns(&filename, exc_patterns) {
                    // ファイル読み込み
                    let content = fs::read_to_string(path)
                        .map_err(|err| format!("ファイル読み込みに失敗: {} ({})", err, filename))?;
                    let file_url = path.to_string_lossy().to_string();
                    results.push(FileInfo {
                        file_url,
                        file_name: filename,
                        file_content: content,
                    });
                }
            }
        }
    }
    Ok(results)
}

/// 正規表現パターンチェック関数
fn is_in_patterns(filename: &str, patterns: &[Regex]) -> bool {
    patterns.iter().any(|re| re.is_match(filename))
}

/// ディレクトリツリー生成
pub fn build_tree_view(base_dir: &str) -> String {
    // 除外したいディレクトリをリスト化
    const SKIP_DIRS: [&str; 2] = [".git", "target"];
    let mut lines = Vec::new();
    for entry in WalkDir::new(base_dir)
        .into_iter()
        .filter_entry(|e| should_show(e, &SKIP_DIRS))
    {
        if let Ok(e) = entry {
            let depth = e.depth();
            let file_name = e.file_name().to_string_lossy().to_string();
            if depth == 0 {
                lines.push(file_name);
            } else {
                let prefix = "  ".repeat(depth);
                lines.push(format!("{}{}", prefix, file_name));
            }
        }
    }
    lines.join("\n")
}

fn should_show(entry: &DirEntry, skip_dirs: &[&str]) -> bool {
    let file_name = entry.file_name().to_string_lossy();
    if skip_dirs.iter().any(|skip| file_name == *skip) {
        return false;
    }
    true
}
