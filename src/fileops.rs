use std::fs;
use std::path::Path;
use walkdir::{WalkDir, DirEntry};
use regex::Regex;

use crate::models::FileInfo;

/// ファイルを集める関数（include/exclude対応＋target/backup強制除外）
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

            // 相対パスを生成（Windowsの '\\' → '/' に置換）
            let rel_path_str = match path.strip_prefix(base_path) {
                Ok(p) => p.to_string_lossy().replace("\\", "/"),
                Err(_) => path.to_string_lossy().replace("\\", "/"),
            };

            // ① target/backup を含むパスは強制除外（入れ子防止）
            if rel_path_str.contains("target/backup") {
                continue;
            }

            // ② includeパターン / excludeパターン判定
            //    → "src/backup.rs" のような文字列に対してマッチを行う
            if is_in_patterns(&rel_path_str, inc_patterns) {
                if !is_in_patterns(&rel_path_str, exc_patterns) {
                    // ファイル読み込み
                    let content = fs::read_to_string(path)
                        .map_err(|err| format!("ファイル読み込みに失敗: {} ({})", err, rel_path_str))?;

                    // 結果に追加
                    results.push(FileInfo {
                        file_url: path.to_string_lossy().to_string(),
                        file_name: rel_path_str.clone(),
                        file_content: content,
                    });
                }
            }
        }
    }
    Ok(results)
}

/// 正規表現パターンチェック関数
fn is_in_patterns(text: &str, patterns: &[Regex]) -> bool {
    patterns.iter().any(|re| re.is_match(text))
}

/// ディレクトリツリー生成
///
/// `exc_patterns` がフォルダパスにマッチした場合は、そのフォルダ以下をツリー表示に含めない。
pub fn build_tree_view(base_dir: &str, exc_patterns: &[Regex]) -> String {
    let mut lines = Vec::new();
    for entry in WalkDir::new(base_dir)
        .into_iter()
        .filter_entry(|e| should_show(e, exc_patterns, base_dir))
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

/// ツリー表示対象にするかどうか判定するフィルタ関数
fn should_show(entry: &DirEntry, exc_patterns: &[Regex], base_dir: &str) -> bool {
    let base_path = Path::new(base_dir);

    // ベースディレクトリからの相対パスを取得（Windowsの'\\' → '/'に置き換え）
    let rel_path = match entry.path().strip_prefix(base_path) {
        Ok(p) => p.to_string_lossy().replace("\\", "/"),
        Err(_) => entry.path().display().to_string().replace("\\", "/"),
    };

    // target/backup を含むパスは強制除外
    if rel_path.contains("target/backup") {
        return false;
    }

    // 除外パターンのいずれかにマッチしたら、このディレクトリ(またはファイル)以下は表示しない
    if exc_patterns.iter().any(|re| re.is_match(&rel_path)) {
        return false;
    }

    true
}
