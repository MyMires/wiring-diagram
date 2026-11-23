fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=../library");
    println!("cargo:rerun-if-changed=assets");
    println!("cargo:rerun-if-changed=../wiring_version.py");
    let version = env!("CARGO_PKG_VERSION");
    let python_version = std::fs::read_to_string("../wiring_version.py")?;
    // A release tag identifies both the Python CLI and the native executable.
    assert_eq!(
        python_version.trim(),
        format!("__version__ = \"{version}\""),
        "wiring_version.py must match the Cargo package version"
    );
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_language(0x0804)
            .set("FileDescription", "测试治具接线图")
            .set("ProductName", "测试治具接线图")
            .set("FileVersion", version)
            .set("ProductVersion", version)
            .set("OriginalFilename", "wiring.exe")
            .compile()?;
    }
    Ok(())
}
