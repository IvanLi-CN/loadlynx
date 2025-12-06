fn main() {
    // Preserve original linker configuration for defmt and STM32-style error hints.
    linker_error_hints();
    println!("cargo:rustc-link-arg=-Tdefmt.x");
    println!("cargo:rustc-link-arg=-Tlinkall.x");

    // Build script that injects a firmware version string into the binary and
    // writes a copy to the repo's tmp/ directory for host-side tooling.
    //
    // Exports:
    //   - LOADLYNX_FW_VERSION: "<crate> <semver> (profile <profile>, git <describe|unknown>)"
    // Writes:
    //   - tmp/<crate>-fw-version.txt (relative to repo root)

    // Re-run when local sources or git HEAD change.
    println!("cargo:rerun-if-changed=src/");
    if let Some(head) = git_head_path() {
        println!("cargo:rerun-if-changed={}", head.display());
    }

    // Re-run when Wi-Fi config changes.
    if let Some(repo_root) = repo_root_from_manifest() {
        let env_path = repo_root.join(".env");
        if env_path.exists() {
            println!("cargo:rerun-if-changed={}", env_path.display());
        }
    }

    let pkg_name = std::env::var("CARGO_PKG_NAME").unwrap_or_else(|_| "unknown".to_string());
    let pkg_ver = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".to_string());
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "unknown".to_string());
    let git_info = git_describe().unwrap_or_else(|| "git unknown".to_string());
    let src_hash = source_digest()
        .map(|h| format!("src 0x{h:016x}"))
        .unwrap_or_else(|| "src unknown".to_string());

    let version_string = format!(
        "{name} {ver} (profile {profile}, {git}, {src})",
        name = pkg_name,
        ver = pkg_ver,
        git = git_info,
        src = src_hash,
    );

    // Make the version string available to firmware code.
    println!("cargo:rustc-env=LOADLYNX_FW_VERSION={}", version_string);

    // Also emit a copy into repo_root/tmp for host-side scripts.
    if let Some(repo_root) = repo_root_from_manifest() {
        let tmp_dir = repo_root.join("tmp");
        let _ = std::fs::create_dir_all(&tmp_dir);
        let file_name = format!("{name}-fw-version.txt", name = pkg_name);
        let path = tmp_dir.join(file_name);
        let _ = std::fs::write(path, &version_string);
    }

    // Load Wi-Fi configuration from .env or environment and inject it as compile-time env vars.
    let mut cfg = std::collections::HashMap::new();
    if let Some(repo_root) = repo_root_from_manifest() {
        let env_path = repo_root.join(".env");
        if env_path.exists() {
            cfg.extend(load_env_file(&env_path));
        }
    }

    let wifi_ssid = get_wifi_cfg("DIGITAL_WIFI_SSID", &cfg);
    let wifi_psk = get_wifi_cfg("DIGITAL_WIFI_PSK", &cfg);

    if wifi_ssid.is_none() || wifi_psk.is_none() {
        eprintln!(
            "error: Wi-Fi config missing. Set DIGITAL_WIFI_SSID and DIGITAL_WIFI_PSK in .env or environment."
        );
        std::process::exit(1);
    }

    let wifi_ssid = wifi_ssid.unwrap();
    let wifi_psk = wifi_psk.unwrap();

    println!("cargo:rustc-env=LOADLYNX_WIFI_SSID={}", wifi_ssid);
    println!("cargo:rustc-env=LOADLYNX_WIFI_PSK={}", wifi_psk);

    if let Some(hostname) = get_wifi_cfg("DIGITAL_WIFI_HOSTNAME", &cfg) {
        println!("cargo:rustc-env=LOADLYNX_WIFI_HOSTNAME={}", hostname);
    }
    if let Some(static_ip) = get_wifi_cfg("DIGITAL_WIFI_STATIC_IP", &cfg) {
        println!("cargo:rustc-env=LOADLYNX_WIFI_STATIC_IP={}", static_ip);
    }
    if let Some(netmask) = get_wifi_cfg("DIGITAL_WIFI_NETMASK", &cfg) {
        println!("cargo:rustc-env=LOADLYNX_WIFI_NETMASK={}", netmask);
    }
    if let Some(gateway) = get_wifi_cfg("DIGITAL_WIFI_GATEWAY", &cfg) {
        println!("cargo:rustc-env=LOADLYNX_WIFI_GATEWAY={}", gateway);
    }
    if let Some(dns) = get_wifi_cfg("DIGITAL_WIFI_DNS", &cfg) {
        println!("cargo:rustc-env=LOADLYNX_WIFI_DNS={}", dns);
    }
}

fn linker_error_hints() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 2 {
        let kind = &args[1];
        let what = &args[2];
        match kind.as_str() {
            "undefined-symbol" => match what.as_str() {
                "_defmt_timestamp" => {
                    eprintln!();
                    eprintln!(
                        "💡 `defmt` not found - make sure `defmt.x` is added as a linker script and defmt is linked."
                    );
                    eprintln!();
                }
                "_stack_start" => {
                    eprintln!();
                    eprintln!("💡 `linkall.x` missing - ensure it is passed as linker script.");
                    eprintln!();
                }
                _ => {}
            },
            _ => {}
        }
        std::process::exit(0);
    }
    println!(
        "cargo:rustc-link-arg=-Wl,--error-handling-script={}",
        std::env::current_exe().unwrap().display()
    );
}

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn repo_root_from_manifest() -> Option<PathBuf> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").ok()?);
    manifest_dir.parent()?.parent().map(|p| p.to_path_buf())
}

fn git_head_path() -> Option<PathBuf> {
    let repo_root = repo_root_from_manifest()?;
    let head = repo_root.join(".git/HEAD");
    if head.exists() { Some(head) } else { None }
}

fn git_describe() -> Option<String> {
    let repo_root = repo_root_from_manifest()?;
    let output = Command::new("git")
        .arg("-C")
        .arg(&repo_root)
        .args(["describe", "--tags", "--dirty", "--always"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

fn source_digest() -> Option<u64> {
    use std::ffi::OsStr;

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").ok()?);
    let src_dir = manifest_dir.join("src");
    if !src_dir.is_dir() {
        return None;
    }

    let mut hash: u64 = 0xcbf29ce484222325; // FNV-1a offset basis

    fn hash_bytes(state: &mut u64, bytes: &[u8]) {
        for &b in bytes {
            *state ^= u64::from(b);
            *state = state.wrapping_mul(0x100000001b3);
        }
    }

    fn walk_dir(dir: &PathBuf, state: &mut u64) -> std::io::Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                walk_dir(&path, state)?;
            } else if path.extension() == Some(OsStr::new("rs")) {
                hash_bytes(state, path.to_string_lossy().as_bytes());
                let data = fs::read(&path)?;
                hash_bytes(state, &data);
            }
        }
        Ok(())
    }

    if walk_dir(&src_dir, &mut hash).is_ok() {
        Some(hash)
    } else {
        None
    }
}

fn load_env_file(path: &std::path::Path) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();

    let contents = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => return map,
    };

    for line in contents.lines() {
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();

            if !key.is_empty() && !value.is_empty() {
                map.insert(key.to_string(), value.to_string());
            }
        }
    }

    map
}

fn get_wifi_cfg(
    key: &str,
    file_cfg: &std::collections::HashMap<String, String>,
) -> Option<String> {
    if let Ok(v) = env::var(key) {
        let v = v.trim();
        if !v.is_empty() {
            return Some(v.to_string());
        }
    }

    if let Some(v) = file_cfg.get(key) {
        let v = v.trim();
        if !v.is_empty() {
            return Some(v.to_string());
        }
    }

    None
}
