# Specification of hanz

`
–-ignore-ext
`
拡張子を無視してファイル名を比較する。
例: a.txt と a.log を同一として扱う。
`
–-strict-ext
`
拡張子を含めて完全一致で比較する。
例: a.txt と a.log は別物として扱う。

`
-–normalize-copy
`
macOSのコピー時に付与される名前（「のコピー」「のコピー1」など）を正規化して比較する。
例: a.txt と aのコピー1.txt を同一として扱う。

`
–-no-normalize
`
コピー名の正規化を行わない。
例: a.txt と aのコピー1.txt は別物として扱う。

`
–-exclude 
`
指定したパターンに一致するパスを除外する。
例: .git, node_modules
`
--exclude-hidden
`
隠しファイルを除外する．
