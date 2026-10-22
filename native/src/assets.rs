//! Sprite pack và effect của Long nhúng sẵn trong exe (build.rs).

include!(concat!(env!("OUT_DIR"), "/assets.rs"));

/// File `path` tính từ `assets/` ("sprites/c-boggo/idle.webp"), không phân biệt hoa thường.
pub fn get(path: &str) -> Option<&'static [u8]> {
    let path = path.to_lowercase();
    FILES.binary_search_by(|(name, _)| (*name).cmp(path.as_str())).ok().map(|i| FILES[i].1)
}

/// Mọi file nằm trong thư mục `dir` ("effects/long-aura"), theo tên.
pub fn list(dir: &str) -> impl Iterator<Item = (&'static str, &'static [u8])> {
    let prefix = format!("{}/", dir.to_lowercase());
    FILES.iter().filter(move |(name, _)| name.starts_with(&prefix)).copied()
}
