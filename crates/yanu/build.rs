fn main() {
    #[cfg(target_os = "windows")]
    embed_resource::compile("../../assets/manifest.rc", embed_resource::NONE);
}
