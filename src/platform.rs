use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(windows)]
use windows::core::PCWSTR;
#[cfg(windows)]
use windows::Win32::Storage::FileSystem::{
    GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDrives, GetVolumeInformationW,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DriveKind {
    Fixed,
    Removable,
    Network,
    Other,
}

#[derive(Clone, Debug)]
pub struct DriveInfo {
    pub root: String,
    pub letter: char,
    pub label: String,
    pub filesystem: String,
    pub total: u64,
    pub free: u64,
    pub kind: DriveKind,
}

impl DriveInfo {
    pub fn display_name(&self) -> String {
        if self.label.is_empty() {
            format!("Local Disk ({}:)", self.letter)
        } else {
            format!("{} ({}:)", self.label, self.letter)
        }
    }

    pub fn used(&self) -> u64 {
        self.total.saturating_sub(self.free)
    }

    pub fn used_fraction(&self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            (self.used() as f64 / self.total as f64) as f32
        }
    }

    pub fn kind_label(&self) -> &'static str {
        match self.kind {
            DriveKind::Fixed => "Local disk",
            DriveKind::Removable => "Removable",
            DriveKind::Network => "Network",
            DriveKind::Other => "Drive",
        }
    }
}

pub fn list_drives() -> Vec<DriveInfo> {
    #[cfg(windows)]
    {
        let mut drives = Vec::new();
        let mask = unsafe { GetLogicalDrives() };
        for index in 0..26u32 {
            if mask & (1 << index) == 0 {
                continue;
            }
            let letter = (b'A' + index as u8) as char;
            let root = format!("{letter}:\\");
            if let Some(drive) = probe_drive(&root, letter) {
                drives.push(drive);
            }
        }
        drives.sort_by_key(|drive| drive.letter);
        if drives.is_empty() {
            if let Some(drive) = probe_drive("C:\\", 'C') {
                drives.push(drive);
            }
        }
        drives
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

#[cfg(windows)]
fn probe_drive(root: &str, letter: char) -> Option<DriveInfo> {
    let wide = to_wide(root);
    let drive_type = unsafe { GetDriveTypeW(PCWSTR(wide.as_ptr())) };
    // DRIVE_REMOVABLE=2, DRIVE_FIXED=3, DRIVE_REMOTE=4, DRIVE_CDROM=5, DRIVE_RAMDISK=6
    if drive_type == 1 || drive_type == 5 {
        return None;
    }
    let kind = match drive_type {
        2 => DriveKind::Removable,
        3 => DriveKind::Fixed,
        4 => DriveKind::Network,
        _ => DriveKind::Other,
    };

    let mut free_available = 0u64;
    let mut total = 0u64;
    let mut total_free = 0u64;
    let ok = unsafe {
        GetDiskFreeSpaceExW(
            PCWSTR(wide.as_ptr()),
            Some(&mut free_available),
            Some(&mut total),
            Some(&mut total_free),
        )
    };
    if ok.is_err() || total == 0 {
        return None;
    }

    let mut label_buf = [0u16; 256];
    let mut fs_buf = [0u16; 64];
    let mut serial = 0u32;
    let mut max_component = 0u32;
    let mut flags = 0u32;
    let got_info = unsafe {
        GetVolumeInformationW(
            PCWSTR(wide.as_ptr()),
            Some(&mut label_buf),
            Some(&mut serial),
            Some(&mut max_component),
            Some(&mut flags),
            Some(&mut fs_buf),
        )
    }
    .is_ok();

    let label = if got_info {
        from_wide_buffer(&label_buf)
    } else {
        String::new()
    };
    let filesystem = if got_info {
        from_wide_buffer(&fs_buf)
    } else {
        String::new()
    };

    Some(DriveInfo {
        root: root.to_string(),
        letter,
        label,
        filesystem,
        total,
        free: free_available,
        kind,
    })
}

pub fn label_for_path(path: &Path) -> String {
    if path.parent().is_none() {
        // Drive root such as C:\
        let root = path.to_string_lossy();
        if let Some(letter) = root.chars().next() {
            if letter.is_ascii_alphabetic() {
                if let Some(drive) = list_drives()
                    .into_iter()
                    .find(|drive| drive.letter.eq_ignore_ascii_case(&letter))
                {
                    return drive.display_name();
                }
            }
        }
        return root.into_owned();
    }
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

pub fn is_drive_root(path: &Path) -> bool {
    let raw = path.to_string_lossy();
    let trimmed = raw.trim_end_matches(['\\', '/']);
    trimmed.len() == 2 && trimmed.as_bytes()[1] == b':'
}

pub fn system_drive() -> PathBuf {
    std::env::var_os("SystemDrive")
        .map(|drive| PathBuf::from(format!("{}\\", drive.to_string_lossy())))
        .unwrap_or_else(|| PathBuf::from("C:\\"))
}

pub fn reveal(path: &Path, is_dir: bool) {
    if !path.exists() {
        return;
    }
    let mut command = Command::new("explorer.exe");
    if is_dir {
        command.arg(path);
    } else {
        command.arg(format!("/select,\"{}\"", path.display()));
    }
    let _ = command.spawn();
}

pub fn open_path(path: &Path) {
    if path.is_dir() {
        let _ = Command::new("explorer.exe").arg(path).spawn();
    } else {
        let _ = Command::new("cmd.exe")
            .args(["/C", "start", ""])
            .arg(path)
            .spawn();
    }
}

pub fn delete_to_recycle_bin(path: &Path) -> Result<(), String> {
    if path.as_os_str().is_empty() {
        return Err("The item no longer exists".to_string());
    }
    trash::delete(path).map_err(|err| err.to_string())
}

/// Erases a file or folder from disk; nothing can be restored afterwards.
pub fn delete_permanently(path: &Path) -> Result<(), String> {
    if path.as_os_str().is_empty() {
        return Err("The item no longer exists".to_string());
    }
    let metadata = std::fs::symlink_metadata(path).map_err(|err| err.to_string())?;
    if metadata.is_dir() {
        return std::fs::remove_dir_all(path).map_err(|err| err.to_string());
    }
    // A read-only file cannot be removed until its attribute is cleared.
    let mut permissions = metadata.permissions();
    if permissions.readonly() {
        #[allow(clippy::permissions_set_readonly_false)]
        permissions.set_readonly(false);
        let _ = std::fs::set_permissions(path, permissions);
    }
    std::fs::remove_file(path).map_err(|err| err.to_string())
}

pub fn delete_item(path: &Path, permanent: bool) -> Result<(), String> {
    if permanent {
        delete_permanently(path)
    } else {
        delete_to_recycle_bin(path)
    }
}

/// Re-launches this executable with administrator rights, optionally rescanning.
/// True when the process already holds administrator rights.
pub fn is_elevated() -> bool {
    #[cfg(windows)]
    {
        use windows::Win32::UI::Shell::IsUserAnAdmin;
        unsafe { IsUserAnAdmin().as_bool() }
    }
    #[cfg(not(windows))]
    {
        false
    }
}

pub fn relaunch_as_admin(scan: Option<PathBuf>) -> Result<(), String> {
    #[cfg(windows)]
    {
        use windows::core::{HSTRING, PCWSTR};
        use windows::Win32::UI::Shell::ShellExecuteW;
        use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

        let exe = std::env::current_exe().map_err(|err| err.to_string())?;
        let arguments = match &scan {
            Some(path) => format!("--scan \"{}\"", path.display()),
            None => String::new(),
        };
        let result = unsafe {
            ShellExecuteW(
                None,
                &HSTRING::from("runas"),
                &HSTRING::from(exe.to_string_lossy().into_owned()),
                &HSTRING::from(arguments),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            )
        };
        if result.0 as isize <= 32 {
            Err("Windows refused the elevation request".to_string())
        } else {
            Ok(())
        }
    }
    #[cfg(not(windows))]
    {
        let _ = scan;
        Err("Only supported on Windows".to_string())
    }
}

const SHELL_KEY: &str = r"HKCU\Software\Classes\Directory\shell\G-SpaceAnalyzer";
const SHELL_COMMAND_KEY: &str =
    r"HKCU\Software\Classes\Directory\shell\G-SpaceAnalyzer\command";

pub fn shell_integration_present() -> bool {
    run_reg(&["query", SHELL_KEY]).is_ok()
}

/// Adds an Explorer context-menu entry for folders (per-user, no admin needed).
pub fn install_shell_integration() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|err| err.to_string())?;
    let exe_text = exe.to_string_lossy().into_owned();
    let command = format!("\"{exe_text}\" \"%1\"");
    run_reg(&[ "add", SHELL_KEY, "/ve", "/d", "Analyze with G-SpaceAnalyzer", "/f"])?;
    run_reg(&["add", SHELL_KEY, "/v", "Icon", "/d", &exe_text, "/f"])?;
    run_reg(&["add", SHELL_COMMAND_KEY, "/ve", "/d", &command, "/f"])?;
    Ok(())
}

pub fn remove_shell_integration() -> Result<(), String> {
    run_reg(&["delete", SHELL_KEY, "/f"])
}

#[cfg(windows)]
fn run_reg(args: &[&str]) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let output = Command::new("reg.exe")
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|err| err.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(if message.is_empty() {
            "reg.exe failed".to_string()
        } else {
            message
        })
    }
}

#[cfg(not(windows))]
fn run_reg(_args: &[&str]) -> Result<(), String> {
    Err("Only supported on Windows".to_string())
}

pub fn free_space_for(path: &Path) -> Option<u64> {
    let root = path.ancestors().last()?;
    let key = root.to_string_lossy().to_string();
    list_drives()
        .into_iter()
        .find(|drive| drive.root.eq_ignore_ascii_case(&key))
        .map(|drive| drive.free)
}

#[cfg(windows)]
fn to_wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(windows)]
fn from_wide_buffer(buffer: &[u16]) -> String {
    let end = buffer.iter().position(|value| *value == 0).unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permanent_delete_erases_files_and_folders() {
        let root = std::env::temp_dir().join("spaceanalyzer-permanent-delete-test");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("nested")).unwrap();
        let file = root.join("nested").join("data.bin");
        std::fs::write(&file, b"hello").unwrap();
        let mut permissions = std::fs::metadata(&file).unwrap().permissions();
        permissions.set_readonly(true);
        std::fs::set_permissions(&file, permissions).unwrap();

        delete_permanently(&root).unwrap();
        assert!(!root.exists());

        assert!(delete_permanently(&root.join("gone.bin")).is_err());
    }
}
