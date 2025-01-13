use serde::Serialize;

/// ファイル情報
#[derive(Serialize, Debug)]
pub struct FileInfo {
    pub file_url: String,
    pub file_name: String,
    pub file_content: String,
}

/// 読み込み結果
#[derive(Serialize, Debug)]
pub struct ProjectOutput {
    pub files: Vec<FileInfo>,
    pub tree_view: Option<String>,
}

/// 設定ファイルから読み込む内容
#[derive(Debug)]
pub struct LoadedSettings {
    pub patterns_include: Vec<String>,
    pub patterns_exclude: Vec<String>,
    pub output_path: Option<String>,
}
