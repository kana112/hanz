---
title: hanz
description: 不要かもしれないファイルを削除せずに見つける CLI ツール
---

# hanz

![Static Badge](https://img.shields.io/badge/License-GPL-blue)
![example workflow](https://github.com/kana112/hanz/actions/workflows/build.yml/badge.svg)
[![Coverage Status](https://coveralls.io/repos/github/kana112/hanz/badge.svg?branch=main)](https://coveralls.io/github/kana112/hanz?branch=main)

`hanz` は、ディレクトリの中から「不要かもしれないファイル」を見つけるための小さな CLI ツールです。
重複っぽいファイル名や、内容が完全に同じファイルを検出します。

> [!NOTE]
> `hanz` は検出結果を表示するだけです。ファイルの削除、移動、コピーは行いません。

## できること

| モード | 概要 |
| --- | --- |
| `--name` | `copy`、`コピー`、` (1)` など、重複ファイルらしい名前を検出します。 |
| `--hash` | SHA-256 が一致する完全重複ファイルを検出します。 |
| `--collect` | 候補へのシンボリックリンクを指定ディレクトリにまとめます。 |

探索対象は指定したディレクトリ配下だけです。
`.git`、`target`、`node_modules`、`.junk-links` は自動で除外されます。

## はじめる

リポジトリを取得して、Rust の標準ツールチェーンでビルドできます。

```bash
git clone https://github.com/kana112/hanz.git
cd hanz
cargo build --release
```

ビルド後の実行ファイルは `target/release/hanz` に作成されます。

## 基本の使い方

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

候補を確認しやすくするため、シンボリックリンクだけを `.junk-links` にまとめられます。

```bash
cargo run -- ./Downloads --name --collect .junk-links
cargo run -- ./Downloads --hash --collect .junk-links
```

> [!WARNING]
> `--collect` は候補ファイル本体を変更しません。ただし、指定した収集先ディレクトリは作り直されます。

## 出力例

```text
NAME  ./Downloads/report (1).pdf
      reason: duplicate-like filename

HASH  ./Downloads/a.pdf
      duplicate of: ./Downloads/b.pdf
      sha256: xxxxx
```

候補がない場合は `No candidates found.` と表示します。

## 判定方法

`--name` は、ファイル名に `コピー`、` copy`、` Copy`、` (数字)` が含まれる場合や、拡張子直前が ` 2` 以上の数字で終わる場合に候補とします。

`--hash` は、最初にファイルサイズでグループ化します。同じサイズのファイルが2つ以上あるグループだけをバッファ読み込みで SHA-256 計算し、ハッシュが一致したファイルを候補とします。

## 安全性

- 通常ファイルだけを対象にします。
- ディレクトリ、シンボリックリンク、特殊ファイルはスキップします。
- `--collect` を指定しない限り、ファイルシステムを変更しません。
- 収集先が探索対象自身、またはその親ディレクトリの場合はエラーにします。

## 開発

```bash
cargo test --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
```
