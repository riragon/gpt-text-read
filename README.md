<!DOCTYPE html>
<html lang="ja">
<head>
<meta charset="UTF-8">

</head>
<body>

<h2>プロジェクト概要</h2>
<p>このプロジェクト「gpt-text-read」は、フォルダ内のテキストファイルを収集し、  
その内容をJSON形式で表示するGUIツールです。  
ユーザーはパターンベース（正規表現）で対象ファイルを指定したり、  
GUI上でファイルを追加してパターンを生成したりすることができます。</p>

<h2>主な機能</h2>
<p>
- プロジェクトディレクトリの選択：  
  「プロジェクト選択」ボタンを使ってフォルダを選ぶと、  
  指定フォルダ内に`text-read-settings.txt`がない場合は初期設定ファイルを自動生成します。  
  存在する場合は、その内容（ファイルマッチパターン）を読み込みます。  
</p>
<p>
- ファイル追加ボタン：  
  「ファイル追加」ボタンで実際のファイルを選択すると、そのファイル名にマッチする  
  正規表現パターン（`^ファイル名$`）が自動的に設定ファイルへ追加されます。  
</p>
<p>
- 設定保存ボタン：  
  「設定保存」ボタンにより、現在GUIで編集したパターン内容を`text-read-settings.txt`へ書き戻します。  
</p>
<p>
- 読み込み実行ボタン：  
  「読み込み実行」ボタンで、現在のパターンにマッチするファイルを再帰的に探索し、  
  対象ファイルの内容をJSON形式でウィンドウ上に表示します。  
</p>
<p>
- コピー機能：  
  「コピー」ボタンで、表示されたJSONテキストをクリップボードへコピーできます。  
</p>

<h2>利用手順</h2>
<p>
1. プロジェクトフォルダを選択します。「プロジェクト選択」ボタンを押し、  
   対象とするフォルダをダイアログから選びます。  
   初回は`text-read-settings.txt`が自動作成され、基本的なコメントや例が含まれます。  
</p>
<p>
2. ファイル追加またはパターン編集を行います。  
   「ファイル追加」ボタンで実際のファイルを追加すると、そのファイル名が  
   パターンとして`pattern_input`に反映されます。  
   また、`pattern_input`は手動で編集可能です。  
</p>
<p>
3. 「設定保存」ボタンで現在のパターンを`text-read-settings.txt`へ保存します。  
</p>
<p>
4. 「読み込み実行」ボタンでパターンにマッチしたファイルを読み込み、  
   JSON形式で結果を表示します。  
</p>
<p>
5. 必要に応じて「コピー」ボタンでJSONをクリップボードへコピーします。  
</p>

<h2>依存関係・ビルド</h2>
<p>
Cargo.toml例：  
<pre>
[package]
name = "gpt-text-read"
version = "0.1.0"
edition = "2021"

[dependencies]
walkdir = "2"
regex = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rfd = "0.8"
fltk = { version = "1", features = ["fltk-bundled"] }
</pre>
`cargo build`や`cargo run`でプロジェクトをビルド・実行可能です。  
</p>

<h2>コード構成</h2>
<p>
main.rsにはGUIの初期化、ボタンコールバック、ファイルパターンの読み込み・保存、  
およびファイル収集処理が記述されています。  
`text-read-settings.txt`を介して対象ファイルパターンを柔軟にカスタマイズできます。  
</p>

<h2>ライセンス</h2>
<p>本プロジェクトは、特に明記がない場合、MITまたはApache-2.0ライセンスで配布可能です。  
詳細はライセンスファイルを参照してください。</p>

</body>
</html>
