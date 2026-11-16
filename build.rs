fn main() {
    println!("cargo:rerun-if-changed=assets/icons/icon.ico");
    println!("cargo:rerun-if-changed=build.rs");

    #[cfg(windows)]
    {
        // The Cargo version is the single source for what Explorer reports.
        let version = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".to_string());

        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icons/icon.ico");
        res.set("ProductName", "G-SpaceAnalyzer");
        res.set(
            "FileDescription",
            "G-SpaceAnalyzer - free and open source disk space analyzer",
        );
        res.set("CompanyName", "G-SpaceAnalyzer");
        res.set("LegalCopyright", "MIT License");
        res.set("OriginalFilename", "G-SpaceAnalyzer.exe");
        res.set("FileVersion", &four_part(&version));
        res.set("ProductVersion", &four_part(&version));
        res.set_version_info(
            winresource::VersionInfo::FILEVERSION,
            packed_version(&version),
        );
        res.set_version_info(
            winresource::VersionInfo::PRODUCTVERSION,
            packed_version(&version),
        );
        if let Err(err) = res.compile() {
            println!("cargo:warning=Unable to embed the Windows version resource: {err}");
        }
    }
}

/// Splits "1.1.0" into the four numeric fields Windows expects.
#[cfg(windows)]
fn version_fields(version: &str) -> [u16; 4] {
    let mut fields = [0u16; 4];
    for (slot, part) in fields.iter_mut().zip(version.split('.')) {
        // Pre-release suffixes ("1.2.0-beta") simply fall back to zero.
        let digits: String = part.chars().take_while(|c| c.is_ascii_digit()).collect();
        *slot = digits.parse().unwrap_or(0);
    }
    fields
}

/// "1.1.0" -> "1.1.0.0", the string form shown in the file properties dialog.
#[cfg(windows)]
fn four_part(version: &str) -> String {
    let fields = version_fields(version);
    format!(
        "{}.{}.{}.{}",
        fields[0], fields[1], fields[2], fields[3]
    )
}

/// "1.1.0" -> 0x0001_0001_0000_0000, the packed form stored in the VERSIONINFO.
#[cfg(windows)]
fn packed_version(version: &str) -> u64 {
    let [major, minor, patch, build] = version_fields(version);
    (u64::from(major) << 48)
        | (u64::from(minor) << 32)
        | (u64::from(patch) << 16)
        | u64::from(build)
}
