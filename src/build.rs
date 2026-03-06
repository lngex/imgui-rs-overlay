fn main() {
    cc::Build::new()
        .cpp(true)
        .flag_if_supported("-std=c++11")
        .file("imgui/imgui_impl_win32.cpp")
        .file("imgui/imgui_impl_dx11.cpp")
        .define("IMGUI_DISABLE_OBSOLETE_FUNCTIONS", None)
        .define("IMGUI_USE_WCHAR32", None)
        .define("CIMGUI_NO_EXPORT", None)
        .define("IMGUI_DISABLE_WIN32_FUNCTIONS", None)
        .define("IMGUI_DISABLE_OSX_FUNCTIONS", None)
        .compile("lingex_imgui_impl");
    println!("cargo:rustc-link-lib=static=lingex_imgui_impl")
}
