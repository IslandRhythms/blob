fn main() {
    println!("cargo:rerun-if-changed=icon.png");

    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() != "windows" {
        return;
    }

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let ico_path = std::path::Path::new(&out_dir).join("icon.ico");
    let img = image::open("icon.png")
        .expect("icon.png")
        .resize_exact(256, 256, image::imageops::FilterType::Lanczos3);
    img.save(&ico_path).expect("write icon.ico");

    let mut res = winresource::WindowsResource::new();
    res.set_icon(ico_path.to_str().expect("utf-8 icon path"));
    res.compile().expect("embed Windows icon");
}
