//! The `monita` command line, run as a process: setup, the env file, whereis, admin.
//! `service` needs root and systemd; scripts/smoke-cli.sh covers it in CI.

use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

use monita::platform::db::Db;

/// `monita <args>` with only PATH from this process, so no MONITA_* leaks in, and `stdin` fed.
fn monita(args: &[&str], env: &[(&str, &str)], stdin: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_monita"))
        .args(args)
        .env_clear()
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        .envs(env.iter().copied())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn out(o: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

struct Host {
    _dir: tempfile::TempDir,
    root: PathBuf,
    env_file: String,
}

impl Host {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let env_file = root.join("etc/monita.env").display().to_string();
        Self {
            _dir: dir,
            root,
            env_file,
        }
    }

    fn path(&self, p: &str) -> String {
        self.root.join(p).display().to_string()
    }

    fn file(&self) -> String {
        std::fs::read_to_string(&self.env_file).unwrap()
    }

    /// The answers `setup --yes` takes from the environment.
    fn answers(&self) -> Vec<(&'static str, String)> {
        vec![
            ("MONITA_BASE_URL", "https://monita.example.com".into()),
            ("MONITA_BIND", "127.0.0.1:18099".into()),
            ("MONITA_DB_PATH", self.path("data/monita.db")),
            ("MONITA_BACKUP_DIR", self.path("backups")),
            ("MONITA_ADMIN_EMAIL", "root@example.com".into()),
            ("MONITA_ADMIN_PASSWORD", "a-long-admin-password".into()),
        ]
    }

    fn setup_yes(&self) -> Output {
        let answers = self.answers();
        let env: Vec<(&str, &str)> = answers.iter().map(|(k, v)| (*k, v.as_str())).collect();
        monita(&["setup", "--yes", "--env-file", &self.env_file], &env, "")
    }
}

async fn can_sign_in(db_path: &str, email: &str, password: &str) -> bool {
    let db = Db::open(Path::new(db_path)).await.unwrap();
    monita::accounts::login(&db, &Default::default(), email, password)
        .await
        .is_ok()
}

#[tokio::test]
async fn setup_writes_the_env_file_and_the_first_admin_and_a_second_run_changes_nothing() {
    let host = Host::new();
    let o = host.setup_yes();
    assert!(o.status.success(), "{}", out(&o));
    assert!(
        out(&o).contains("created super admin root@example.com"),
        "{}",
        out(&o)
    );

    let file = host.file();
    for line in [
        "MONITA_ENV=prod",
        "MONITA_BASE_URL=https://monita.example.com",
        "MONITA_BIND=127.0.0.1:18099",
        "MONITA_TRUST_PROXY=true",
        "MONITA_MAIL_FROM='Monita <monita@monita.example.com>'",
        "MONITA_SMTP_URL=",
    ] {
        assert!(
            file.lines().any(|l| l == line),
            "{line} missing from:\n{file}"
        );
    }
    assert!(
        !file.contains("PASSWORD"),
        "the admin password stays out of the file"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&host.env_file)
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    let again = host.setup_yes();
    assert!(again.status.success(), "{}", out(&again));
    assert!(
        out(&again).contains("nothing changed in"),
        "{}",
        out(&again)
    );
    assert!(
        out(&again).contains("already exists; kept"),
        "{}",
        out(&again)
    );
    assert_eq!(host.file(), file);
    assert!(
        can_sign_in(
            &host.path("data/monita.db"),
            "root@example.com",
            "a-long-admin-password"
        )
        .await
    );
}

#[tokio::test]
async fn setup_keeps_the_comments_and_variables_it_does_not_ask_about() {
    let host = Host::new();
    std::fs::create_dir_all(host.path("etc")).unwrap();
    std::fs::write(
        &host.env_file,
        "# kept comment\nMONITA_BIND=127.0.0.1:1\nMONITA_PING_URL=https://peer.example.com/ping/x\n",
    )
    .unwrap();
    let o = host.setup_yes();
    assert!(o.status.success(), "{}", out(&o));
    let file = host.file();
    assert!(file.starts_with("# kept comment\nMONITA_BIND=127.0.0.1:18099\nMONITA_PING_URL=https://peer.example.com/ping/x\n"), "{file}");
}

#[tokio::test]
async fn setup_without_an_https_public_url_writes_nothing() {
    let host = Host::new();
    let o = monita(&["setup", "--yes", "--env-file", &host.env_file], &[], "");
    assert!(!o.status.success());
    assert!(out(&o).contains("MONITA_BASE_URL"), "{}", out(&o));
    assert!(!Path::new(&host.env_file).exists());
}

#[tokio::test]
async fn setup_asks_each_value_and_takes_the_default_on_enter() {
    let host = Host::new();
    let db = host.path("data/monita.db");
    // URL, bind, trust proxy, db, backups (- for none), SMTP, sender, admin email, password twice.
    let answers = format!(
        "https://status.example.org\n\nfalse\n{db}\n-\n\n\nboss@example.org\npassword-of-12\npassword-of-12\n"
    );
    let o = monita(&["setup", "--env-file", &host.env_file], &[], &answers);
    assert!(o.status.success(), "{}", out(&o));
    let file = host.file();
    for line in [
        "MONITA_BASE_URL=https://status.example.org",
        "MONITA_BIND=127.0.0.1:7466",
        "MONITA_TRUST_PROXY=false",
        "MONITA_BACKUP_DIR=",
        "MONITA_MAIL_FROM='Monita <monita@status.example.org>'",
    ] {
        assert!(
            file.lines().any(|l| l == line),
            "{line} missing from:\n{file}"
        );
    }
    assert!(can_sign_in(&db, "boss@example.org", "password-of-12").await);
}

#[tokio::test]
async fn setup_refuses_two_different_passwords() {
    let host = Host::new();
    let answers =
        "https://s.example.org\n\n\n\n\n\n\nboss@example.org\npassword-of-12\npassword-of-13\n";
    let o = monita(&["setup", "--env-file", &host.env_file], &[], answers);
    assert!(!o.status.success());
    assert!(out(&o).contains("the two passwords differ"), "{}", out(&o));
}

#[tokio::test]
async fn admin_and_whereis_read_the_env_file() {
    let host = Host::new();
    assert!(host.setup_yes().status.success());
    let ef = host.env_file.as_str();

    let o = monita(
        &[
            "--env-file",
            ef,
            "admin",
            "create",
            "--email",
            "second@example.com",
        ],
        &[],
        "another-long-password\n",
    );
    assert!(o.status.success(), "{}", out(&o));
    let o = monita(
        &[
            "admin",
            "create",
            "--email",
            "second@example.com",
            "--env-file",
            ef,
        ],
        &[],
        "another-long-password\n",
    );
    assert_eq!(o.status.code(), Some(1), "{}", out(&o));
    assert!(out(&o).contains("already exists"), "{}", out(&o));

    let o = monita(&["--env-file", ef, "whereis", "data"], &[], "");
    assert_eq!(
        String::from_utf8_lossy(&o.stdout),
        format!("{}\n", host.path("data/monita.db"))
    );
    let o = monita(&["--env-file", ef, "whereis"], &[], "");
    let text = String::from_utf8_lossy(&o.stdout);
    assert!(text.contains(&format!("env      {ef}\n")), "{text}");
    assert!(
        text.contains(&format!("backups  {}\n", host.path("backups"))),
        "{text}"
    );
}

#[tokio::test]
async fn the_environment_wins_over_the_env_file() {
    let host = Host::new();
    assert!(host.setup_yes().status.success());
    let other = host.path("elsewhere.db");
    let o = monita(
        &["--env-file", &host.env_file, "whereis", "data"],
        &[("MONITA_DB_PATH", &other)],
        "",
    );
    assert_eq!(String::from_utf8_lossy(&o.stdout), format!("{other}\n"));
}
