# coreserver-cli

CORESERVER V2 をコマンドラインから操作する CLI。V2 の中身は DirectAdmin なので、その API を叩く。
API（2222番）は外から届かないため、SSH でサーバーに入り、サーバー上の `curl` に実行させる。

## 必要なもの

- `ssh` コマンドで対象サーバーに入れること（鍵認証を推奨）
- Rust（ビルド時のみ）

## インストール

```bash
cargo install --path .
```

## 使い方

```bash
coreserver login <ssh接続先>      # 例: coreserver login user@v2008.coreserver.jp
coreserver domains
coreserver mail list example.com
coreserver forward add info@example.com me@example.net
coreserver catchall set example.com box@example.com
coreserver get CMD_API_SHOW_USER_CONFIG   # 任意の DirectAdmin API
```

`coreserver` を引数なしで実行すると全コマンドが出る。

`login` はサーバーアカウントのパスワードを一度だけ聞き、DirectAdmin のログインキーを作って
`~/.coreserver` に保存する。パスワード自体は保存しない。別アカウントは環境変数
`CORESERVER_CONFIG` で設定ファイルを切り替える。

## License

MIT OR Apache-2.0
