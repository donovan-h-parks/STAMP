//! Build script: compress the larger built-in example dataset and, on
//! Windows, embed the application icon and version details in the executable
//! so that Explorer and the taskbar show them.

/// Write a gzip-compressed copy of an example file to the build directory,
/// for the program to carry inside it. The coal-seam profile is 4 MB of
/// text and about 80 kB compressed, which matters for the web page.
fn compress_example(source: &str, name: &str) {
    use std::io::Write;
    println!("cargo:rerun-if-changed={source}");
    let data = std::fs::read(source).unwrap_or_else(|e| panic!("cannot read {source}: {e}"));
    let out = std::path::Path::new(&std::env::var("OUT_DIR").expect("set by cargo")).join(name);
    let file = std::fs::File::create(&out).unwrap_or_else(|e| panic!("cannot write {name}: {e}"));
    let mut encoder = flate2::write::GzEncoder::new(file, flate2::Compression::best());
    encoder
        .write_all(&data)
        .and_then(|_| encoder.finish().map(|_| ()))
        .unwrap_or_else(|e| panic!("cannot compress {source}: {e}"));
}

fn main() {
    compress_example("../../examples/CoalSeamGas/coal.spf", "coal.spf.gz");
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
