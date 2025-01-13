<!DOCTYPE html>
<html lang="ja">
<head>
<meta charset="UTF-8">
<title>gpt-text-read README</title>
</head>
<body>

<h2>プロジェクト概要</h2>
<p>このプロジェクト「gpt-text-read」は、フォルダ内のテキストファイルを収集し、  
その内容をJSON形式で表示・コピー・テキスト出力できるGUIツールです。  
ユーザーは正規表現パターンを使って対象ファイルや除外ファイルを柔軟に指定できます。  
また、GUI上でフォルダを選択・ファイルを追加・フォルダ除外などを行うことで、  
簡単にパターンの編集を行うことができます。</p>

<h2>主な機能</h2>
<ul>
  <li>
    <strong>プロジェクトディレクトリの選択：</strong><br>
    「<em>プロジェクト選択</em>」ボタンを使ってフォルダを選ぶと、
    指定フォルダ内に<code>text-read-settings.txt</code>がない場合は初期設定ファイルを自動生成します。
    存在する場合は、その内容（ファイルマッチパターンや出力先パスなど）を読み込みます。
  </li>
  <li>
    <strong>ファイル追加ボタン：</strong><br>
    「<em>ファイル追加</em>」ボタンで実際のファイルを選択すると、
    そのファイル名にマッチする正規表現パターン（<code>^ファイル名$</code>）が
    <code>text-read-settings.txt</code>のIncludeパターンへ追記されます。
  </li>
  <li>
    <strong>フォルダ除外ボタン：</strong><br>
    「<em>フォルダ除外</em>」ボタンを押すと、フォルダ名に対する除外パターン（<code>^FolderName/.*$</code>）を
    <code>text-read-settings.txt</code>のExcludeパターンへ追記できます。
    デフォルトでは<code>.git</code>や<code>target</code>が自動的に除外パターンに含まれます。
  </li>
  <li>
    <strong>ツリー表示チェック：</strong><br>
    「<em>ツリー表示</em>」チェックボックスを有効にすると、
    フォルダの階層構造をテキストツリーとして取得できます。
  </li>
  <li>
    <strong>設定保存ボタン：</strong><br>
    「<em>設定保存</em>」ボタンにより、現在GUIで編集したパターン内容を<code>text-read-settings.txt</code>へ書き戻します。
    また、テキスト出力した際も、出力先フォルダが<code>OUTPUT_PATH</code>として保存されます。
  </li>
  <li>
    <strong>読み込み実行ボタン：</strong><br>
    「<em>読み込み実行</em>」ボタンで、現在のパターンにマッチするファイルを再帰的に探索し、
    対象ファイルの内容をJSON形式でウィンドウ上に表示します。
    （ツリー表示がONならディレクトリ構造も表示します）
  </li>
  <li>
    <strong>コピー機能：</strong><br>
    「<em>コピー</em>」ボタンで、表示されたJSONテキストをクリップボードへコピーできます。
  </li>
  <li>
    <strong>テキスト出力：</strong><br>
    「<em>テキスト出力</em>」ボタンで、表示中のJSONテキストを任意のパスに出力できます。
    出力先フォルダが<code>text-read-settings.txt</code>の<code>OUTPUT_PATH</code>に自動保存され、
    次回以降の保存先の初期値として利用されます。
  </li>
</ul>

<h2>利用手順</h2>
<ol>
  <li>
    <strong>プロジェクトフォルダを選択する：</strong><br>
    「<em>プロジェクト選択</em>」ボタンを押し、対象とするフォルダをダイアログから選びます。
    初回は<code>text-read-settings.txt</code>が自動作成され、初期除外パターンやコメントが含まれます。
  </li>
  <li>
    <strong>ファイル追加・フォルダ除外・パターン編集を行う：</strong><br>
    「<em>ファイル追加</em>」や「<em>フォルダ除外</em>」ボタンで実際のファイル・フォルダを追加/除外すると、
    そのパターンが<code>pattern_input</code>（GUIエリア）に自動的に追加されます。
    また、GUI上でパターンを手動編集することもできます。
  </li>
  <li>
    <strong>設定を保存する：</strong><br>
    「<em>設定保存</em>」ボタンで現在のパターンを<code>text-read-settings.txt</code>へ保存します。
  </li>
  <li>
    <strong>読み込みを実行する：</strong><br>
    「<em>読み込み実行</em>」ボタンでパターンにマッチしたファイルを読み込み、
    JSON形式で結果を表示します。<br>
    また、<em>ツリー表示</em>チェックがONの場合はディレクトリ構造が合わせて表示されます。
  </li>
  <li>
    <strong>コピーまたはテキスト出力：</strong><br>
    JSONをコピーしたい場合は「<em>コピー</em>」ボタンを押します。  
    テキストファイルとして出力したい場合は「<em>テキスト出力</em>」ボタンを押し、
    出力先フォルダを指定します。その後の出力先は設定ファイル<code>OUTPUT_PATH</code>へ保存されます。
  </li>
</ol>

<h2>依存関係・ビルド</h2>
<p>
このプロジェクトはRustで開発されており、<code>Cargo.toml</code>の例は以下の通りです：
</p>
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
chrono = "0.4"
</pre>
<p>
<code>cargo build</code>や<code>cargo run</code>でビルド・実行できます。
</p>

<h2>コード構成</h2>
<p>
<code>main.rs</code>にGUIの初期化、ボタンコールバック、ファイルパターンの読み込み・保存、  
およびファイル収集処理が記述されています。  
<code>text-read-settings.txt</code>にはIncludeパターンとExcludeパターン、そして<code>OUTPUT_PATH</code>が含まれ、  
GUIからの編集によって内容が更新されます。  
</p>
<p>
<code>fileops.rs</code>には、指定フォルダを再帰的に探索してマッチするファイルを集める機能や、  
ディレクトリツリーを構築する機能が定義されています。  
</p>
<p>
<code>settings.rs</code>には、<code>text-read-settings.txt</code>の読み書きロジックがあり、  
Includeパターン・Excludeパターン・出力先パス(<code>OUTPUT_PATH</code>)を管理します。  
</p>

<h2>ライセンス</h2>
<p>
本プロジェクトは、特に明記がない場合、MITまたはApache-2.0ライセンスで配布可能です。  
詳細はライセンスファイルを参照してください。
</p>

</body>
</html>
