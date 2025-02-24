use std::cell::RefCell;
use std::collections::HashSet;
use std::path::Path;
use std::rc::Rc;

use chrono::Local;
use fltk::prelude::*;
use fltk::{app, dialog::{choice2, input}};
use regex::Regex;

use crate::backup::backup_included_files;
use crate::fileops::{collect_target_files, build_tree_view};
use crate::models::ProjectOutput;
use crate::settings::{load_settings, write_settings};
use crate::ui::{UiMessage, build_ui, GuiComponents};

/// アプリ全体でやり取りするデータ
pub struct AppData {
    pub selected_project_dir: RefCell<Option<String>>,
    pub current_output_path: RefCell<Option<String>>,
    pub loaded_output: RefCell<Option<ProjectOutput>>,
}

/// アプリを起動する
pub fn run_app() {
    let app = app::App::default();

    // AppData は mutable でなくても良い
    let app_data = Rc::new(AppData {
        selected_project_dir: RefCell::new(None),
        current_output_path: RefCell::new(None),
        loaded_output: RefCell::new(None),
    });

    let mut gui = build_ui(app_data.clone());

    gui.win.show();

    while app.wait() {
        if let Some(msg) = gui.receiver.recv() {
            match msg {
                UiMessage::SelectProject => {
                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                        let folder_path = folder.to_string_lossy().to_string();
                        *app_data.selected_project_dir.borrow_mut() = Some(folder_path.clone());
                        append_log(&gui, &format!("プロジェクト選択: {}", folder_path));

                        // 設定ファイル読み込み
                        let loaded = load_settings(&folder_path);

                        // パターン等をGUIへ反映
                        gui.include_input.borrow_mut()
                            .set_value(&loaded.patterns_include.join("\n"));
                        gui.exclude_input.borrow_mut()
                            .set_value(&loaded.patterns_exclude.join("\n"));
                        gui.dev_memo_buffer.borrow_mut()
                            .set_text(&loaded.dev_memo.join("\n"));

                        // LLM補足をGUIに反映
                        gui.llm_buffer.borrow_mut()
                            .set_text(&loaded.llm_note.join("\n"));

                        // 出力先フォルダ
                        *app_data.current_output_path.borrow_mut() = loaded.output_path;

                        gui.sender.send(UiMessage::StartLoad);
                    }
                }

                UiMessage::AddFile => {
                    if let Some(paths) = rfd::FileDialog::new().set_directory(".").pick_files() {
                        append_log(&gui, "ファイル追加開始");

                        let mut input = gui.include_input.borrow_mut();
                        let mut current_text = input.value();

                        let mut existing_patterns: HashSet<String> = current_text
                            .lines()
                            .map(|s| s.trim().to_string())
                            .collect();

                        if !current_text.is_empty() && !current_text.ends_with('\n') {
                            current_text.push('\n');
                        }

                        let base_dir_opt = app_data.selected_project_dir.borrow().clone();
                        for path in paths {
                            if path.is_file() {
                                let pattern = if let Some(ref base_str) = base_dir_opt {
                                    let base_path = Path::new(base_str);
                                    if let Ok(rel) = path.strip_prefix(base_path) {
                                        let rel_str = rel.to_string_lossy().replace("\\", "/");
                                        format!("^{}$", regex::escape(&rel_str))
                                    } else {
                                        let fname = path.file_name().unwrap().to_string_lossy().to_string();
                                        format!("^{}$", regex::escape(&fname))
                                    }
                                } else {
                                    let fname = path.file_name().unwrap().to_string_lossy().to_string();
                                    format!("^{}$", regex::escape(&fname))
                                };

                                if !existing_patterns.contains(&pattern) {
                                    existing_patterns.insert(pattern.clone());
                                    current_text.push_str(&pattern);
                                    current_text.push('\n');
                                }
                            } else if path.is_dir() {
                                let pattern = if let Some(ref base_str) = base_dir_opt {
                                    let base_path = Path::new(base_str);
                                    if let Ok(rel) = path.strip_prefix(base_path) {
                                        let rel_str = rel.to_string_lossy().replace("\\", "/");
                                        format!("^{}.*$", regex::escape(&rel_str))
                                    } else {
                                        let dname = path.file_name().unwrap().to_string_lossy().to_string();
                                        format!("^{}.*$", regex::escape(&dname))
                                    }
                                } else {
                                    let dname = path.file_name().unwrap().to_string_lossy().to_string();
                                    format!("^{}.*$", regex::escape(&dname))
                                };

                                if !existing_patterns.contains(&pattern) {
                                    existing_patterns.insert(pattern.clone());
                                    current_text.push_str(&pattern);
                                    current_text.push('\n');
                                }
                            }
                        }

                        input.set_value(&current_text);

                        gui.sender.send(UiMessage::SaveSettings);
                        gui.sender.send(UiMessage::StartLoad);

                        append_log(&gui, "ファイル追加完了");
                    }
                }

                UiMessage::ExcludeFolder => {
                    if let Some(folder_path) = rfd::FileDialog::new().set_directory(".").pick_folder() {
                        append_log(&gui, &format!("フォルダ除外指定: {:?}", folder_path));
                        let mut input = gui.exclude_input.borrow_mut();
                        let mut current_text = input.value();
                        if !current_text.is_empty() && !current_text.ends_with('\n') {
                            current_text.push('\n');
                        }
                        if let Some(dir_name) = folder_path.file_name() {
                            let dname = dir_name.to_string_lossy().to_string();
                            let escaped = regex::escape(&dname);
                            let pattern = format!("^{}/.*$", escaped);
                            current_text.push_str(&pattern);
                            current_text.push('\n');
                        }
                        input.set_value(&current_text);

                        gui.sender.send(UiMessage::SaveSettings);
                        gui.sender.send(UiMessage::StartLoad);
                    }
                }

                UiMessage::SaveSettings => {
                    if let Some(dir) = &*app_data.selected_project_dir.borrow() {
                        let inc_text = gui.include_input.borrow().value();
                        let inc_patterns: Vec<&str> = inc_text
                            .lines()
                            .map(|s| s.trim())
                            .filter(|s| !s.is_empty())
                            .collect();

                        let exc_text = gui.exclude_input.borrow().value();
                        let exc_patterns: Vec<&str> = exc_text
                            .lines()
                            .map(|s| s.trim())
                            .filter(|s| !s.is_empty())
                            .collect();

                        let dev_text = gui.dev_memo_buffer.borrow().text();
                        let dev_lines: Vec<&str> = dev_text.lines().collect();

                        // LLM補足
                        let llm_text = gui.llm_buffer.borrow().text();
                        let llm_lines: Vec<&str> = llm_text.lines().collect();

                        let output_dir_opt = app_data.current_output_path.borrow().clone();

                        if let Err(e) = write_settings(
                            dir,
                            &inc_patterns,
                            &exc_patterns,
                            &output_dir_opt,
                            &dev_lines,
                            &llm_lines,
                        ) {
                            alert_default(&format!("設定保存に失敗しました: {}", e));
                            append_log(&gui, &format!("設定保存エラー: {}", e));
                        } else {
                            append_log(&gui, "設定ファイルを保存しました。");
                        }
                    }
                }

                UiMessage::StartLoad => {
                    append_log(&gui, "ファイル読み込み開始");
                    let dir_opt = app_data.selected_project_dir.borrow().clone();
                    let inc_text = gui.include_input.borrow().value();
                    let exc_text = gui.exclude_input.borrow().value();
                    let tree_on = *gui.tree_check_state.borrow();
                    let sender = gui.sender.clone();

                    std::thread::spawn(move || {
                        if let Some(dir) = dir_opt {
                            let inc_patterns: Vec<Regex> = inc_text
                                .lines()
                                .map(|s| s.trim())
                                .filter(|s| !s.is_empty())
                                .filter_map(|p| Regex::new(p).ok())
                                .collect();

                            let exc_patterns: Vec<Regex> = exc_text
                                .lines()
                                .map(|s| s.trim())
                                .filter(|s| !s.is_empty())
                                .filter_map(|p| Regex::new(p).ok())
                                .collect();

                            match collect_target_files(&dir, &inc_patterns, &exc_patterns) {
                                Ok(files) => {
                                    let tree_view = if tree_on {
                                        Some(build_tree_view(&dir, &exc_patterns))
                                    } else {
                                        None
                                    };
                                    // llm_note は後でUIスレッド側で代入する
                                    let output = ProjectOutput {
                                        files,
                                        tree_view,
                                        llm_note: None,
                                    };
                                    sender.send(UiMessage::LoadFinished(Ok(output)));
                                }
                                Err(e) => {
                                    sender.send(UiMessage::LoadFinished(Err(e)));
                                }
                            }
                        }
                    });
                }

                UiMessage::LoadFinished(result) => {
                    match result {
                        Ok(mut output) => {
                            let llm_txt = gui.llm_buffer.borrow().text();
                            output.llm_note = Some(llm_txt);

                            let json_str = match serde_json::to_string_pretty(&output) {
                                Ok(js) => js,
                                Err(e) => {
                                    alert_default(&format!("JSON変換に失敗: {}", e));
                                    append_log(&gui, &format!("JSON変換エラー: {}", e));
                                    continue;
                                }
                            };
                            gui.json_buffer.borrow_mut().set_text(&json_str);

                            // 全ファイルの内容をまとめたテキスト
                            let mut all_text = String::new();
                            for file_info in &output.files {
                                all_text.push_str("File: ");
                                all_text.push_str(&file_info.file_name);
                                all_text.push('\n');
                                all_text.push_str(&file_info.file_content);
                                all_text.push_str("\n--------------------------------\n");
                            }
                            gui.chosen_file_buffer.borrow_mut().set_text(&all_text);

                            // ツリー
                            if let Some(tv) = &output.tree_view {
                                gui.tree_buffer.borrow_mut().set_text(tv);
                            } else {
                                gui.tree_buffer.borrow_mut().set_text("");
                            }

                            let size = json_str.len();
                            gui.sender.send(UiMessage::UpdateCopySize(size));

                            *app_data.loaded_output.borrow_mut() = Some(output);

                            append_log(&gui, "ファイル読み込み完了");
                        }
                        Err(e) => {
                            alert_default(&format!("読み込みエラー: {}", e));
                            append_log(&gui, &format!("読み込みエラー: {}", e));
                        }
                    }
                }

                UiMessage::Copy => {
                    let val = gui.json_buffer.borrow().text();
                    app::copy(&val);
                    gui.sender.send(UiMessage::UpdateCopySize(val.len()));
                    append_log(&gui, &format!("JSONコピー ({} bytes)", val.len()));
                }

                UiMessage::UpdateCopySize(size) => {
                    gui.copy_size_label.set_label(&format!("Copy Size: {}", size));
                }

                UiMessage::ExportTxt => {
                    let val = gui.json_buffer.borrow().text();
                    if val.is_empty() {
                        alert_default("JSONが空です。先に読み込み実行してください。");
                        append_log(&gui, "テキスト出力失敗：JSONが空");
                        continue;
                    }

                    let current_dir_opt = app_data.current_output_path.borrow().clone();
                    let dir_opt = app_data.selected_project_dir.borrow().clone();
                    let now = Local::now();

                    let dialog_dir = if let Some(op) = current_dir_opt {
                        op
                    } else if let Some(proj) = dir_opt.clone() {
                        proj
                    } else {
                        ".".to_string()
                    };

                    let time_str = now.format("%Y%m%d_%H%M%S").to_string();
                    let default_file_name = if let Some(proj_dir) = &dir_opt {
                        let folder_name = Path::new(proj_dir)
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                        format!("{}_{}.txt", folder_name, time_str)
                    } else {
                        format!("output_{}.txt", time_str)
                    };

                    if let Some(chosen_path) = rfd::FileDialog::new()
                        .set_directory(&dialog_dir)
                        .set_file_name(&default_file_name)
                        .save_file()
                    {
                        if let Some(parent) = chosen_path.parent() {
                            if let Err(e) = std::fs::create_dir_all(parent) {
                                alert_default(&format!("フォルダ作成失敗: {}", e));
                                append_log(&gui, &format!("フォルダ作成失敗: {}", e));
                                continue;
                            }
                        }

                        let total_size = val.len();
                        const CHUNK_LIMIT: usize = 50_000;

                        // ファイル先頭コメント
                        let label = {
                            let project_name = match dir_opt.as_ref() {
                                Some(d) => {
                                    Path::new(d)
                                        .file_name()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                        .to_string()
                                },
                                None => "NoProject".to_string(),
                            };
                            let date_str = now.format("%Y-%m-%d %H:%M:%S").to_string();
                            format!("// Project: {}, Date: {}\n", project_name, date_str)
                        };

                        // LLM補足
                        let llm_raw = gui.llm_buffer.borrow().text();

                        let labelled_val = format!(
                            "// LLM補足:\n{}\n\n{}{}\n// End of chunk.\n",
                            llm_raw,
                            label,
                            val
                        );

                        if total_size > CHUNK_LIMIT {
                            let choice = choice2(
                                0,
                                0,
                                &format!(
                                    "JSONサイズが大きいです ({} bytes)。\nチャンク分割しますか？",
                                    total_size
                                ),
                                "Yes", "No", ""
                            );

                            match choice {
                                Some(0) => {
                                    // = Yes => 分割
                                    match split_into_chunks(&labelled_val, CHUNK_LIMIT) {
                                        Ok(chunks) => {
                                            let stem = chosen_path.file_stem()
                                                .unwrap_or_default()
                                                .to_string_lossy()
                                                .to_string();
                                            let ext = chosen_path
                                                .extension()
                                                .unwrap_or_default()
                                                .to_string_lossy()
                                                .to_string();
                                            for (i, ch) in chunks.iter().enumerate() {
                                                let chunk_file_name = format!("{}_chunk_{}.{}", stem, i+1, ext);
                                                let chunk_path = chosen_path
                                                    .parent().unwrap_or_else(|| Path::new("."))
                                                    .join(&chunk_file_name);

                                                if let Err(e) = std::fs::write(&chunk_path, ch) {
                                                    alert_default(&format!(
                                                        "チャンクファイル書き込み失敗: {} ({})",
                                                        e, chunk_path.display()
                                                    ));
                                                    append_log(&gui, &format!(
                                                        "チャンクファイル書き込み失敗: {} ({})",
                                                        e, chunk_path.display()
                                                    ));
                                                }
                                            }
                                            append_log(&gui, "テキスト出力(チャンク分割)完了");
                                        },
                                        Err(e) => {
                                            alert_default(&format!("チャンク分割エラー: {}", e));
                                            append_log(&gui, &format!("チャンク分割エラー: {}", e));
                                        }
                                    }
                                },
                                Some(1) => {
                                    // = No => 単一ファイル
                                    if let Err(e) = std::fs::write(&chosen_path, labelled_val.as_bytes()) {
                                        alert_default(&format!("書き込み失敗: {}", e));
                                        append_log(&gui, &format!("テキスト書き込み失敗: {}", e));
                                        continue;
                                    }
                                    append_log(&gui, &format!("テキスト出力完了: {}", chosen_path.display()));
                                },
                                _ => { /* キャンセル/クローズ => 何もしない */ }
                            }
                        } else {
                            // チャンク不要
                            if let Err(e) = std::fs::write(&chosen_path, labelled_val.as_bytes()) {
                                alert_default(&format!("書き込み失敗: {}", e));
                                append_log(&gui, &format!("テキスト書き込み失敗: {}", e));
                                continue;
                            }
                            append_log(&gui, &format!("テキスト出力完了: {}", chosen_path.display()));
                        }

                        // 出力先を記憶
                        if let Some(parent_dir) = chosen_path.parent() {
                            let new_path_str = parent_dir.to_string_lossy().to_string();
                            *app_data.current_output_path.borrow_mut() = Some(new_path_str);
                        }

                        // OUTPUT_PATH保存
                        let inc_text = gui.include_input.borrow().value();
                        let exc_text = gui.exclude_input.borrow().value();
                        let dev_text = gui.dev_memo_buffer.borrow().text();
                        let inc_patterns: Vec<&str> = inc_text
                            .lines()
                            .map(|s| s.trim())
                            .filter(|s| !s.is_empty())
                            .collect();
                        let exc_patterns: Vec<&str> = exc_text
                            .lines()
                            .map(|s| s.trim())
                            .filter(|s| !s.is_empty())
                            .collect();
                        let dev_lines: Vec<&str> = dev_text.lines().collect();

                        let llm_text = gui.llm_buffer.borrow().text();
                        let llm_lines: Vec<&str> = llm_text.lines().collect();

                        if let Some(proj_dir) = &*app_data.selected_project_dir.borrow() {
                            if let Err(e) = write_settings(
                                proj_dir,
                                &inc_patterns,
                                &exc_patterns,
                                &app_data.current_output_path.borrow(),
                                &dev_lines,
                                &llm_lines,
                            ) {
                                alert_default(&format!("OUTPUT_PATHの設定保存に失敗: {}", e));
                                append_log(&gui, &format!("OUTPUT_PATHの設定保存に失敗: {}", e));
                            }
                        }
                    }
                }

                UiMessage::Backup => {
                    if let Some(base_dir) = &*app_data.selected_project_dir.borrow() {
                        if let Some(ref output) = *app_data.loaded_output.borrow() {
                            let input_str_opt = input(
                                0,
                                0,
                                "スナップショットフォルダに付加する英数字コメント（空欄可）を入力してください",
                                ""
                            );
                            if input_str_opt.is_none() {
                                continue;
                            }
                            let mut folder_comment = String::new();
                            let trimmed = input_str_opt.unwrap().trim().to_string();
                            if !trimmed.is_empty() {
                                let re_alnum = Regex::new("^[A-Za-z0-9]+$").unwrap();
                                if re_alnum.is_match(&trimmed) {
                                    folder_comment = trimmed;
                                } else {
                                    alert_default("英数字のみを入力してください。コメントは無視されます。");
                                    append_log(&gui, "スナップショットコメントが英数字以外 → 無視");
                                }
                            }

                            match backup_included_files(base_dir, output, &folder_comment) {
                                Ok(dest) => {
                                    append_log(&gui, &format!("スナップショット作成完了: {}", dest.display()));
                                }
                                Err(e) => {
                                    alert_default(&format!("バックアップ失敗: {}", e));
                                    append_log(&gui, &format!("バックアップ失敗: {}", e));
                                }
                            }
                        } else {
                            alert_default("まだファイルが読み込まれていません。");
                            append_log(&gui, "バックアップ失敗：ファイル未読み込み");
                        }
                    } else {
                        alert_default("プロジェクトフォルダが選択されていません。");
                        append_log(&gui, "バックアップ失敗：プロジェクト未選択");
                    }
                }
            }
        }
    }
}

/// バイト数でナイーブに分割
fn split_into_chunks(text: &str, chunk_size: usize) -> Result<Vec<String>, String> {
    if chunk_size == 0 {
        return Err("チャンクサイズが0です。".to_string());
    }
    let bytes = text.as_bytes();
    let mut chunks = Vec::new();
    let mut start = 0;
    while start < bytes.len() {
        let end = (start + chunk_size).min(bytes.len());
        let slice = &bytes[start..end];
        chunks.push(String::from_utf8_lossy(slice).to_string());
        start += chunk_size;
    }
    Ok(chunks)
}

fn alert_default(msg: &str) {
    fltk::dialog::alert(0, 0, msg);
}

/// ログテキストバッファにメッセージを追記する補助関数
fn append_log(gui: &GuiComponents, msg: &str) {
    let mut buffer = gui.log_buffer.borrow_mut();
    buffer.append(msg);
    buffer.append("\n");
}
