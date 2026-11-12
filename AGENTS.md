# Notes for AI agents

`coreserver` operates a CORESERVER V2 hosting account (DirectAdmin) from the shell.
Use it instead of driving the web control panel (cp.coreserver.jp) in a browser.

## Using the CLI

- Always pass `--json`: stdout is then exactly one line of JSON (the DirectAdmin response).
- Exit code 0 = success, 1 = failure. The reason is on stderr; stdout is empty on failure.
- `coreserver --help` lists every command. There are no hidden subcommands.
- If you get `... がありません。先に coreserver login を実行してください` or `Not logged in`,
  stop and ask the user to run `coreserver login <ssh-destination>` themselves.
  **Never run `login` or `mail add` yourself**: they prompt for a password on the terminal.
- Each call opens one SSH connection (about a second). Do not loop tightly.

## Read-only commands (safe to run without asking)

```
coreserver domains
coreserver usage
coreserver db list
coreserver cron list
coreserver mail list <domain>
coreserver forward list <domain>
coreserver catchall get <domain>
coreserver subdomain list <domain>
coreserver get <CMD_API_...> [key=value ...]
```

## Commands that change the account (confirm with the user first)

| Command | Effect | Undo |
|---|---|---|
| `forward add <addr> <dest[,dest]>` | Creates a forwarder. Fails if it already exists. | `forward del` |
| `forward del <addr>` | Deletes a forwarder. | `forward add` |
| `catchall set <domain> <addr\|:fail:\|:blackhole:>` | Replaces the catch-all. Read the old value with `catchall get` first. | `catchall set` with the old value |
| `subdomain add <domain> <sub>` | Creates the subdomain and `~/domains/<sub>.<domain>/`. | `subdomain del` |
| `subdomain del <domain> <sub>` | Removes the subdomain. The folder is kept. | `subdomain add` |
| `mail del <addr>` | **Deletes the mailbox and all its mail. Cannot be undone.** | none |
| `post <CMD_API_...> ...` | Anything the account can do, including deletion. | depends |

## Raw API

`get` and `post` call any legacy DirectAdmin API (`CMD_API_*`); parameters are `key=value`
arguments and `json=yes` is added for you. Reference: https://www.directadmin.com/api.php

```
coreserver get CMD_API_POP action=full_list domain=example.com --json
```

An account's domains all live on one server. Addresses are split at `@` into `user` and `domain`.

## Working on this repository

- One file: `src/main.rs`. Keep it that way; add a command as one arm of the `match` in `run`.
- `cargo test` runs offline unit tests. There is no mock server; verify API changes against a
  real account with a throwaway name (e.g. `zz-cli-test`) and clean up afterwards.
- Secrets go to the server only through curl's stdin config (`curl -K -`), never in argv.
