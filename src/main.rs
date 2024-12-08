use serde::Serialize;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use fltk::{app, prelude::*, window::Window, button::Button, input::MultilineInput, group::Flex};
use std::rc::Rc;
use std::cell::RefCell;
use walkdir::WalkDir;
use regex::Regex;

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

fn main() {
    let app = app::App::default();
    let mut win = Window::new(100, 100, 800, 600, "Text-Read with Settings");

    let mut flex = Flex::default().size_of_parent().column();
    flex.set_margin(10);

    // ファイルパターン入力欄
    let pattern_input = MultilineInput::new(0,0,0,0,"");
    let pattern_input = Rc::new(RefCell::new(pattern_input));
    pattern_input.borrow_mut().set_readonly(false); 

    // ボタンエリア
    let mut btn_flex = Flex::default().row();
    btn_flex.set_spacing(10);

    let mut project_btn = Button::default().with_label("プロジェクト選択");
    let mut add_file_btn = Button::default().with_label("ファイル追加"); // 復活したファイル追加ボタン
    let mut save_btn = Button::default().with_label("設定保存");
    let mut load_btn = Button::default().with_label("読み込み実行");
    let mut copy_btn = Button::default().with_label("コピー");

    btn_flex.end();

    let json_input_widget = MultilineInput::new(0,0,0,0,"");
    let json_input = Rc::new(RefCell::new(json_input_widget));
    json_input.borrow_mut().set_readonly(true);

    flex.end();
    win.end();
    win.show();

    // 選択されたプロジェクトディレクトリパス
    let selected_project_dir: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));

    {
        let selected_project_dir = Rc::clone(&selected_project_dir);
        let pattern_input = Rc::clone(&pattern_input);
        project_btn.set_callback(move |_| {
            if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                let folder_path = folder.to_string_lossy().to_string();
                *selected_project_dir.borrow_mut() = Some(folder_path.clone());

                // settingsファイルチェック
                let settings_path = Path::new(&folder_path).join("text-read-settings.txt");
                if !settings_path.exists() {
                    // 存在しないので初期作成
                    let default_content = r#"# 表示対象ファイルパターンを正規表現で記述してください
# 例：^main\.rs$
# 例：^Cargo\.toml$
"#;
                    if let Ok(mut file) = File::create(&settings_path) {
                        let _ = file.write_all(default_content.as_bytes());
                    }
                }

                // 読み込み
                let patterns = load_target_patterns(&folder_path);
                pattern_input.borrow_mut().set_value(&patterns.join("\n"));
            }
        });
    }

    {
        let pattern_input = Rc::clone(&pattern_input);
        add_file_btn.set_callback(move |_| {
            // ファイル選択ダイアログ
            if let Some(files) = rfd::FileDialog::new().set_directory(".").pick_files() {
                let mut input = pattern_input.borrow_mut();
                let mut current_text = input.value();
                if !current_text.is_empty() && !current_text.ends_with('\n') {
                    current_text.push('\n');
                }

                // 選択したファイルをパターン化
                for file in files {
                    if let Some(file_name) = file.file_name() {
                        let fname = file_name.to_string_lossy().to_string();
                        // 正確に一致するパターンを追加
                        let escaped = regex::escape(&fname);
                        let pattern = format!("^{}$", escaped);
                        current_text.push_str(&pattern);
                        current_text.push('\n');
                    }
                }

                input.set_value(&current_text);
            }
        });
    }

    {
        let selected_project_dir = Rc::clone(&selected_project_dir);
        let pattern_input = Rc::clone(&pattern_input);
        save_btn.set_callback(move |_| {
            if let Some(dir) = &*selected_project_dir.borrow() {
                let settings_path = Path::new(dir).join("text-read-settings.txt");
                let text = pattern_input.borrow().value();
                // 保存
                if let Ok(mut file) = File::create(&settings_path) {
                    let _ = file.write_all(text.as_bytes());
                }
            }
        });
    }

    {
        let selected_project_dir = Rc::clone(&selected_project_dir);
        let pattern_input = Rc::clone(&pattern_input);
        let json_input = Rc::clone(&json_input);
        load_btn.set_callback(move |_| {
            if let Some(dir) = &*selected_project_dir.borrow() {
                let base_dir = dir.as_str();
                // pattern_inputからパターンを取得
                let patterns_text = pattern_input.borrow().value();
                let patterns: Vec<&str> = patterns_text.lines()
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty() && !s.starts_with('#'))
                    .collect();

                let regex_patterns: Vec<Regex> = patterns.iter()
                    .filter_map(|p| Regex::new(p).ok())
                    .collect();

                let files = collect_target_files(base_dir, &regex_patterns);

                let output = ProjectOutput { files };
                let json_str = serde_json::to_string_pretty(&output).unwrap();
                json_input.borrow_mut().set_value(&json_str);
            }
        });
    }

    {
        let json_input = Rc::clone(&json_input);
        copy_btn.set_callback(move |_| {
            let val = json_input.borrow().value();
            app::copy(val.as_str());
        });
    }

    app.run().unwrap();
}

/// text-read-settings.txtから表示対象パターンを取得
fn load_target_patterns(base_dir: &str) -> Vec<String> {
    let settings_path = Path::new(base_dir).join("text-read-settings.txt");
    let mut patterns = Vec::new();
    if settings_path.exists() {
        if let Ok(file) = File::open(settings_path) {
            for line in BufReader::new(file).lines() {
                if let Ok(pat) = line {
                    let trimmed = pat.trim();
                    if trimmed.is_empty() || trimmed.starts_with('#') {
                        continue;
                    }
                    patterns.push(trimmed.to_string());
                }
            }
        }
    }
    patterns
}

/// パターンに合致するファイルを収集
fn collect_target_files(base_dir: &str, targets: &[Regex]) -> Vec<FileInfo> {
    let mut results = Vec::new();
    let base_path = Path::new(base_dir);

    for entry in WalkDir::new(base_path) {
        if let Ok(e) = entry {
            if e.file_type().is_file() {
                let path = e.path();
                let filename = path.file_name().unwrap().to_string_lossy().to_string();
                if is_in_target_patterns(&filename, targets) {
                    if let Ok(content) = fs::read_to_string(path) {
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
    }

    results
}

/// ターゲットパターンにマッチするか
fn is_in_target_patterns(filename: &str, patterns: &[Regex]) -> bool {
    patterns.iter().any(|re| re.is_match(filename))
}