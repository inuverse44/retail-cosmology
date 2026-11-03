//! ResourceAccess：結果のファイル（CSV、テキスト、TeX の表）を書く。

use std::path::Path;

/// 文字列をファイルに書く。親のフォルダがなければ作る。
pub fn write_text(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, text)
}
