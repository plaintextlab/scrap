fn main() {
    // Gives scrap.exe its icon in Explorer and the taskbar.
    #[cfg(windows)]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("packaging/icons/scrap.ico");
        res.set("ProductName", "Scrap");
        res.set("FileDescription", "Scrap");
        if let Err(e) = res.compile() {
            println!("cargo:warning=couldn't embed the app icon: {e}");
        }
    }
    println!("cargo:rerun-if-changed=packaging/icons/scrap.ico");
}
