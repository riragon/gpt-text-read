use serde::Serialize;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use fltk::{
    app, prelude::*,
    window::Window, button::Button, input::MultilineInput, group::Flex,
    text::{TextEditor, TextBuffer, WrapMode},
    frame::Frame,
    dialog::alert
};
use std::rc::Rc;
use std::cell::RefCell;
use walkdir::WalkDir;
use regex::Regex;
// chronoを使って時刻を取得
use chrono::Local;

#[derive(Serialize, Debug)]
struct FileInfo {
    file_url: String,
    file_name: String,
    file_content: String,
}

#[derive(Serialize, Debug)]
struct ProjectOutput {
    files: Vec<FileInfo>,
}

// メッセージ(enum)でGUIイベントやバックグラウンド処理結果をやり取り
#[derive(Debug)]
enum Message {
    SelectProject,
    AddFile,
    SaveSettings,
    StartLoad,
    LoadFinished(Result<ProjectOutput, String>),
    Copy,
    UpdateCopySize(usize),
    ExportTxt,
}

// 設定ファイルのロード結果をまとめる
// patterns: 正規表現パターン一覧
// output_path: テキスト出力先フォルダ (ユーザーがファイル保存ダイアログで指定したフォルダ)
struct LoadedSettings {
    patterns: Vec<String>,
    output_path: Option<String>, // 例: "C:/Users/xxx/Desktop"
}

fn main() {
    let app = app::App::default();
    let mut win = Window::new(100, 100, 950, 600, "Text-Read with Settings (Async)");
    
    // メッセージチャネル (sender, receiver)
    let (s, r) = app::channel::<Message>();

    let mut main_flex = Flex::default().size_of_parent().column();
    main_flex.set_margin(10);

    // パターン入力 (正規表現パターン)
    let pattern_input = Rc::new(RefCell::new(MultilineInput::new(0, 0, 0, 0, "")));
    pattern_input.borrow_mut().set_readonly(false);
    {
        let pi = pattern_input.borrow();
        main_flex.fixed(&*pi, 100);
    }

    // ボタン行
    let mut btn_flex = Flex::default().row();
    btn_flex.set_spacing(10);

    let mut project_btn = Button::default().with_label("プロジェクト選択");
    let mut add_file_btn = Button::default().with_label("ファイル追加");
    let mut save_btn = Button::default().with_label("設定保存");
    let mut load_btn = Button::default().with_label("読み込み実行");
    let mut copy_btn = Button::default().with_label("コピー");
    let mut export_btn = Button::default().with_label("テキスト出力");

    let mut copy_size_label = Frame::default().with_label("Copy Size: 0");

    btn_flex.end();
    main_flex.fixed(&btn_flex, 40);

    // 中段Flex：選択ファイル内容表示とJSON表示を上下に並べる
    let mid_flex = Flex::default().column();

    let chosen_file_buffer = Rc::new(RefCell::new(TextBuffer::default()));
    let mut chosen_file_editor = TextEditor::new(0, 0, 0, 0, "");
    chosen_file_editor.set_buffer(chosen_file_buffer.borrow().clone());
    chosen_file_editor.wrap_mode(WrapMode::AtBounds, 0);

    let json_buffer = Rc::new(RefCell::new(TextBuffer::default()));
    let mut json_editor = TextEditor::new(0, 0, 0, 0, "");
    json_editor.set_buffer(json_buffer.borrow().clone());
    json_editor.wrap_mode(WrapMode::AtBounds, 0);

    mid_flex.end();
    main_flex.add(&mid_flex);
    main_flex.end();

    win.resizable(&main_flex);
    win.end();
    win.show();

    // 選択されたプロジェクトディレクトリ
    let selected_project_dir: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    // メモリ上で保持する output_path (設定ファイルに書かれたもの)
    let current_output_path: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));

    // --- ボタンのコールバック設定 ---
    {
        let s = s.clone();
        project_btn.set_callback(move |_| {
            s.send(Message::SelectProject);
        });
    }

    {
        let s = s.clone();
        add_file_btn.set_callback(move |_| {
            s.send(Message::AddFile);
        });
    }

    {
        let s = s.clone();
        save_btn.set_callback(move |_| {
            s.send(Message::SaveSettings);
        });
    }

    {
        let s = s.clone();
        load_btn.set_callback(move |_| {
            s.send(Message::StartLoad);
        });
    }

    {
        let s = s.clone();
        copy_btn.set_callback(move |_| {
            s.send(Message::Copy);
        });
    }

    {
        let s = s.clone();
        export_btn.set_callback(move |_| {
            s.send(Message::ExportTxt);
        });
    }

    // 受信ループ
    // 非同期処理の結果やエラーを受け取り、GUIを更新
    while app.wait() {
        if let Some(msg) = r.recv() {
            match msg {
                // ===============================
                // プロジェクトフォルダ選択
                // ===============================
                Message::SelectProject => {
                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                        let folder_path = folder.to_string_lossy().to_string();
                        *selected_project_dir.borrow_mut() = Some(folder_path.clone());
                        
                        let settings_path = Path::new(&folder_path).join("text-read-settings.txt");
                        if !settings_path.exists() {
                            let default_content = r#"# 表示対象ファイルパターンを正規表現で記述してください
# 例：^main\.rs$
# 例：^Cargo\.toml$

# OUTPUT_PATH=ここには自動で保存先フォルダが書き込まれます
"#;
                            if let Ok(mut file) = File::create(&settings_path) {
                                let _ = file.write_all(default_content.as_bytes());
                            }
                        }
                        
                        // 設定ファイルを読み込む
                        let loaded = load_settings(&folder_path);
                        // patterns をGUIに反映
                        pattern_input.borrow_mut().set_value(&loaded.patterns.join("\n"));
                        // output_path をメモリに保持
                        *current_output_path.borrow_mut() = loaded.output_path;
                    }
                }

                // ===============================
                // ファイル追加（パターン生成）
                // ===============================
                Message::AddFile => {
                    if let Some(files) = rfd::FileDialog::new().set_directory(".").pick_files() {
                        let mut input = pattern_input.borrow_mut();
                        let mut current_text = input.value();
                        if !current_text.is_empty() && !current_text.ends_with('\n') {
                            current_text.push('\n');
                        }
        
                        for file in files {
                            if let Some(file_name) = file.file_name() {
                                let fname = file_name.to_string_lossy().to_string();
                                let escaped = regex::escape(&fname);
                                let pattern = format!("^{}$", escaped);
                                current_text.push_str(&pattern);
                                current_text.push('\n');
                            }
                        }
        
                        input.set_value(&current_text);
                    }
                }

                // ===============================
                // 設定保存
                // ===============================
                Message::SaveSettings => {
                    if let Some(dir) = &*selected_project_dir.borrow() {
                        let patterns_text = pattern_input.borrow().value();
                        let patterns: Vec<&str> = patterns_text.lines()
                            .map(|s| s.trim())
                            .filter(|s| !s.is_empty() && !s.starts_with('#'))
                            .collect();

                        // メモリ上にある output_path を取得
                        let output_dir_opt = current_output_path.borrow().clone();

                        if let Err(e) = write_settings(dir, &patterns, &output_dir_opt) {
                            alert_default(&format!("設定保存に失敗しました: {}", e));
                        }
                    }
                }

                // ===============================
                // 読み込み開始（非同期）
                // ===============================
                Message::StartLoad => {
                    let dir_opt = selected_project_dir.borrow().clone();
                    let patterns_text = pattern_input.borrow().value();
                    let sender = s.clone();

                    std::thread::spawn(move || {
                        if let Some(dir) = dir_opt {
                            let patterns: Vec<&str> = patterns_text.lines()
                                .map(|s| s.trim())
                                .filter(|s| !s.is_empty() && !s.starts_with('#'))
                                .collect();

                            let regex_patterns: Vec<Regex> = patterns
                                .iter()
                                .filter_map(|p| Regex::new(p).ok())
                                .collect();

                            match collect_target_files(&dir, &regex_patterns) {
                                Ok(files) => {
                                    let output = ProjectOutput { files };
                                    sender.send(Message::LoadFinished(Ok(output)));
                                }
                                Err(e) => {
                                    sender.send(Message::LoadFinished(Err(e)));
                                }
                            }
                        }
                    });
                }

                // ===============================
                // 読み込み結果
                // ===============================
                Message::LoadFinished(result) => {
                    match result {
                        Ok(output) => {
                            let json_str = match serde_json::to_string_pretty(&output) {
                                Ok(js) => js,
                                Err(e) => {
                                    alert_default(&format!("JSON変換に失敗しました: {}", e));
                                    continue;
                                }
                            };
                            json_buffer.borrow_mut().set_text(&json_str);

                            let mut all_files_text = String::new();
                            for file_info in &output.files {
                                all_files_text.push_str("File: ");
                                all_files_text.push_str(&file_info.file_name);
                                all_files_text.push_str("\n");
                                all_files_text.push_str(&file_info.file_content);
                                all_files_text.push_str("\n--------------------------------\n");
                            }
                            chosen_file_buffer.borrow_mut().set_text(&all_files_text);

                            let size = json_str.len();
                            s.send(Message::UpdateCopySize(size));
                        }
                        Err(e) => {
                            alert_default(&format!("読み込み中にエラーが発生しました: {}", e));
                        }
                    }
                }

                // ===============================
                // コピー
                // ===============================
                Message::Copy => {
                    let val = json_buffer.borrow().text();
                    app::copy(&val);

                    // コピーしたテキスト長を通知
                    s.send(Message::UpdateCopySize(val.len()));
                }

                // ===============================
                // テキスト出力 (ファイル保存ダイアログ)
                // ===============================
                Message::ExportTxt => {
                    let val = json_buffer.borrow().text();
                    if val.is_empty() {
                        alert_default("読み込まれたJSONが空です。先に『読み込み実行』してください。");
                        continue;
                    }

                    // 現在の output_path を取得
                    let current_dir_opt = current_output_path.borrow().clone();
                    // プロジェクトフォルダ
                    let dir_opt = selected_project_dir.borrow().clone();

                    // ダイアログで表示するディレクトリを決める
                    // 1) OUTPUT_PATH があればそれを使う
                    // 2) なければプロジェクトフォルダ
                    // 3) なければカレントディレクトリ
                    let dialog_dir = if let Some(op) = current_dir_opt.clone() {
                        op
                    } else if let Some(proj) = dir_opt.clone() {
                        proj
                    } else {
                        ".".to_string()
                    };

                    // デフォルトファイル名: <プロジェクトフォルダ名>_YYYYMMDD_HHMMSS.txt
                    let now = Local::now();
                    let time_str = now.format("%Y%m%d_%H%M%S").to_string();
                    let default_file_name = if let Some(proj_dir) = &dir_opt {
                        let folder_name = Path::new(proj_dir)
                            .file_name()
                            .unwrap_or_else(|| std::ffi::OsStr::new("NoFolder"))
                            .to_string_lossy()
                            .to_string();
                        format!("{}_{}.txt", folder_name, time_str)
                    } else {
                        format!("output_{}.txt", time_str)
                    };

                    let save_path = rfd::FileDialog::new()
                        .set_directory(&dialog_dir)
                        .set_file_name(&default_file_name)
                        .save_file();

                    if let Some(chosen_path) = save_path {
                        // ファイル書き込み
                        if let Some(parent) = chosen_path.parent() {
                            if let Err(e) = std::fs::create_dir_all(parent) {
                                alert_default(&format!("フォルダ作成失敗: {}", e));
                                continue;
                            }
                        }
                        match File::create(&chosen_path) {
                            Ok(mut f) => {
                                if let Err(e) = f.write_all(val.as_bytes()) {
                                    alert_default(&format!("TXT書き込みに失敗: {}", e));
                                    continue;
                                }
                            }
                            Err(e) => {
                                alert_default(&format!("ファイル作成に失敗: {}", e));
                                continue;
                            }
                        }

                        // ダイアログで選んだフォルダを OUTPUT_PATH として保存
                        // "C:/Users/.../filename.txt" -> "C:/Users/..." を OUTPUT_PATH に
                        if let Some(parent_dir) = chosen_path.parent() {
                            let new_path_str = parent_dir.to_string_lossy().to_string();
                            *current_output_path.borrow_mut() = Some(new_path_str.clone());
                        }

                        // さらに、設定ファイルにも書き戻す
                        // 1) patterns の取得
                        let patterns_text = pattern_input.borrow().value();
                        let patterns: Vec<&str> = patterns_text.lines()
                            .map(|s| s.trim())
                            .filter(|s| !s.is_empty() && !s.starts_with('#'))
                            .collect();

                        // 2) 書き込み
                        if let Some(proj_dir) = &*selected_project_dir.borrow() {
                            if let Err(e) = write_settings(proj_dir, &patterns, &current_output_path.borrow()) {
                                alert_default(&format!("OUTPUT_PATHの設定保存に失敗: {}", e));
                            }
                        }
                    }
                }

                // ===============================
                // コピーサイズの表示更新
                // ===============================
                Message::UpdateCopySize(size) => {
                    copy_size_label.set_label(&format!("Copy Size: {}", size));
                }
            }
        }
    }
}

// --------------------------------------
// 設定ファイルの読み込み
// --------------------------------------
fn load_settings(base_dir: &str) -> LoadedSettings {
    let settings_path = Path::new(base_dir).join("text-read-settings.txt");
    let mut patterns = Vec::new();
    let mut output_path: Option<String> = None;

    if settings_path.exists() {
        if let Ok(file) = File::open(settings_path) {
            for line in BufReader::new(file).lines() {
                if let Ok(pat) = line {
                    let trimmed = pat.trim();
                    if trimmed.is_empty() || trimmed.starts_with('#') {
                        continue;
                    }
                    // OUTPUT_PATH= で始まる行をチェック
                    if let Some(rest) = trimmed.strip_prefix("OUTPUT_PATH=") {
                        let val = rest.trim();
                        if !val.is_empty() {
                            output_path = Some(val.to_string());
                        }
                    } else {
                        patterns.push(trimmed.to_string());
                    }
                }
            }
        }
    }

    LoadedSettings {
        patterns,
        output_path,
    }
}

// --------------------------------------
// 設定ファイルの書き込み
// --------------------------------------
fn write_settings(
    project_dir: &str,
    patterns: &[&str],
    output_path: &Option<String>,
) -> Result<(), String> {
    let settings_path = Path::new(project_dir).join("text-read-settings.txt");

    let mut file = File::create(&settings_path)
        .map_err(|e| format!("設定ファイル作成に失敗: {}", e))?;

    // 1) OUTPUT_PATH
    if let Some(op) = output_path {
        // OUTPUT_PATH=... を書き込み
        let line = format!("OUTPUT_PATH={}\n\n", op);
        file.write_all(line.as_bytes())
            .map_err(|e| format!("OUTPUT_PATH書き込み失敗: {}", e))?;
    }

    // 2) パターンの書き込み
    for pat in patterns {
        if let Err(e) = writeln!(file, "{}", pat) {
            return Err(format!("パターン書き込みに失敗: {}", e));
        }
    }

    Ok(())
}

// --------------------------------------
// ファイルを集める関数（非同期で呼び出される想定）
// --------------------------------------
fn collect_target_files(base_dir: &str, targets: &[Regex]) -> Result<Vec<FileInfo>, String> {
    let mut results = Vec::new();
    let base_path = Path::new(base_dir);

    for entry in WalkDir::new(base_path) {
        let e = entry.map_err(|e| e.to_string())?;
        if e.file_type().is_file() {
            let path = e.path();
            let filename = path.file_name().unwrap().to_string_lossy().to_string();
            if is_in_target_patterns(&filename, targets) {
                let content = fs::read_to_string(path)
                    .map_err(|e| format!("ファイル読み込みに失敗: {} ({})", e, filename))?;
                let file_url = path.to_string_lossy().to_string();
                results.push(FileInfo {
                    file_url,
                    file_name: filename,
                    file_content: content,
                });
            }
        }
    }

    Ok(results)
}

// --------------------------------------
// 正規表現パターンに合致するか確認する関数
// --------------------------------------
fn is_in_target_patterns(filename: &str, patterns: &[Regex]) -> bool {
    patterns.iter().any(|re| re.is_match(filename))
}

// --------------------------------------
// 簡易エラーアラート
// --------------------------------------
fn alert_default(msg: &str) {
    alert(0, 0, msg);
}
