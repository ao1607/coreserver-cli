# coreserver-cli

ブラウザでコントロールパネルを開かずに CORESERVER V2 を操作するためのコマンドラインツールです。

```console
$ coreserver forward add info@example.com me@example.net
$ coreserver catchall get example.com
box@example.com
```

CORESERVER V2 は DirectAdmin 上で動作しているため、このツールは DirectAdmin API を呼び出します。API ポート（2222）はサーバー外からアクセスできないため、ツールは SSH 経由で接続し、サーバー上の `curl` にリクエストを行わせます。

約 300KB の単一の実行ファイルであり、実行時に必要となるのは `ssh` コマンドのみです。

## 必要なもの

- CORESERVER V2 のアカウントです。V1 やその他の旧プランは DirectAdmin を使用していないため、サポート対象外となっています。
- `ssh` コマンドと、サーバーに登録済みの SSH キー（コントロールパネルの「ツール」→「SSH設定」内）です。
- Rust は、ビルド時のみです。

## インストール

[Releases](https://github.com/ao1607/coreserver-cli/releases) からお使いの OS 用のファイルをダウンロードして展開し、`coreserver` を PATH の通ったフォルダに置いてください。

各リリースには `SHA256SUMS` ファイルが付属しています。また、`gh attestation verify <file> --repo ao1607/coreserver-cli` を実行することで、このリポジトリからファイルがビルドされたことを確認できます。

代わりにソースからビルドする場合は、このリポジトリをクローンして以下のコマンドを実行してください。

```bash
cargo install --path .
```

## ログイン

最初に一度だけログインします。引数は `ssh` コマンドに渡すものと同じです。

```bash
coreserver login user@v0000.coreserver.jp
```

サーバーアカウントのパスワードを一度だけ入力するよう求められます。ツールはこのパスワードを使用して DirectAdmin のログインキーを生成し、そのキーのみを `~/.coreserver` に保存します。パスワード自体は一切保存されません。

複数のアカウントを使用する場合は、環境変数 `CORESERVER_CONFIG` にアカウントごとに異なるファイルを指定してください。

## コマンド

| コマンド | 内容 |
|---|---|
| `coreserver domains` | ドメイン一覧を表示します |
| `coreserver usage` | ディスクやメールなどの使用状況を表示します |
| `coreserver db list` | データベース一覧を表示します |
| `coreserver cron list` | CRON ジョブの一覧を表示します |
| `coreserver mail list <domain>` | ドメインのメールアカウント一覧を表示します |
| `coreserver mail add <addr> [MB]` | メールアカウントを作成します（パスワードの入力が求められます。MB 単位の容量制限は任意です） |
| `coreserver mail del <addr>` | メールアカウントを削除します |
| `coreserver forward list <domain>` | ドメインの転送設定を一覧表示します |
| `coreserver forward add <addr> <転送先>` | 転送設定を作成します |
| `coreserver forward del <addr>` | 転送設定を削除します |
| `coreserver catchall get <domain>` | キャッチオールアドレスを表示します |
| `coreserver catchall set <domain> <addr>` | キャッチオールアドレスを設定します（`:fail:` はメールを拒否し、`:blackhole:` は破棄します） |
| `coreserver subdomain list <domain>` | サブドメイン一覧を表示します |
| `coreserver subdomain add <domain> <sub>` | サブドメインを作成します |
| `coreserver subdomain del <domain> <sub>` | サブドメインを削除します（フォルダは保持されます） |
| `coreserver get\|post <CMD_API_...> [key=value ...]` | 任意の DirectAdmin API を呼び出します |

上記に記載されていないことは、DirectAdmin の任意の API を直接呼び出す `get` および `post` で実現可能です。

```bash
coreserver get CMD_API_SHOW_USER_CONFIG
coreserver get CMD_API_POP action=full_list domain=example.com
```

API の一覧は [DirectAdmin の資料](https://www.directadmin.com/api.php) にあります。

## AI エージェントから使う

このツールは、Claude Code などの AI エージェントが、ブラウザでコントロールパネルを操作する代わりに利用できるように設計されています。

- 任意のコマンドに `--json` を付けると、API のレスポンスを JSON の1行として取得できます。
- 終了コードは、成功時は 0、失敗時は 1 です。失敗の理由は標準エラー出力に書き出されます。
- `login` と `mail add` だけが入力を求めます。これらはパスワードを扱うためです。この2つは人間が実行します。それ以外のコマンドは何も求めずに実行されます。
- `coreserver --help` では、すべてのコマンドが1画面に表示されます。
- [AGENTS.md](AGENTS.md) には、どのコマンドで何かを変更または削除するかなど、エージェントが最初に読むべき注意事項が記載されています。

```console
$ coreserver forward list example.com --json
{"info":["me@example.net"],"sales":["a@example.net","b@example.net"]}
```

## 安全性

- ログインキーとパスワードは標準入力経由で `curl` に渡されるため、共用サーバーのプロセス一覧には表示されません。
- ログインキーは、アカウントでできるすべての操作が可能です。`~/.coreserver` を他人に読まれないようにしてください。
- ログインキーが漏洩した可能性がある場合や、本ツールの使用を停止する際は、`coreserver logout` を実行してください。サーバー上のログインキーを削除し、`~/.coreserver` も削除します。
- これは非公式のツールです。CORESERVER を運営する GMO デジロック社とは一切関係ありません。

## 動作確認

`mail add` と `mail del` を除くすべてのコマンドは、実際の CORESERVER V2 サーバーで動作確認済みです。

## License

[MIT](LICENSE-MIT) または [Apache-2.0](LICENSE-APACHE)
