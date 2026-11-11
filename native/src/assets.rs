//! Sprite pack nhúng sẵn trong exe (build.rs).

include!(concat!(env!("OUT_DIR"), "/assets.rs"));

/// File `path` tính từ `assets/` ("sprites/c-boggo/idle.webp"), không phân biệt hoa thường.
pub fn get(path: &str) -> Option<&'static [u8]> {
    let path = path.to_lowercase();
    FILES.binary_search_by(|(name, _)| (*name).cmp(path.as_str())).ok().map(|i| FILES[i].1)
}
