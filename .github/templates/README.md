# hanz

![Static Badge](https://img.shields.io/badge/License-GPL-blue)
![example workflow](https://github.com/kana112/hanz/actions/workflows/build.yml/badge.svg)
[![DOI](https://zenodo.org/badge/1206547447.svg)](https://doi.org/10.5281/zenodo.21378519)
[![Coverage Status](https://coveralls.io/repos/github/kana112/hanz/badge.svg?branch=main)](https://coveralls.io/github/kana112/hanz?branch=main)

Version: ${VERSION}

`hanz` は、指定したディレクトリ内から「不要かもしれないファイル」を検出して表示する CLI ツールです。
ファイルの削除、移動、コピーは行いません。

## 機能

- 指定したディレクトリ配下だけを再帰的に探索
- 重複らしいファイル名を `--name` で検出
- SHA-256 が一致する完全重複ファイルや、内容・構成が同じディレクトリを `--hash` で検出
- `--collect` 指定時だけ、候補へのシンボリックリンクを作成
- `.git`、`target`、`node_modules`、`.junk-links` を探索対象から除外
- シンボリックリンクや特殊ファイルをスキップ

## ビルド

```bash
cargo build --release
```

実行ファイルは `target/release/hanz` に作成されます。

## 使い方

名前から候補を検出します。

```bash
cargo run -- ./Downloads --name
```

完全重複ファイルを検出します。

```bash
cargo run -- ./Downloads --hash
```

両方の方法で検出します。

```bash
cargo run -- ./Downloads --name --hash
```

候補へのシンボリックリンクを `.junk-links` にまとめます。

```bash
cargo run -- ./Downloads --name --collect .junk-links
cargo run -- ./Downloads --hash --collect .junk-links
```

## 出力例

```text
NAME  ./Downloads/report (1).pdf
      reason: duplicate-like filename

HASH  ./Downloads/a.pdf
      duplicate of: ./Downloads/b.pdf
      sha256: xxxxx

DIR_HASH  ./Downloads/backup-a
      duplicate of: ./Downloads/backup-b
      sha256: xxxxx
```

候補がない場合は `No candidates found.` と表示します。

## 判定方法

`--name` は、ファイル名に `コピー`、` copy`、` Copy`、` (数字)` が含まれる場合や、拡張子直前が ` 2` 以上の数字で終わる場合に候補とします。

`--hash` は、まずファイルサイズなどで候補を絞り込みます。同じサイズのファイルが2つ以上あるグループだけをバッファ読み込みで SHA-256 計算し、ハッシュが一致したファイルを候補とします。

ディレクトリは、配下の相対パス・ファイルサイズ・各ファイルの SHA-256 から内容と構成を比較します。同一ディレクトリが見つかった場合は `DIR_HASH` としてディレクトリを表示し、その配下のファイル候補は重複して表示しません。

## リンク収集

`--collect <DIR>` を指定すると、指定した収集先ディレクトリを作成し、候補へのシンボリックリンクをそこにまとめます。リンク先には候補ファイルの絶対パスを使用します。

既存の収集先は、中身を消去してから作り直します。収集先が通常ディレクトリでない場合や、探索対象またはその親ディレクトリを収集先に指定した場合はエラーになります。

## テスト

```bash
cargo test --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
```
