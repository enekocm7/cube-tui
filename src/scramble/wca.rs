use super::WcaEvent;
use reqwest::blocking::Client;
use serde::Deserialize;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

const SCRAMBLE_API_URL: &str = "http://127.0.0.1:3311";
const STARTUP_RETRIES: usize = 30;
const STARTUP_WAIT: Duration = Duration::from_millis(100);

static WCA_ENABLED: AtomicBool = AtomicBool::new(false);
static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();

pub struct WcaScrambleServer {
    child: Option<Child>,
}

impl Drop for WcaScrambleServer {
    fn drop(&mut self) {
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

pub fn start_wca_scramble_server() -> Result<WcaScrambleServer, String> {
    if is_server_ready() {
        WCA_ENABLED.store(true, Ordering::Relaxed);
        return Ok(WcaScrambleServer { child: None });
    }

    let scrambles_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scrambles");
    if !scrambles_dir.is_dir() {
        return Err("Missing scrambles directory next to Cargo.toml".to_string());
    }

    for (program, args) in runtimes() {
        if !runtime_exists(program) {
            continue;
        }

        let Ok(mut child) = Command::new(program)
            .args(args)
            .current_dir(&scrambles_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        else {
            continue;
        };

        for _ in 0..STARTUP_RETRIES {
            if is_server_ready() {
                WCA_ENABLED.store(true, Ordering::Relaxed);
                return Ok(WcaScrambleServer { child: Some(child) });
            }

            if let Ok(Some(_)) = child.try_wait() {
                break;
            }

            thread::sleep(STARTUP_WAIT);
        }

        let _ = child.kill();
        let _ = child.wait();
    }

    Err("Could not start WCA scrambles server (tried Node, Bun, and Deno)".to_string())
}

pub fn fetch_wca_scramble(event: WcaEvent) -> Option<String> {
    if !WCA_ENABLED.load(Ordering::Relaxed) {
        return None;
    }

    let url = format!("{SCRAMBLE_API_URL}/scramble/{}", event_api_id(event));
    let response = http_client().get(url).send().ok()?;
    if !response.status().is_success() {
        return None;
    }

    let body: ScrambleApiResponse = response.json().ok()?;
    let scramble = body.scramble.trim();
    if scramble.is_empty() {
        return None;
    }

    Some(scramble.to_string())
}

#[derive(Deserialize)]
struct ScrambleApiResponse {
    scramble: String,
}

const fn runtimes() -> [(&'static str, &'static [&'static str]); 3] {
    [
        ("node", &["index.ts"]),
        ("bun", &["index.ts"]),
        (
            "deno",
            &[
                "run",
                "--allow-net",
                "--allow-read",
                "--allow-env",
                "--node-modules-dir=auto",
                "index.ts",
            ],
        ),
    ]
}

fn runtime_exists(program: &str) -> bool {
    Command::new(program)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn is_server_ready() -> bool {
    let Ok(response) = http_client()
        .get(format!("{SCRAMBLE_API_URL}/events"))
        .send()
    else {
        return false;
    };

    if !response.status().is_success() {
        return false;
    }

    response
        .json::<Vec<String>>()
        .is_ok_and(|events| events.iter().any(|event| event == "333"))
}

fn http_client() -> &'static Client {
    HTTP_CLIENT.get_or_init(|| {
        Client::builder()
            .timeout(Duration::from_millis(900))
            .build()
            .expect("failed to create scramble API client")
    })
}

const fn event_api_id(event: WcaEvent) -> &'static str {
    match event {
        WcaEvent::Cube2x2 => "222",
        WcaEvent::Cube3x3 => "333",
        WcaEvent::Cube4x4 => "444",
        WcaEvent::Cube5x5 => "555",
        WcaEvent::Cube6x6 => "666",
        WcaEvent::Cube7x7 => "777",
        WcaEvent::Megaminx => "minx",
        WcaEvent::Pyraminx => "pyram",
        WcaEvent::Skewb => "skewb",
        WcaEvent::Square1 => "sq1",
        WcaEvent::Clock => "clock",
    }
}
