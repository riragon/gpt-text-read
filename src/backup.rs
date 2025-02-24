use chrono::Local;
use std::fs;
use std::path::{Path, PathBuf};

use crate::models::ProjectOutput;

/// 読み込んだファイルのみを target/backup/日付時刻-[snapshot or comment]/ にコピーする関数
/// `folder_comment` が空でなければ、その文字列で "-snapshot" を置き換える。
pub fn backup_included_files(
    base_dir: &str,
    output: &ProjectOutput,
    folder_comment: &str,
) -> Result<PathBuf, String> {
    // バックアップ先フォルダ名を生成
    let now = Local::now();
    let date_str = now.format("%Y-%m-%d_%H%M%S").to_string();

    let snapshot_folder_name = if folder_comment.is_empty() {
        // 従来どおり "-snapshot"
        format!("{}-snapshot", date_str)
    } else {
        // コメントが入っている場合は "-コメント"
        format!("{}-{}", date_str, folder_comment)
    };

    let backup_path = Path::new(base_dir)
        .join("target")
        .join("backup")
        .join(snapshot_folder_name);

    // バックアップ先ディレクトリを作成
    fs::create_dir_all(&backup_path)
        .map_err(|e| format!("バックアップ先フォルダ作成に失敗しました: {}", e))?;

    // ProjectOutput.files に含まれるファイルのみをコピー
    for file_info in &output.files {
        let original_file_path = Path::new(&file_info.file_url);
        if original_file_path.exists() {
            let relative_path = match original_file_path.strip_prefix(base_dir) {
                Ok(p) => p,
                Err(_) => original_file_path, // strip_prefixできない場合はフルパスのまま
            };

            let dest_path = backup_path.join(relative_path);

            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent).map_err(|e| {
                    format!(
                        "コピー先ディレクトリの作成に失敗しました: {} (path: {:?})",
                        e, parent
                    )
                })?;
            }

            fs::copy(&original_file_path, &dest_path).map_err(|e| {
                format!(
                    "ファイルコピーに失敗しました: {} (元: {:?}, 先: {:?})",
                    e, original_file_path, dest_path
                )
            })?;
        }
    }

    Ok(backup_path)
}
