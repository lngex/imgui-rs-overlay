fn main() {
    cc::Build::new()
        .cpp(true)
        .file("imgui/imgui_impl_win32.cpp")
        .file("imgui/imgui_impl_dx11.cpp")
        .compile("lingex_imgui_impl");
    println!("cargo:rustc-link-lib=static=lingex_imgui_impl")
}