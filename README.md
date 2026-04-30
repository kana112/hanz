# hanz
![Static Badge](https://img.shields.io/badge/License-GPL-blue)
![example workflow](https://github.com/kana112/hanz/.github/workflows/build.yml/badge.svg)
[![Coverage Status](https://coveralls.io/repos/github/kana112/hanz/badge.svg?branch=main)](https://coveralls.io/github/kana112/hanz?branch=main)


必要ないファイルを推奨して表示するlsコマンドです．

# 概要
hanzは，ディレクトリ内の「不要かもしれないファイル」を探索するためのツールです．
通常の重複検出とは異なり，ゆるい判定ルールを用いて，名前ベースで重複候補を抽出します．

デフォルトでは，指定したパスから親ディレクトリを遡り探索を行います．

# 使い方
カレントディレクトリから検索
```
hanz .
```

任意のディレクトリから検索
```
hanz /Users/username/Documents
```


# インストール方法

# プロジェクトについて
