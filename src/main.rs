// CORESERVER V2 は DirectAdmin で動いている。DirectAdmin の API (2222番) は外から届かないので、
// SSH でサーバーに入り、サーバー上の curl に叩かせる。認証は login で作るログインキー。
use serde_json::Value;
use std::io::Write;
use std::path::PathBuf;
use std::process::{exit, Command, Stdio};
use std::{env, fs};

type R<T> = Result<T, String>;

const USAGE: &str = "coreserver - CORESERVER V2 CLI (DirectAdmin API over SSH)

  coreserver login <ssh接続先> [ユーザー名] [https://ホスト:2222]
  coreserver domains | usage | db list | cron list
  coreserver mail list <domain> | add <addr> [容量MB] | del <addr>
  coreserver forward list <domain> | add <addr> <転送先[,転送先]> | del <addr>
  coreserver catchall get <domain> | set <domain> <addr|:fail:|:blackhole:>
  coreserver subdomain list <domain> | add <domain> <sub> | del <domain> <sub>
  coreserver get|post <CMD_API_...> [key=value ...]   任意の DirectAdmin API

  --json   どのコマンドでも API の応答を JSON のまま1行で出す（スクリプト・AI エージェント向け）

終了コード: 0=成功 / 1=失敗（理由は標準エラー）。対話入力があるのは login と mail add だけ。
設定ファイル: ~/.coreserver（環境変数 CORESERVER_CONFIG で別ファイル＝別アカウント）";

struct Cfg {
    ssh: String,
    user: String,
    url: String,
    key: String,
}

fn cfg_path() -> PathBuf {
    if let Ok(p) = env::var("CORESERVER_CONFIG") {
        return p.into();
    }
    let home = env::var("USERPROFILE").or_else(|_| env::var("HOME")).unwrap_or_default();
    PathBuf::from(home).join(".coreserver")
}

fn load() -> R<Cfg> {
    let p = cfg_path();
    let s = fs::read_to_string(&p)
        .map_err(|_| format!("{} がありません。先に coreserver login を実行してください", p.display()))?;
    let get = |k: &str| {
        s.lines()
            .find_map(|l| l.strip_prefix(k)?.strip_prefix('='))
            .map(str::to_string)
            .ok_or(format!("{} に {k} がありません", p.display()))
    };
    Ok(Cfg { ssh: get("ssh")?, user: get("user")?, url: get("url")?, key: get("key")? })
}

fn save(c: &Cfg) -> R<()> {
    let p = cfg_path();
    let body = format!("ssh={}\nuser={}\nurl={}\nkey={}\n", c.ssh, c.user, c.url, c.key);
    let mut o = fs::OpenOptions::new();
    o.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut o, 0o600);
    o.open(&p).and_then(|mut f| f.write_all(body.as_bytes())).map_err(|e| format!("{}: {e}", p.display()))
}

/// curl の設定ファイル (-K) 用の引用。
fn q(s: &str) -> R<String> {
    if s.contains(['\n', '\r']) {
        return Err("値に改行は使えません".into());
    }
    Ok(format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")))
}

fn curl_config(url: &str, userpass: &str, cmd: &str, post: bool, params: &[(&str, &str)]) -> R<String> {
    if !cmd.starts_with("CMD_") || !cmd.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
        return Err(format!("API 名が不正です: {cmd}"));
    }
    let mut c = format!("silent\nshow-error\nmax-time = 60\nurl = {}\nuser = {}\n", q(&format!("{url}/{cmd}"))?, q(userpass)?);
    if !post {
        c.push_str("get\n");
    }
    for (k, v) in params.iter().chain(&[("json", "yes")]) {
        c.push_str(&format!("data-urlencode = {}\n", q(&format!("{k}={v}"))?));
    }
    Ok(c)
}

fn ssh(dest: &str, remote: &str, stdin: &str) -> R<String> {
    // 鍵やパスワードは引数に載せず標準入力で渡す（共用サーバーの ps に出さない）
    let mut ch = Command::new("ssh")
        .args([dest, remote])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("ssh を起動できません: {e}"))?;
    ch.stdin.take().unwrap().write_all(stdin.as_bytes()).map_err(|e| e.to_string())?;
    let o = ch.wait_with_output().map_err(|e| e.to_string())?;
    if !o.status.success() {
        return Err(format!("ssh/curl 失敗: {}", String::from_utf8_lossy(&o.stderr).trim()));
    }
    Ok(String::from_utf8_lossy(&o.stdout).into_owned())
}

fn api_err(v: &Value) -> Option<String> {
    let s = |k| v.get(k).and_then(Value::as_str).unwrap_or("");
    let failed = !matches!(s("error"), "" | "0") || s("success") == "no";
    failed.then(|| format!("{} {} {}", s("error"), s("text"), s("details")).trim().to_string())
}

fn call(ssh_dest: &str, url: &str, userpass: &str, cmd: &str, post: bool, params: &[(&str, &str)]) -> R<Value> {
    let out = ssh(ssh_dest, "curl -K -", &curl_config(url, userpass, cmd, post, params)?)?;
    let v: Value = serde_json::from_str(&out)
        .map_err(|_| format!("JSON でない応答: {}", out.chars().take(300).collect::<String>()))?;
    match api_err(&v) {
        Some(e) => Err(format!("API エラー: {e}")),
        None => Ok(v),
    }
}

fn show(v: &Value) {
    match v.as_array() {
        Some(a) if a.iter().all(Value::is_string) => a.iter().for_each(|x| println!("{}", x.as_str().unwrap())),
        _ => println!("{}", serde_json::to_string_pretty(v).unwrap()),
    }
}

fn addr(s: &str) -> R<(&str, &str)> {
    s.split_once('@').filter(|(u, d)| !u.is_empty() && d.contains('.')).ok_or(format!("メールアドレスではありません: {s}"))
}

fn hex(n: usize) -> R<String> {
    let mut b = vec![0u8; n];
    getrandom::getrandom(&mut b).map_err(|e| e.to_string())?;
    Ok(b.iter().map(|x| format!("{x:02x}")).collect())
}

fn prompt(msg: &str) -> R<String> {
    rpassword::prompt_password(msg).map_err(|e| e.to_string())
}

fn login(dest: &str, rest: &[&str]) -> R<()> {
    let probe = ssh(dest, "echo $USER; hostname -f", "")?;
    let mut l = probe.lines();
    let user = rest.first().copied().or(l.next()).unwrap_or("").to_string();
    let host = l.next().unwrap_or("");
    let url = rest.get(1).map(|s| s.trim_end_matches('/').to_string()).unwrap_or(format!("https://{host}:2222"));
    eprintln!("ユーザー {user} / API {url}");
    let pw = prompt("サーバーアカウントのパスワード（保存しません）: ")?;
    let key = hex(24)?;
    let name = format!("cscli{}", hex(4)?);
    call(dest, &url, &format!("{user}:{pw}"), "CMD_API_LOGIN_KEYS", true, &[
        ("action", "create"), ("type", "key"), ("keyname", &name), ("key", &key), ("key2", &key),
        ("never_expires", "yes"), ("max_uses", "0"), ("clear_key", "yes"), ("allow_htm", "yes"), ("passwd", &pw),
    ])?;
    let c = Cfg { ssh: dest.into(), user, url, key };
    save(&c)?;
    call(&c.ssh, &c.url, &format!("{}:{}", c.user, c.key), "CMD_API_SHOW_DOMAINS", false, &[])?;
    eprintln!("ログインキー {name} を作成し {} に保存しました", cfg_path().display());
    Ok(())
}

fn run(a: &[&str]) -> R<()> {
    let json = a.contains(&"--json");
    let a: Vec<&str> = a.iter().copied().filter(|x| *x != "--json").collect();
    let a = a.as_slice();
    match a {
        ["login", dest, rest @ ..] => return login(dest, rest),
        ["help" | "--help" | "-h"] => {
            println!("{USAGE}");
            return Ok(());
        }
        _ => {}
    }
    let pw;
    let kv: Vec<(&str, &str)>;
    let (cmd, post, params): (&str, bool, &[(&str, &str)]) = match a {
        ["domains"] => ("CMD_API_SHOW_DOMAINS", false, &[]),
        ["usage"] => ("CMD_API_SHOW_USER_USAGE", false, &[]),
        ["db", "list"] => ("CMD_API_DATABASES", false, &[]),
        ["cron", "list"] => ("CMD_API_CRON_JOBS", false, &[]),
        ["mail", "list", d] => ("CMD_API_POP", false, &[("action", "list"), ("domain", d)]),
        ["mail", "add", ad, rest @ ..] => {
            let (u, d) = addr(ad)?;
            pw = prompt(&format!("{ad} の新しいパスワード: "))?;
            if pw != prompt("もう一度: ")? {
                return Err("パスワードが一致しません".into());
            }
            // quota 0 = 無制限
            ("CMD_API_POP", true, &[("action", "create"), ("domain", d), ("user", u), ("passwd", &pw), ("passwd2", &pw),
                ("quota", rest.first().copied().unwrap_or("0")), ("limit", "0")])
        }
        ["mail", "del", ad] => {
            let (u, d) = addr(ad)?;
            ("CMD_API_POP", true, &[("action", "delete"), ("domain", d), ("user", u)])
        }
        ["forward", "list", d] => ("CMD_API_EMAIL_FORWARDERS", false, &[("domain", d)]),
        ["forward", "add", ad, to] => {
            let (u, d) = addr(ad)?;
            ("CMD_API_EMAIL_FORWARDERS", true, &[("action", "create"), ("domain", d), ("user", u), ("email", to)])
        }
        ["forward", "del", ad] => {
            let (u, d) = addr(ad)?;
            ("CMD_API_EMAIL_FORWARDERS", true, &[("action", "delete"), ("domain", d), ("select0", u)])
        }
        ["catchall", "get", d] => ("CMD_API_EMAIL_CATCH_ALL", false, &[("domain", d)]),
        ["catchall", "set", d, v] => {
            let catch = if v.starts_with(':') { v } else { "address" };
            ("CMD_API_EMAIL_CATCH_ALL", true, &[("domain", d), ("update", "Update"), ("catch", catch), ("value", v)])
        }
        ["subdomain", "list", d] => ("CMD_API_SUBDOMAINS", false, &[("domain", d)]),
        ["subdomain", "add", d, s] => ("CMD_API_SUBDOMAINS", true, &[("action", "create"), ("domain", d), ("subdomain", s)]),
        // contents=no: ディレクトリの中身は消さない
        ["subdomain", "del", d, s] => ("CMD_API_SUBDOMAINS", true, &[("action", "delete"), ("domain", d), ("select0", s), ("contents", "no")]),
        [m @ ("get" | "post"), cmd, rest @ ..] => {
            kv = rest.iter().map(|x| x.split_once('=').ok_or(format!("key=value で書いてください: {x}"))).collect::<R<_>>()?;
            (cmd, *m == "post", &kv)
        }
        _ => return Err(USAGE.into()),
    };
    let c = load()?;
    let v = call(&c.ssh, &c.url, &format!("{}:{}", c.user, c.key), cmd, post, params)?;
    // catchall get は選択肢の一覧まで返すので、現在値だけ出す
    match (a, v.get("value").and_then(Value::as_str)) {
        _ if json => println!("{v}"),
        (["catchall", "get", _], Some(s)) => println!("{s}"),
        _ => show(&v),
    }
    Ok(())
}

fn main() {
    let a: Vec<String> = env::args().skip(1).collect();
    if let Err(e) = run(&a.iter().map(String::as_str).collect::<Vec<_>>()) {
        eprintln!("{e}");
        exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_quotes_and_rejects() {
        let c = curl_config("https://h:2222", "u:p\"\\x", "CMD_API_POP", false, &[("a", "b c")]).unwrap();
        assert!(c.contains("user = \"u:p\\\"\\\\x\"\n"));
        assert!(c.contains("get\n") && c.contains("data-urlencode = \"a=b c\"\n") && c.contains("\"json=yes\""));
        assert!(curl_config("u", "x", "CMD_X", true, &[("a", "b\nurl = evil")]).is_err());
        assert!(curl_config("u", "x", "../etc", true, &[]).is_err());
    }

    #[test]
    fn errors_and_addr() {
        assert!(api_err(&serde_json::json!({"error":"1","text":"ng"})).is_some());
        assert!(api_err(&serde_json::json!({"success":"no"})).is_some());
        assert!(api_err(&serde_json::json!({"error":"0"})).is_none());
        assert!(api_err(&serde_json::json!(["a"])).is_none());
        assert_eq!(addr("a@b.c").unwrap(), ("a", "b.c"));
        assert!(addr("ab.c").is_err());
    }
}
