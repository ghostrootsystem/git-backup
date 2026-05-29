use chrono::Local;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let home = std::env::var("HOME")?;
    let user = std::env::var("USER")?;
    let date = Local::now().format("%Y-%m-%d %H:%M").to_string();
    let repo_url = "https://github.com/ghostrootsystem/voidlinux.git";

    let root = std::env::temp_dir().join("void_sync");
    if root.exists() {
        fs::remove_dir_all(&root)?;
    }

    let etc_base = root.join("etc");
    let home_root = root.join("home");
    let home_base = home_root.join(&user);
    let config_dest = home_base.join(".config");

    fs::create_dir_all(&etc_base)?;
    fs::File::create(etc_base.join(".keep"))?; 

    fs::create_dir_all(&config_dest)?;
    fs::File::create(home_root.join(".keep"))?; 
    fs::File::create(home_base.join(".keep"))?; 
    fs::File::create(config_dest.join(".keep"))?; 

    let grub_src = Path::new("/etc/default/grub");
    if grub_src.exists() {
        let etc_default_dest = etc_base.join("default");
        fs::create_dir_all(&etc_default_dest)?;
        Command::new("sudo")
            .args(["cp", "-p", grub_src.to_str().unwrap(), etc_default_dest.to_str().unwrap()])
            .status()?;
    }

    let config_whitelist = vec!["ironbar", "kitty", "driftwm", "helix"];
    for folder in config_whitelist {
        let src = Path::new(&home).join(".config").join(folder);
        if src.exists() {
            Command::new("cp")
                .args(["-rp", src.to_str().unwrap(), config_dest.to_str().unwrap()])
                .status()?;
        }
    }

    let qbit_themes = Path::new(&home).join(".config/qBittorrent/themes");
    if qbit_themes.exists() {
        let target_parent = config_dest.join("qBittorrent");
        fs::create_dir_all(&target_parent)?;
        Command::new("cp")
            .args(["-rp", qbit_themes.to_str().unwrap(), target_parent.to_str().unwrap()])
            .status()?;
        fs::File::create(target_parent.join(".keep"))?;
    }

    let extra_items = vec![".zshrc", ".zprofile"];
    for item in extra_items {
        let src = Path::new(&home).join(item);
        if src.exists() {
            if let Some(parent) = Path::new(item).parent() {
                if parent.to_str() != Some("") {
                    fs::create_dir_all(home_base.join(parent))?;
                }
            }

            Command::new("cp")
                .args(["-rp", src.to_str().unwrap(), home_base.join(item).to_str().unwrap()])
                .status()?;

            let dest_path = home_base.join(item);
            if dest_path.is_dir() {
                let git_in_backup = dest_path.join(".git");
                if git_in_backup.exists() {
                    fs::remove_dir_all(git_in_backup)?;
                }
                fs::File::create(dest_path.join(".keep"))?;
            }
        }
    }

    let xbps_output = Command::new("xbps-query").arg("-m").output()?;
    if xbps_output.status.success() {
        let packages_raw = String::from_utf8_lossy(&xbps_output.stdout);
        let packages_list: Vec<&str> = packages_raw
            .lines()
            .filter_map(|line| line.rsplit_once('-').map(|(name, _)| name))
            .collect();
        let packages_string = packages_list.join(" ");
        let script_content = format!(
            "#!/bin/bash\nsudo xbps-install -Syu\nsudo xbps-install -S {}\n",
            packages_string
        );
        let packages_script_path = home_base.join("packages-install");
        fs::write(&packages_script_path, script_content)?;
        
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&packages_script_path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&packages_script_path, perms)?;
        }
    }

    Command::new("sudo")
        .args(["chown", "-R", &format!("{}:users", user), root.to_str().unwrap()])
        .status()?;

    std::env::set_current_dir(&root)?;

    let git = |args: &[&str]| { Command::new("git").args(args).status() };
    git(&["init"])?;
    git(&["remote", "add", "origin", repo_url])?;
    git(&["add", "."])?;
    git(&["commit", "-m", &format!("Void Linux Backup: {}", date)])?;
    git(&["branch", "-M", "main"])?;
    git(&["push", "-u", "origin", "main", "--force"])?;

    fs::remove_dir_all(&root)?;

    Ok(())
}
