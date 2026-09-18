use std::path::PathBuf;
use std::process::Command;

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        build_widget_bridge();
    }
    tauri_build::build()
}

/// Compiles `macos/widget_bridge.swift` into a static library and links it,
/// so the core can ask WidgetKit to reload the desktop widget.
///
/// Needs `swiftc`, which ships with Xcode and with the Command Line Tools.
fn build_widget_bridge() {
    let source = "macos/widget_bridge.swift";
    println!("cargo:rerun-if-changed={source}");
    println!("cargo:rerun-if-env-changed=MACOSX_DEPLOYMENT_TARGET");

    let out = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let arch = match std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
        Ok("aarch64") => "arm64",
        Ok("x86_64") => "x86_64",
        other => panic!("no widget bridge for architecture {other:?}"),
    };
    // Matches bundle.macOS.minimumSystemVersion unless the build overrides it.
    let minimum = std::env::var("MACOSX_DEPLOYMENT_TARGET").unwrap_or_else(|_| "10.15".into());

    let status = Command::new("xcrun")
        .args([
            "swiftc",
            "-parse-as-library",
            "-emit-library",
            "-static",
            "-O",
        ])
        .args(["-swift-version", "5", "-module-name", "NankivWidgetBridge"])
        .args(["-target", &format!("{arch}-apple-macosx{minimum}")])
        .arg("-o")
        .arg(out.join("libnankiv_widget_bridge.a"))
        .arg(source)
        .status()
        .expect("could not run swiftc; install Xcode or the Command Line Tools");
    assert!(status.success(), "compiling the widget bridge failed");

    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=nankiv_widget_bridge");
    // Weak, so the app still launches where WidgetKit does not exist.
    println!("cargo:rustc-link-arg=-Wl,-weak_framework,WidgetKit");

    // The Swift runtime is part of macOS; the compatibility shims for older
    // systems come from the toolchain.
    let toolchain_lib = Command::new("xcrun")
        .args(["--find", "swiftc"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|p| PathBuf::from(p.trim()))
        .and_then(|p| p.parent()?.parent().map(|d| d.join("lib/swift/macosx")));
    if let Some(dir) = toolchain_lib {
        println!("cargo:rustc-link-search=native={}", dir.display());
    }
    let sdk = Command::new("xcrun")
        .args(["--sdk", "macosx", "--show-sdk-path"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok());
    if let Some(sdk) = sdk {
        println!(
            "cargo:rustc-link-search=native={}/usr/lib/swift",
            sdk.trim()
        );
    }
    println!("cargo:rustc-link-search=native=/usr/lib/swift");
    println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
}
