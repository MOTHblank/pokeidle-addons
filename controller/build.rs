mod icon;

fn main() {
    println!("cargo:rerun-if-changed=icon.rs");

    if std::env::var("CARGO_CFG_TARGET_OS").ok().as_deref() != Some("windows") {
        return;
    }

    let out_dir = std::path::PathBuf::from(
        std::env::var_os("OUT_DIR").expect("OUT_DIR is required for Windows resource generation"),
    );
    let icon_path = out_dir.join("moth-controller.ico");
    icon::write_ico(&icon_path, icon::DEFAULT_ICON_SIZE)
        .expect("could not generate Moth Controller icon");

    let mut resources = winresource::WindowsResource::new();
    resources.set_icon(
        icon_path
            .to_str()
            .expect("generated Moth Controller icon path is not valid UTF-8"),
    );
    resources
        .compile()
        .expect("could not embed Moth Controller icon into Windows executable");
}
