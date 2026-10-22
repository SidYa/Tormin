//! OS integration: file associations, shutdown, translations for native menus.

use std::collections::HashMap;
use std::process::Command;

/// Translations shared with the other versions (lang/*.json), embedded for the tray menu.
pub struct Strings {
    map: HashMap<String, String>,
    fallback: HashMap<String, String>,
}

fn parse(json: &str) -> HashMap<String, String> {
    serde_json::from_str(json).unwrap_or_default()
}

impl Strings {
    pub fn new(lang: &str) -> Strings {
        let fallback = parse(include_str!("../../../lang/en.json"));
        let map = match lang {
            "uk" => parse(include_str!("../../../lang/uk.json")),
            _ => fallback.clone(),
        };
        Strings { map, fallback }
    }

    pub fn t(&self, key: &str) -> String {
        self.map.get(key).or_else(|| self.fallback.get(key)).cloned().unwrap_or_else(|| key.to_string())
    }
}

/// Starts the system shutdown after all downloads are done.
pub fn shutdown() {
    let result = if cfg!(windows) {
        // 60 s grace period: "shutdown /a" cancels it
        Command::new("shutdown").args(["/s", "/t", "60"]).spawn()
    } else {
        Command::new("systemctl").arg("poweroff").spawn()
    };
    if let Err(e) = result {
        crate::log(&format!("Failed to shutdown: {e}"));
    }
}

#[cfg(windows)]
pub fn associate_files() -> Result<(), String> {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    const PROG_ID: &str = "Tormin.torrent";
    const CAPS: &str = r"Software\Tormin\Capabilities";
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe = exe.to_string_lossy();
    let command = format!("\"{exe}\" \"%1\"");
    let icon = format!("\"{exe}\",0");
    let err = |e: std::io::Error| e.to_string();

    // Per-user registration, no administrator rights needed
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (classes, _) = hkcu.create_subkey(r"Software\Classes").map_err(err)?;

    let (prog, _) = classes.create_subkey(PROG_ID).map_err(err)?;
    prog.set_value("", &"Torrent file").map_err(err)?;
    prog.create_subkey("DefaultIcon").map_err(err)?.0.set_value("", &icon).map_err(err)?;
    prog.create_subkey(r"shell\open\command").map_err(err)?.0.set_value("", &command).map_err(err)?;

    let (ext, _) = classes.create_subkey(".torrent").map_err(err)?;
    ext.set_value("", &PROG_ID).map_err(err)?;
    ext.set_value("Content Type", &"application/x-bittorrent").map_err(err)?;
    ext.create_subkey("OpenWithProgids").map_err(err)?.0.set_value(PROG_ID, &"").map_err(err)?;

    let (magnet, _) = classes.create_subkey("magnet").map_err(err)?;
    magnet.set_value("", &"URL:Magnet link").map_err(err)?;
    magnet.set_value("URL Protocol", &"").map_err(err)?;
    magnet.create_subkey("DefaultIcon").map_err(err)?.0.set_value("", &icon).map_err(err)?;
    magnet.create_subkey(r"shell\open\command").map_err(err)?.0.set_value("", &command).map_err(err)?;

    // Shows Tormin in Settings > Apps > Default apps
    let (caps, _) = hkcu.create_subkey(CAPS).map_err(err)?;
    caps.set_value("ApplicationName", &"Tormin").map_err(err)?;
    caps.set_value("ApplicationDescription", &"Minimalist BitTorrent client").map_err(err)?;
    caps.create_subkey("FileAssociations").map_err(err)?.0.set_value(".torrent", &PROG_ID).map_err(err)?;
    caps.create_subkey("URLAssociations").map_err(err)?.0.set_value("magnet", &"magnet").map_err(err)?;
    hkcu.create_subkey(r"Software\RegisteredApplications").map_err(err)?.0.set_value("Tormin", &CAPS).map_err(err)?;

    // Windows only lets the user confirm the default app, so open that page
    let _ = Command::new("cmd")
        .args(["/C", "start", "", "ms-settings:defaultapps?registeredAppUser=Tormin"])
        .spawn();
    Ok(())
}

#[cfg(not(windows))]
pub fn associate_files() -> Result<(), String> {
    use std::fs;

    let exe = std::env::var("APPIMAGE")
        .ok()
        .or_else(|| std::env::current_exe().ok().map(|p| p.to_string_lossy().into_owned()))
        .ok_or("cannot find the executable")?;
    let apps = dirs::data_dir().ok_or("no data dir")?.join("applications");
    fs::create_dir_all(&apps).map_err(|e| e.to_string())?;
    let desktop = format!(
        "[Desktop Entry]\nType=Application\nName=Tormin\nExec=\"{exe}\" %U\nIcon=tormin\nTerminal=false\n\
         Categories=Network;FileTransfer;P2P;\nMimeType=application/x-bittorrent;x-scheme-handler/magnet;\n\
         StartupWMClass=tormin\n"
    );
    fs::write(apps.join("tormin.desktop"), desktop).map_err(|e| e.to_string())?;
    for mime in ["application/x-bittorrent", "x-scheme-handler/magnet"] {
        let _ = Command::new("xdg-mime").args(["default", "tormin.desktop", mime]).status();
    }
    let _ = Command::new("update-desktop-database").arg(&apps).status();
    Ok(())
}
