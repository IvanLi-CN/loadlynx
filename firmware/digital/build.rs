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
    for path in git_watch_paths() {
        println!("cargo:rerun-if-changed={}", path.display());
    }

    let pkg_name = std::env::var("CARGO_PKG_NAME").unwrap_or_else(|_| "unknown".to_string());
    let pkg_ver = std::env::var("LOADLYNX_RELEASE_VERSION")
        .or_else(|_| std::env::var("LOADLYNX_PROJECT_VERSION"))
        .or_else(|_| std::env::var("CARGO_PKG_VERSION"))
        .unwrap_or_else(|_| "0.0.0".to_string());
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "unknown".to_string());
    let git_info = std::env::var("LOADLYNX_RELEASE_TAG")
        .unwrap_or_else(|_| git_describe().unwrap_or_else(|| "git unknown".to_string()));
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
    println!("cargo:rustc-env=LOADLYNX_PACKAGE_VERSION={}", pkg_ver);
    println!("cargo:rustc-env=LOADLYNX_FW_PROFILE={}", profile);
    println!("cargo:rustc-env=LOADLYNX_FW_SRC_DIGEST={}", src_hash);
    if let Ok(target) = std::env::var("TARGET") {
        println!("cargo:rustc-env=LOADLYNX_FW_TARGET={}", target);
    }

    // Also emit a copy into repo_root/tmp for host-side scripts.
    if let Some(repo_root) = repo_root_from_manifest() {
        let tmp_dir = repo_root.join("tmp");
        let _ = std::fs::create_dir_all(&tmp_dir);
        let file_name = format!("{name}-fw-version.txt", name = pkg_name);
        let path = tmp_dir.join(file_name);
        let _ = std::fs::write(path, &version_string);
    }

    // Development firmware must not carry default Wi-Fi credentials. Runtime
    // Wi-Fi is written through USB/devd or Web Serial and persisted in EEPROM.
    // Factory Wi-Fi is an explicit, controlled build mode only.
    println!("cargo:rerun-if-env-changed=LOADLYNX_ENABLE_FACTORY_WIFI");
    if env::var("LOADLYNX_ENABLE_FACTORY_WIFI").as_deref() == Ok("1") {
        inject_factory_wifi_from_environment();
    }
}

fn linker_error_hints() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 2 {
        let kind = &args[1];
        let what = &args[2];
        if kind.as_str() == "undefined-symbol" {
            match what.as_str() {
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
            }
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
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root_from_manifest() -> Option<PathBuf> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").ok()?);
    manifest_dir.parent()?.parent().map(|p| p.to_path_buf())
}

fn git_watch_paths() -> Vec<PathBuf> {
    let Some(repo_root) = repo_root_from_manifest() else {
        return Vec::new();
    };
    let Some((git_dir, common_dir)) = git_dirs(&repo_root) else {
        return Vec::new();
    };

    let mut paths = Vec::new();

    let head = git_dir.join("HEAD");
    if head.exists() {
        paths.push(head.clone());
    } else {
        return paths;
    }

    // When refs are packed, commits update packed-refs rather than a loose ref file.
    let packed_refs = common_dir.join("packed-refs");
    if packed_refs.exists() {
        paths.push(packed_refs);
    }

    // Track the active branch ref file so commits rerun the build script and
    // the embedded version string stays accurate.
    if let Ok(contents) = fs::read_to_string(&head)
        && let Some(line) = contents.lines().next()
    {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("ref:") {
            let ref_name = rest.trim();
            if !ref_name.is_empty() {
                let ref_path = common_dir.join(ref_name);
                if ref_path.exists() {
                    paths.push(ref_path);
                }
            }
        }
    }

    paths
}

fn git_dirs(repo_root: &Path) -> Option<(PathBuf, PathBuf)> {
    let dot_git = repo_root.join(".git");
    let git_dir = if dot_git.is_dir() {
        dot_git
    } else if dot_git.is_file() {
        let contents = fs::read_to_string(&dot_git).ok()?;
        let line = contents.lines().next()?.trim();
        let raw = line.strip_prefix("gitdir:")?.trim();
        resolve_maybe_relative(repo_root, raw)
    } else {
        return None;
    };

    let common_dir = {
        let commondir = git_dir.join("commondir");
        if commondir.exists() {
            let contents = fs::read_to_string(commondir).ok()?;
            let raw = contents.lines().next()?.trim();
            resolve_maybe_relative(&git_dir, raw)
        } else {
            git_dir.clone()
        }
    };

    Some((git_dir, common_dir))
}

fn resolve_maybe_relative(base: &Path, raw: &str) -> PathBuf {
    let path = PathBuf::from(raw);
    if path.is_absolute() {
        path
    } else {
        base.join(path)
    }
}

fn git_describe() -> Option<String> {
    let repo_root = repo_root_from_manifest()?;
    let output = Command::new("git")
        .arg("-C")
        .arg(&repo_root)
        .args([
            "describe", "--tags", "--match", "v[0-9]*", "--dirty", "--always",
        ])
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

    fn walk_dir(dir: &Path, state: &mut u64) -> std::io::Result<()> {
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

fn get_non_empty_env(key: &str) -> Option<String> {
    if let Ok(v) = env::var(key) {
        let v = v.trim();
        if !v.is_empty() {
            return Some(v.to_string());
        }
    }

    None
}

fn inject_factory_wifi_from_environment() {
    for key in [
        "LOADLYNX_FACTORY_WIFI_SSID",
        "LOADLYNX_FACTORY_WIFI_PSK",
        "LOADLYNX_FACTORY_WIFI_HOSTNAME",
        "LOADLYNX_FACTORY_WIFI_STATIC_IP",
        "LOADLYNX_FACTORY_WIFI_NETMASK",
        "LOADLYNX_FACTORY_WIFI_GATEWAY",
        "LOADLYNX_FACTORY_WIFI_DNS",
    ] {
        println!("cargo:rerun-if-env-changed={key}");
    }

    let Some(wifi_ssid) = get_non_empty_env("LOADLYNX_FACTORY_WIFI_SSID") else {
        panic!("LOADLYNX_ENABLE_FACTORY_WIFI=1 requires LOADLYNX_FACTORY_WIFI_SSID");
    };
    let Some(wifi_psk) = get_non_empty_env("LOADLYNX_FACTORY_WIFI_PSK") else {
        panic!("LOADLYNX_ENABLE_FACTORY_WIFI=1 requires LOADLYNX_FACTORY_WIFI_PSK");
    };

    println!("cargo:rustc-env=LOADLYNX_WIFI_SSID={wifi_ssid}");
    println!("cargo:rustc-env=LOADLYNX_WIFI_PSK={wifi_psk}");
    if let Some(hostname) = get_non_empty_env("LOADLYNX_FACTORY_WIFI_HOSTNAME") {
        println!("cargo:rustc-env=LOADLYNX_WIFI_HOSTNAME={hostname}");
    }
    if let Some(static_ip) = get_non_empty_env("LOADLYNX_FACTORY_WIFI_STATIC_IP") {
        println!("cargo:rustc-env=LOADLYNX_WIFI_STATIC_IP={static_ip}");
    }
    if let Some(netmask) = get_non_empty_env("LOADLYNX_FACTORY_WIFI_NETMASK") {
        println!("cargo:rustc-env=LOADLYNX_WIFI_NETMASK={netmask}");
    }
    if let Some(gateway) = get_non_empty_env("LOADLYNX_FACTORY_WIFI_GATEWAY") {
        println!("cargo:rustc-env=LOADLYNX_WIFI_GATEWAY={gateway}");
    }
    if let Some(dns) = get_non_empty_env("LOADLYNX_FACTORY_WIFI_DNS") {
        println!("cargo:rustc-env=LOADLYNX_WIFI_DNS={dns}");
    }
}
