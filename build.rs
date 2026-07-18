fn main() {
    #[cfg(target_os = "windows")]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/programmer.ico");
        res.set("ProductName", "Programmer-Wander");
        res.set("FileDescription", "Programmer-Wander MCP server - AI dev shell");
        res.set("CompanyName", "AIWander");
        res.set("LegalCopyright", "Copyright 2026 Joseph Wander - Apache 2.0");
        if let Err(err) = res.compile() {
            // Icon embedding is best-effort: a missing rc toolchain must not
            // break the build, but the warning should surface in CI logs.
            println!("cargo:warning=icon resource not embedded: {err}");
        }
    }
    println!("cargo:rerun-if-changed=assets/programmer.ico");
}
