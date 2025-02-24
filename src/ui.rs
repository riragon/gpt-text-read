use fltk::prelude::*;
use fltk::{
    button::{Button, CheckButton},
    dialog::alert,
    enums::{Color, Font},
    frame::Frame,
    group::{Flex, Tabs, Group},
    input::MultilineInput,
    text::{TextBuffer, TextEditor, WrapMode},
    window::Window,
    app::{Sender, Receiver},
};
use std::{cell::RefCell, rc::Rc};

use crate::app::AppData;

/// メッセージ（イベント）
#[derive(Clone, Debug)]
pub enum UiMessage {
    SelectProject,
    AddFile,
    ExcludeFolder,
    SaveSettings,
    StartLoad,
    LoadFinished(Result<crate::models::ProjectOutput, String>),
    Copy,
    UpdateCopySize(usize),
    ExportTxt,
    Backup,
}

/// GUI部品をまとめた構造体
pub struct GuiComponents {
    pub win: Window,
    pub receiver: Receiver<UiMessage>,
    pub sender: Sender<UiMessage>,

    pub include_input: Rc<RefCell<MultilineInput>>,
    pub exclude_input: Rc<RefCell<MultilineInput>>,

    pub chosen_file_buffer: Rc<RefCell<TextBuffer>>,
    pub json_buffer: Rc<RefCell<TextBuffer>>,
    pub tree_buffer: Rc<RefCell<TextBuffer>>,
    pub dev_memo_buffer: Rc<RefCell<TextBuffer>>,

    // LLM補足用のバッファ
    pub llm_buffer: Rc<RefCell<TextBuffer>>,

    // 追加: ログ表示用バッファ
    pub log_buffer: Rc<RefCell<TextBuffer>>,

    pub tree_check_state: Rc<RefCell<bool>>,
    pub copy_size_label: Frame,
}

/// GUI を生成して GuiComponents を返す
pub fn build_ui(_app_data: Rc<AppData>) -> GuiComponents {
    let win = Window::new(100, 100, 1000, 600, "Text-Read (Refactored)");

    // チャネル
    let (s, r) = fltk::app::channel::<UiMessage>();

    // レイアウト用 Flex
    let mut main_flex = Flex::default().size_of_parent().column();
    main_flex.set_margin(10);

    // -------------------------------
    // パターン入力欄 (上段)
    // -------------------------------
    let mut pattern_flex = Flex::default().row();
    pattern_flex.set_spacing(10);

    let mut left_flex = Flex::default().column();
    left_flex.set_spacing(5);

    let mut add_file_btn = Button::default().with_label("ファイル追加");
    add_file_btn.set_label_size(14);
    add_file_btn.set_label_color(Color::Black);
    add_file_btn.set_label_font(Font::HelveticaBold);
    left_flex.fixed(&add_file_btn, 30);

    let include_input = Rc::new(RefCell::new(MultilineInput::new(0, 0, 0, 0, "")));
    include_input.borrow_mut().set_readonly(false);
    left_flex.add(&*include_input.borrow());
    left_flex.end();

    let mut right_flex = Flex::default().column();
    right_flex.set_spacing(5);

    let mut exclude_folder_btn = Button::default().with_label("ツリーフォルダ除外");
    exclude_folder_btn.set_label_size(14);
    exclude_folder_btn.set_label_color(Color::Black);
    exclude_folder_btn.set_label_font(Font::HelveticaBold);
    right_flex.fixed(&exclude_folder_btn, 30);

    let exclude_input = Rc::new(RefCell::new(MultilineInput::new(0, 0, 0, 0, "")));
    exclude_input.borrow_mut().set_readonly(false);
    right_flex.add(&*exclude_input.borrow());
    right_flex.end();

    pattern_flex.add(&left_flex);
    pattern_flex.add(&right_flex);
    pattern_flex.end();

    main_flex.fixed(&pattern_flex, 180);

    // -------------------------------
    // ボタン類 (中段)
    // -------------------------------
    let mut btn_flex = Flex::default().row();
    btn_flex.set_spacing(10);

    let mut project_btn = Button::default().with_label("プロジェクト選択");
    project_btn.set_label_size(14);
    project_btn.set_label_color(Color::Black);
    project_btn.set_label_font(Font::HelveticaBold);

    let mut copy_btn = Button::default().with_label("コピー");
    copy_btn.set_label_size(14);
    copy_btn.set_label_color(Color::Black);
    copy_btn.set_label_font(Font::HelveticaBold);

    let mut backup_btn = Button::default().with_label("スナップショット作成");
    backup_btn.set_label_size(14);
    backup_btn.set_label_color(Color::Black);
    backup_btn.set_label_font(Font::HelveticaBold);

    let mut export_btn = Button::default().with_label("テキスト出力");
    export_btn.set_label_size(14);
    export_btn.set_label_color(Color::Black);
    export_btn.set_label_font(Font::HelveticaBold);

    let tree_check_state = Rc::new(RefCell::new(true));
    let mut tree_check = CheckButton::default().with_label("ツリー表示");
    tree_check.set_value(true);
    {
        let state = tree_check_state.clone();
        tree_check.set_callback(move |cb| {
            *state.borrow_mut() = cb.value();
        });
    }

    let mut update_btn = Button::default().with_label("保存更新");
    update_btn.set_label_size(14);
    update_btn.set_label_color(Color::Black);
    update_btn.set_label_font(Font::HelveticaBold);

    let copy_size_label = Frame::default().with_label("Copy Size: 0");

    btn_flex.add(&project_btn);
    btn_flex.add(&copy_btn);
    btn_flex.add(&backup_btn);
    btn_flex.add(&export_btn);
    btn_flex.add(&tree_check);
    btn_flex.add(&update_btn);
    btn_flex.add(&copy_size_label);
    btn_flex.end();

    main_flex.fixed(&btn_flex, 40);

    // -------------------------------
    // 下段タブ
    // -------------------------------
    let tabs_flex = Flex::default().column();

    let chosen_file_buffer = Rc::new(RefCell::new(TextBuffer::default()));
    let json_buffer = Rc::new(RefCell::new(TextBuffer::default()));
    let tree_buffer = Rc::new(RefCell::new(TextBuffer::default()));
    let dev_memo_buffer = Rc::new(RefCell::new(TextBuffer::default()));
    let llm_buffer = Rc::new(RefCell::new(TextBuffer::default()));
    let log_buffer = Rc::new(RefCell::new(TextBuffer::default())); // ログ用

    let tabs = Tabs::new(0, 0, 1000, 300, "");

    // --- ファイル内容タブ
    let grp_text = Group::new(0, 25, 1000, 275, "ファイル内容");
    {
        let mut chosen_file_editor = TextEditor::new(5, 30, 990, 260, "");
        chosen_file_editor.set_buffer(chosen_file_buffer.borrow().clone());
        chosen_file_editor.wrap_mode(WrapMode::AtBounds, 0);
    }
    grp_text.end();

    // --- ツリー内容タブ
    let grp_tree = Group::new(0, 25, 1000, 275, "ツリー内容");
    {
        let mut tree_editor = TextEditor::new(5, 30, 990, 260, "");
        tree_editor.set_buffer(tree_buffer.borrow().clone());
        tree_editor.wrap_mode(WrapMode::AtBounds, 0);
    }
    grp_tree.end();

    // --- JSONデータタブ
    let grp_json = Group::new(0, 25, 1000, 275, "JSONデータ");
    {
        let mut json_editor = TextEditor::new(5, 30, 990, 260, "");
        json_editor.set_buffer(json_buffer.borrow().clone());
        json_editor.wrap_mode(WrapMode::AtBounds, 0);
    }
    grp_json.end();

    // --- 開発メモタブ
    let grp_dev_memo = Group::new(0, 25, 1000, 275, "開発メモ");
    {
        let mut dev_memo_editor = TextEditor::new(5, 30, 990, 260, "");
        dev_memo_editor.set_buffer(dev_memo_buffer.borrow().clone());
        dev_memo_editor.wrap_mode(WrapMode::AtBounds, 0);
    }
    grp_dev_memo.end();

    // --- LLM補足タブ
    let grp_llm = Group::new(0, 25, 1000, 275, "LLM補足");
    {
        let mut llm_editor = TextEditor::new(5, 30, 990, 260, "");
        llm_editor.set_buffer(llm_buffer.borrow().clone());
        llm_editor.wrap_mode(WrapMode::AtBounds, 0);
    }
    grp_llm.end();

    // --- ログタブ (追加)
    let grp_log = Group::new(0, 25, 1000, 275, "ログ");
    {
        let mut log_editor = TextEditor::new(5, 30, 990, 260, "");
        log_editor.set_buffer(log_buffer.borrow().clone());
        log_editor.wrap_mode(WrapMode::AtBounds, 0);
        // 読み取り専用にはしないが、ユーザーが編集してもログに影響はありません
    }
    grp_log.end();

    tabs.end();
    tabs_flex.end();

    main_flex.add(&tabs_flex);
    main_flex.end();

    win.end();

    // -----------------------------
    // ボタンのコールバック
    // -----------------------------
    {
        let sender = s.clone();
        project_btn.set_callback(move |_| {
            sender.send(UiMessage::SelectProject);
        });
    }
    {
        let sender = s.clone();
        add_file_btn.set_callback(move |_| {
            sender.send(UiMessage::AddFile);
        });
    }
    {
        let sender = s.clone();
        exclude_folder_btn.set_callback(move |_| {
            sender.send(UiMessage::ExcludeFolder);
        });
    }
    {
        let sender = s.clone();
        copy_btn.set_callback(move |_| {
            sender.send(UiMessage::Copy);
        });
    }
    {
        let sender = s.clone();
        export_btn.set_callback(move |_| {
            sender.send(UiMessage::ExportTxt);
        });
    }
    {
        let sender = s.clone();
        update_btn.set_callback(move |_| {
            sender.send(UiMessage::SaveSettings);
            sender.send(UiMessage::StartLoad);
        });
    }
    {
        let sender = s.clone();
        backup_btn.set_callback(move |_| {
            sender.send(UiMessage::Backup);
        });
    }

    GuiComponents {
        win,
        receiver: r,
        sender: s,

        include_input,
        exclude_input,

        chosen_file_buffer,
        json_buffer,
        tree_buffer,
        dev_memo_buffer,

        llm_buffer,

        log_buffer,

        tree_check_state,
        copy_size_label,
    }
}

fn _alert_default(msg: &str) {
    alert(0, 0, msg);
}
