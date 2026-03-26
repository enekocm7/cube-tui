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
    for Runtime { name, run, install } in runtimes() {
        eprintln!("Trying runtime: {name}");
        if !runtime_exists(name) {
            eprintln!("  {name} not found, skipping");
            continue;
        }
        eprintln!("  Running {name} install...");
        let install_result = Command::new(install[0])
            .args([install[1]])
            .current_dir(&scrambles_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        if !install_result.is_ok_and(|s| s.success()) {
            eprintln!("  Install failed for {name}");
            continue;
        }
        eprintln!("  Install succeeded for {name}");
        eprintln!("  Starting {name} server...");
        let Ok(mut child) = Command::new(name)
            .arg(run)
            .current_dir(&scrambles_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        else {
            eprintln!("  Failed to spawn {name}");
            continue;
        };
        for _ in 0..STARTUP_RETRIES {
            if is_server_ready() {
                eprintln!("  Server ready!");
                WCA_ENABLED.store(true, Ordering::Relaxed);
                return Ok(WcaScrambleServer { child: Some(child) });
            }
            if let Ok(Some(_)) = child.try_wait() {
                eprintln!("  Server exited early");
                break;
            }
            thread::sleep(STARTUP_WAIT);
        }
        eprintln!("  Server failed to start, killing...");
        let _ = child.kill();
        let _ = child.wait();
    }
    Err("Could not start WCA scrambles server (tried Node, Bun)".to_string())
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

struct Runtime<'a> {
    name: &'a str,
    run: &'a str,
    install: [&'a str; 2],
}

impl<'a> Runtime<'a> {
    const fn new(name: &'a str, install_name: &'a str) -> Self {
        Self {
            name,
            run: "index.ts",
            install: [install_name, "install"],
        }
    }
}

fn runtimes() -> Vec<Runtime<'static>> {
    vec![Runtime::new("bun", "bun"), Runtime::new("node", "npm")]
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
