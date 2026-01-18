use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

// Build script that injects a firmware version string into the binary and
// writes a copy to the repo's tmp/ directory for host-side tooling.
//
// Exports:
//   - LOADLYNX_FW_VERSION: "<crate> <semver> (profile <profile>, git <describe|unknown>)"
// Writes:
//   - tmp/<crate>-fw-version.txt (relative to repo root)

fn main() {
    // Ensure essential linker args are present even when building from the repo root
    // (so `firmware/analog/.cargo/config.toml` is not picked up).
    //
    // When building from `firmware/analog/`, these args already come from `.cargo/config.toml`;
    // avoid emitting duplicates (they can cause duplicate `memory.x` definitions at link time).
    let rustflags = std::env::var("CARGO_ENCODED_RUSTFLAGS").unwrap_or_default();
    if !rustflags.contains("link.x") {
        println!("cargo:rustc-link-arg=-Tlink.x");
    }
    if !rustflags.contains("defmt.x") {
        println!("cargo:rustc-link-arg=-Tdefmt.x");
    }
    if !rustflags.contains("--nmagic") {
        println!("cargo:rustc-link-arg=--nmagic");
    }

    // Re-run when local sources or git HEAD change.
    println!("cargo:rerun-if-changed=src/");
    for path in git_watch_paths() {
        println!("cargo:rerun-if-changed={}", path.display());
    }

    let pkg_name = env::var("CARGO_PKG_NAME").unwrap_or_else(|_| "unknown".to_string());
    let pkg_ver = env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".to_string());
    let profile = env::var("PROFILE").unwrap_or_else(|_| "unknown".to_string());
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
        let _ = fs::create_dir_all(&tmp_dir);
        let file_name = format!("{name}-fw-version.txt", name = pkg_name);
        let path = tmp_dir.join(file_name);
        let _ = fs::write(path, &version_string);
    }
}

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
