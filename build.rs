fn main() {
    #[cfg(target_os = "windows")]
    embed_resource::compile("resources/manifest.rc", embed_resource::NONE);
}
