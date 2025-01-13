#![windows_subsystem = "windows"]

mod models;
mod settings;
mod fileops;

use fltk::{
    app, prelude::*,
    window::Window,
    button::{Button, CheckButton},
    input::MultilineInput,
    group::Flex,
    text::{TextEditor, TextBuffer, WrapMode},
    frame::Frame,
    dialog::alert,
    enums::{Color, Font},
};
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use chrono::Local;
use std::fs::File;
use std::io::Write;

// 使われている型だけインポート
use crate::models::ProjectOutput;
use crate::settings::{load_settings, write_settings};
use crate::fileops::{collect_target_files, build_tree_view};

// ------------------------------
// GUIメッセージ
// ------------------------------
#[derive(Debug)]
enum Message {
    SelectProject,
    AddFile,
    ExcludeFolder,
    SaveSettings,
    StartLoad,
    LoadFinished(Result<ProjectOutput, String>),
    Copy,
    UpdateCopySize(usize),
    ExportTxt,
}

fn main() {
    let app = app::App::default();
    let mut win = Window::new(100, 100, 1000, 600, "Text-Read with Folder Exclude (Default .git, target)");
    
    // メッセージチャネル (sender, receiver)
    let (s, r) = app::channel::<Message>();

    let mut main_flex = Flex::default().size_of_parent().column();
    main_flex.set_margin(10);

    // -------------------------
    // 上段：パターン入力部（左右2カラム）
    // -------------------------
    let mut pattern_flex = Flex::default().row();
    pattern_flex.set_spacing(10);

    // 左カラム: Include
    let mut left_flex = Flex::default().column();
    left_flex.set_spacing(5);

    // ↓ mut を外しました
    let inc_title = Frame::default().with_label("ファイル追加パターン（Include）");
    left_flex.fixed(&inc_title, 20);

    let include_input = Rc::new(RefCell::new(MultilineInput::new(0, 0, 0, 0, "")));
    include_input.borrow_mut().set_readonly(false);
    left_flex.add(&*include_input.borrow());

    let mut add_file_btn = Button::default().with_label("ファイル追加");
    // ボタンの文字設定
    add_file_btn.set_label_size(14);
    add_file_btn.set_label_color(Color::Black);
    add_file_btn.set_label_font(Font::HelveticaBold);
    left_flex.fixed(&add_file_btn, 30);

    left_flex.end();

    // 右カラム: Exclude
    let mut right_flex = Flex::default().column();
    right_flex.set_spacing(5);

    // ↓ mut を外しました
    let exc_title = Frame::default().with_label("ファイル除外パターン（Exclude）");
    right_flex.fixed(&exc_title, 20);

    let exclude_input = Rc::new(RefCell::new(MultilineInput::new(0, 0, 0, 0, "")));
    exclude_input.borrow_mut().set_readonly(false);
    right_flex.add(&*exclude_input.borrow());

    let mut exclude_folder_btn = Button::default().with_label("フォルダ除外");
    // ボタンの文字設定
    exclude_folder_btn.set_label_size(14);
    exclude_folder_btn.set_label_color(Color::Black);
    exclude_folder_btn.set_label_font(Font::HelveticaBold);
    right_flex.fixed(&exclude_folder_btn, 30);

    right_flex.end();

    pattern_flex.add(&left_flex);
    pattern_flex.add(&right_flex);
    pattern_flex.end();

    main_flex.fixed(&pattern_flex, 180);

    // -------------------------
    // 中段：ボタン行
    // -------------------------
    let mut btn_flex = Flex::default().row();
    btn_flex.set_spacing(10);

    let mut project_btn = Button::default().with_label("プロジェクト選択");
    project_btn.set_label_size(14);
    project_btn.set_label_color(Color::Black);
    project_btn.set_label_font(Font::HelveticaBold);

    let mut save_btn = Button::default().with_label("設定保存");
    save_btn.set_label_size(14);
    save_btn.set_label_color(Color::Black);
    save_btn.set_label_font(Font::HelveticaBold);

    let mut load_btn = Button::default().with_label("読み込み実行");
    load_btn.set_label_size(14);
    load_btn.set_label_color(Color::Black);
    load_btn.set_label_font(Font::HelveticaBold);

    let mut copy_btn = Button::default().with_label("コピー");
    copy_btn.set_label_size(14);
    copy_btn.set_label_color(Color::Black);
    copy_btn.set_label_font(Font::HelveticaBold);

    let mut export_btn = Button::default().with_label("テキスト出力");
    export_btn.set_label_size(14);
    export_btn.set_label_color(Color::Black);
    export_btn.set_label_font(Font::HelveticaBold);

    // チェックボックス：ツリー表示
    let tree_check_state = Rc::new(RefCell::new(false));
    let mut tree_check = CheckButton::default().with_label("ツリー表示");
    // 初期値をオンにする
    tree_check.set_value(true);
    *tree_check_state.borrow_mut() = true;
    {
        let state = tree_check_state.clone();
        tree_check.set_callback(move |cb| {
            *state.borrow_mut() = cb.value();
        });
    }

    let mut copy_size_label = Frame::default().with_label("Copy Size: 0");

    btn_flex.end();
    main_flex.fixed(&btn_flex, 40);

    // -------------------------
    // 下段：テキストエリア（上がファイル内容、下がJSON表示）
    // -------------------------
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

    // --------------------------------
    // 選択されたプロジェクトディレクトリや設定
    // --------------------------------
    let selected_project_dir: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let current_output_path: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));

    // --------------------------------
    // コールバック設定
    // --------------------------------
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
        exclude_folder_btn.set_callback(move |_| {
            s.send(Message::ExcludeFolder);
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

    // --------------------------------
    // イベントループ
    // --------------------------------
    while app.wait() {
        if let Some(msg) = r.recv() {
            match msg {
                // --------------------------
                // プロジェクトフォルダ選択
                // --------------------------
                Message::SelectProject => {
                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                        let folder_path = folder.to_string_lossy().to_string();
                        *selected_project_dir.borrow_mut() = Some(folder_path.clone());

                        let settings_path = Path::new(&folder_path).join("text-read-settings.txt");
                        if !settings_path.exists() {
                            // ここで .git, target を初期除外設定に含める
                            let default_content = r#"# 表示対象ファイルパターン（include）はそのまま記述
# 除外したい場合は行頭に「EXCLUDE:」を付けて記述
# 例：^main\.rs$
# 例：^Cargo\.toml$

# OUTPUT_PATH=ここには自動で保存先フォルダが書き込まれます

# 以下は初期除外パターン
EXCLUDE:^\.git/.*$
EXCLUDE:^target/.*$
"#;
                            if let Ok(mut f) = File::create(&settings_path) {
                                let _ = f.write_all(default_content.as_bytes());
                            }
                        }

                        let loaded = load_settings(&folder_path);
                        // includeパターンをGUIに反映
                        include_input.borrow_mut().set_value(&loaded.patterns_include.join("\n"));
                        // excludeパターンをGUIに反映
                        exclude_input.borrow_mut().set_value(&loaded.patterns_exclude.join("\n"));

                        *current_output_path.borrow_mut() = loaded.output_path;
                    }
                }

                // --------------------------
                // ファイル追加（Include）
                // --------------------------
                Message::AddFile => {
                    if let Some(paths) = rfd::FileDialog::new().set_directory(".").pick_files() {
                        let mut input = include_input.borrow_mut();
                        let mut current_text = input.value();
                        if !current_text.is_empty() && !current_text.ends_with('\n') {
                            current_text.push('\n');
                        }

                        for path in paths {
                            if path.is_file() {
                                if let Some(file_name) = path.file_name() {
                                    let fname = file_name.to_string_lossy().to_string();
                                    let escaped = regex::escape(&fname);
                                    let pattern = format!("^{}$", escaped);
                                    current_text.push_str(&pattern);
                                    current_text.push('\n');
                                }
                            } else if path.is_dir() {
                                if let Some(dir_name) = path.file_name() {
                                    let dname = dir_name.to_string_lossy().to_string();
                                    let escaped = regex::escape(&dname);
                                    let pattern = format!("^{}/.*$", escaped);
                                    current_text.push_str(&pattern);
                                    current_text.push('\n');
                                }
                            }
                        }
                        input.set_value(&current_text);
                    }
                }

                // --------------------------
                // フォルダ除外
                // --------------------------
                Message::ExcludeFolder => {
                    if let Some(folder_path) = rfd::FileDialog::new().set_directory(".").pick_folder() {
                        let mut input = exclude_input.borrow_mut();
                        let mut current_text = input.value();
                        if !current_text.is_empty() && !current_text.ends_with('\n') {
                            current_text.push('\n');
                        }
                        // 例：^MyFolder/.*$
                        if let Some(dir_name) = folder_path.file_name() {
                            let dname = dir_name.to_string_lossy().to_string();
                            let escaped = regex::escape(&dname);
                            let pattern = format!("^{}/.*$", escaped);
                            current_text.push_str(&pattern);
                            current_text.push('\n');
                        }
                        input.set_value(&current_text);
                    }
                }

                // --------------------------
                // 設定保存
                // --------------------------
                Message::SaveSettings => {
                    if let Some(dir) = &*selected_project_dir.borrow() {
                        let inc_text = include_input.borrow().value();
                        let exc_text = exclude_input.borrow().value();

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

                        let output_dir_opt = current_output_path.borrow().clone();
                        if let Err(e) = write_settings(dir, &inc_patterns, &exc_patterns, &output_dir_opt) {
                            alert_default(&format!("設定保存に失敗しました: {}", e));
                        }
                    }
                }

                // --------------------------
                // 読み込み実行
                // --------------------------
                Message::StartLoad => {
                    let dir_opt = selected_project_dir.borrow().clone();
                    let inc_text = include_input.borrow().value();
                    let exc_text = exclude_input.borrow().value();
                    let sender = s.clone();
                    let include_tree = *tree_check_state.borrow();

                    std::thread::spawn(move || {
                        if let Some(dir) = dir_opt {
                            let inc_patterns: Vec<regex::Regex> = inc_text
                                .lines()
                                .map(|s| s.trim())
                                .filter(|s| !s.is_empty())
                                .filter_map(|p| regex::Regex::new(p).ok())
                                .collect();

                            let exc_patterns: Vec<regex::Regex> = exc_text
                                .lines()
                                .map(|s| s.trim())
                                .filter(|s| !s.is_empty())
                                .filter_map(|p| regex::Regex::new(p).ok())
                                .collect();

                            match collect_target_files(&dir, &inc_patterns, &exc_patterns) {
                                Ok(files) => {
                                    // ツリー表示
                                    let tree_view = if include_tree {
                                        Some(build_tree_view(&dir))
                                    } else {
                                        None
                                    };
                                    let output = ProjectOutput { files, tree_view };
                                    sender.send(Message::LoadFinished(Ok(output)));
                                }
                                Err(e) => {
                                    sender.send(Message::LoadFinished(Err(e)));
                                }
                            }
                        }
                    });
                }

                // --------------------------
                // 読み込み結果
                // --------------------------
                Message::LoadFinished(result) => {
                    match result {
                        Ok(output) => {
                            // JSON表示
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
                            if let Some(tree) = &output.tree_view {
                                all_files_text.push_str("\n[Directory Tree]\n");
                                all_files_text.push_str(tree);
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

                // --------------------------
                // コピー
                // --------------------------
                Message::Copy => {
                    let val = json_buffer.borrow().text();
                    app::copy(&val);
                    s.send(Message::UpdateCopySize(val.len()));
                }

                // --------------------------
                // テキスト出力
                // --------------------------
                Message::ExportTxt => {
                    let val = json_buffer.borrow().text();
                    if val.is_empty() {
                        alert_default("読み込まれたJSONが空です。先に『読み込み実行』してください。");
                        continue;
                    }

                    let current_dir_opt = current_output_path.borrow().clone();
                    let dir_opt = selected_project_dir.borrow().clone();

                    let dialog_dir = if let Some(op) = current_dir_opt.clone() {
                        op
                    } else if let Some(proj) = dir_opt.clone() {
                        proj
                    } else {
                        ".".to_string()
                    };

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

                        // 保存先を output_path に反映
                        if let Some(parent_dir) = chosen_path.parent() {
                            let new_path_str = parent_dir.to_string_lossy().to_string();
                            *current_output_path.borrow_mut() = Some(new_path_str.clone());
                        }

                        // 設定ファイルにも書き戻し
                        let inc_text = include_input.borrow().value();
                        let exc_text = exclude_input.borrow().value();

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

                        if let Some(proj_dir) = &*selected_project_dir.borrow() {
                            if let Err(e) = write_settings(proj_dir, &inc_patterns, &exc_patterns, &current_output_path.borrow()) {
                                alert_default(&format!("OUTPUT_PATHの設定保存に失敗: {}", e));
                            }
                        }
                    }
                }

                // --------------------------
                // コピーサイズ更新
                // --------------------------
                Message::UpdateCopySize(size) => {
                    copy_size_label.set_label(&format!("Copy Size: {}", size));
                }
            }
        }
    }
}

// --------------------------------------------------
// アラート
// --------------------------------------------------
fn alert_default(msg: &str) {
    alert(0, 0, msg);
}
