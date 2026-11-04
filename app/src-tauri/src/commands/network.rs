use super::*;

#[derive(Serialize)]
pub struct Adapter {
    pub name: String,
    pub ip: String,
    pub kind: String,
}

#[tauri::command]
pub fn list_adapters() -> Vec<Adapter> {
    let mut adapters = Vec::new();
    if let Ok(list) = if_addrs::get_if_addrs() {
        for item in list {
            let if_addrs::IfAddr::V4(v4) = item.addr else {
                continue;
            };
            if v4.ip.is_loopback() {
                continue;
            }
            let octets = v4.ip.octets();
            let kind = if octets[0] == 25 {
                "Hamachi"
            } else if octets[0] == 26 {
                "Radmin VPN"
            } else if octets[0] == 100 && (64..=127).contains(&octets[1]) {
                "Tailscale"
            } else {
                "LAN"
            };
            adapters.push(Adapter {
                name: item.name,
                ip: v4.ip.to_string(),
                kind: kind.to_string(),
            });
        }
    }
    adapters.sort_by_key(|item| match item.kind.as_str() {
        "Hamachi" => 0,
        "Radmin VPN" => 1,
        "Tailscale" => 2,
        _ => 3,
    });
    adapters
}

#[tauri::command]
pub fn write_share_files(
    state: State<AppState>,
    ip: String,
    http_port: u16,
) -> Result<Vec<String>, String> {
    let dir = state.server_dir();
    let sections = config_sections(&state);
    let name = section_value(&sections, "SERVER", "NAME").unwrap_or_else(|| "AC Server".into());
    let password = section_value(&sections, "SERVER", "PASSWORD").unwrap_or_default();
    let track = section_value(&sections, "SERVER", "TRACK").unwrap_or_default();
    let cars = used_cars(&sections).join(", ");
    let link = format!(
        "acmanager://race/online/join?ip={}&httpPort={}",
        ip, http_port
    );

    let url_path = dir.join("entra.url");
    write_text(
        &url_path,
        &format!("[InternetShortcut]\r\nURL={}\r\n", link),
    )?;

    let local_link = format!(
        "acmanager://race/online/join?ip=127.0.0.1&httpPort={}",
        http_port
    );
    let local_path = dir.join("entra-locale.url");
    write_text(
        &local_path,
        &format!("[InternetShortcut]\r\nURL={}\r\n", local_link),
    )?;

    let info = format!(
        "Server      : {}\nIP          : {}\nPista       : {}\nAuto        : {}\nPassword    : {}\nPorta gioco : UDP 9600\nPorta HTTP  : {}\n\nLink per Content Manager:\n{}\n\nLauncher originale: ONLINE > LAN, oppure IP {} porta 9600\n",
        name,
        ip,
        track,
        cars,
        if password.is_empty() { "(nessuna)" } else { "(impostata)" },
        http_port,
        link,
        ip
    );
    let info_path = dir.join("server-info.txt");
    write_text(&info_path, &info)?;

    Ok(vec![
        url_path.to_string_lossy().to_string(),
        local_path.to_string_lossy().to_string(),
        info_path.to_string_lossy().to_string(),
    ])
}

#[derive(Serialize)]
pub struct PortState {
    pub port: u16,
    pub tcp_free: bool,
    pub udp_free: bool,
}

/// Porte locali in ascolto lette da `netstat -ano`, separate fra TCP e UDP.
/// Su Windows il bind di prova riesce anche quando un altro processo ascolta
/// su `0.0.0.0`, quindi non basta provare a legare la porta.
fn listening_ports() -> Option<(HashSet<u16>, HashSet<u16>)> {
    #[cfg(windows)]
    let mut command: Command = {
        let mut command = Command::new("netstat");
        command.args(["-ano"]);
        command.creation_flags(CREATE_NO_WINDOW);
        command
    };
    #[cfg(not(windows))]
    let mut command: Command = {
        let mut command = Command::new("netstat");
        command.args(["-ano"]);
        command
    };
    let output = command.output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(parse_netstat(&String::from_utf8_lossy(&output.stdout)))
}

fn parse_netstat(text: &str) -> (HashSet<u16>, HashSet<u16>) {
    let mut tcp = HashSet::new();
    let mut udp = HashSet::new();
    for line in text.lines() {
        let columns: Vec<&str> = line.split_whitespace().collect();
        if columns.len() < 4 {
            continue;
        }
        let Some((_, port)) = columns[1].rsplit_once(':') else {
            continue;
        };
        let Ok(port) = port.parse::<u16>() else {
            continue;
        };
        // Solo il socket in ascolto del server: le connessioni dei client hanno
        // la porta locale effimera e non devono marcare la porta come occupata.
        if columns[0].eq_ignore_ascii_case("TCP") && columns[3].eq_ignore_ascii_case("LISTENING") {
            tcp.insert(port);
        } else if columns[0].eq_ignore_ascii_case("UDP") {
            udp.insert(port);
        }
    }
    (tcp, udp)
}

#[tauri::command]
pub fn check_ports(ports: Vec<u16>) -> Vec<PortState> {
    let table = listening_ports();
    ports
        .into_iter()
        .map(|port| {
            let (tcp_free, udp_free) = match &table {
                Some((tcp, udp)) => (!tcp.contains(&port), !udp.contains(&port)),
                None => (
                    std::net::TcpListener::bind(("0.0.0.0", port)).is_ok(),
                    std::net::UdpSocket::bind(("0.0.0.0", port)).is_ok(),
                ),
            };
            PortState {
                port,
                tcp_free,
                udp_free,
            }
        })
        .collect()
}

#[cfg(test)]
mod port_tests {
    use super::parse_netstat;

    const SAMPLE: &str = "\r\nAttività connessioni\r\n\r\n  Proto  Indirizzo locale        Indirizzo esterno       Stato           PID\r\n  TCP    0.0.0.0:9600           0.0.0.0:0              LISTENING       29144\r\n  TCP    127.0.0.1:51044          127.0.0.1:8081         TIME_WAIT       0\r\n  TCP    [::]:8081              [::]:0                 LISTENING       29144\r\n  UDP    0.0.0.0:9600           *:*                                    29144\r\n  TCP    127.0.0.1:443           0.0.0.0:0              LISTENING       4\r\n";

    #[test]
    fn marks_listening_ports_only() {
        let (tcp, udp) = parse_netstat(SAMPLE);
        assert!(tcp.contains(&9600));
        assert!(tcp.contains(&8081));
        assert!(udp.contains(&9600));
        assert!(!tcp.contains(&51044));
        assert!(!udp.contains(&8081));
    }
}

#[tauri::command]
pub async fn server_info(port: u16) -> Result<serde_json::Value, String> {
    let url = format!("http://127.0.0.1:{}/INFO", port);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .map_err(|error| error.to_string())?;
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|error| error.to_string())?;
    let text = response.text().await.map_err(|error| error.to_string())?;
    serde_json::from_str(&text).map_err(|error| format!("{} (risposta: {})", error, text))
}

#[tauri::command]
pub fn open_firewall(state: State<AppState>) -> Result<(), String> {
    let script = state.server_dir().join("apri-porte-firewall.ps1");
    if !script.is_file() {
        return Err(messages::tr(
            "apri-porte-firewall.ps1 non trovato",
            "apri-porte-firewall.ps1 not found",
        ));
    }
    let path = script.to_string_lossy().replace('\'', "''");
    let command = format!(
        "Start-Process powershell -ArgumentList '-NoProfile','-ExecutionPolicy','Bypass','-File','{}' -Verb RunAs",
        path
    );
    let mut process = Command::new("powershell");
    process.args([
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-Command",
        &command,
    ]);
    #[cfg(windows)]
    process.creation_flags(CREATE_NO_WINDOW);
    process.spawn().map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn open_path(path: String) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err(messages::tr("percorso vuoto", "empty path"));
    }
    #[cfg(windows)]
    {
        let mut process = Command::new("cmd");
        process.args(["/C", "start", "", path.as_str()]);
        process.creation_flags(CREATE_NO_WINDOW);
        process.spawn().map_err(|error| error.to_string())?;
    }
    Ok(())
}

/// Apre solo URI http/https/acmanager, sempre tra virgolette: niente injection
/// di metacaratteri della shell.
#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    let value = url.trim();
    let allowed = ["acmanager://", "http://", "https://"]
        .iter()
        .any(|prefix| value.to_lowercase().starts_with(prefix));
    if !allowed {
        return Err(messages::tr("URL non consentito", "URL not allowed"));
    }
    if value
        .chars()
        .any(|c| c.is_control() || c == '"' || c == '<' || c == '>')
    {
        return Err(messages::tr("URL non valido", "invalid URL"));
    }
    #[cfg(windows)]
    {
        // `raw_arg` tiene le virgolette: senza, `Command` non quota l'URL e cmd
        // spezza il link sul `&` (`httpPort=...` diventa un comando a sé).
        let mut process = Command::new("cmd");
        process.arg("/C");
        process.raw_arg(format!("start \"\" \"{}\"", value));
        process.creation_flags(CREATE_NO_WINDOW);
        process.spawn().map_err(|error| error.to_string())?;
    }
    Ok(())
}

/// `true` se Windows ha un gestore registrato per `acmanager://` (Content Manager).
#[tauri::command]
pub fn cm_protocol_ready() -> bool {
    #[cfg(windows)]
    {
        let mut process = Command::new("reg");
        process.args([
            "query",
            r"HKCU\Software\Classes\acmanager\shell\open\command",
            "/ve",
        ]);
        process.creation_flags(CREATE_NO_WINDOW);
        process
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }
    #[cfg(not(windows))]
    {
        false
    }
}

// ---------------------------------------------------------------- content manager
