//! Build script: on Windows, embed the application icon and version details
//! in the executable so that Explorer and the taskbar show them.

fn main() {
    println!("cargo:rerun-if-changed=assets/stamp.ico");
    #[cfg(windows)]
    {
        // Only when building for Windows on Windows; a missing resource
        // compiler should not stop the build, the icon is cosmetic.
        if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
            let mut resource = winresource::WindowsResource::new();
            resource.set_icon("assets/stamp.ico");
            resource.set("ProductName", "STAMP");
            resource.set(
                "FileDescription",
                "STAMP: statistical analysis of taxonomic and functional profiles",
            );
            if let Err(error) = resource.compile() {
                println!("cargo:warning=could not embed the Windows icon: {error}");
            }
        }
    }
}
