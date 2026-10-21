//! Địa hình pet sống: mặt đất (mép trên taskbar) cộng mép trên và cạnh bên của các cửa sổ thật. Pet đứng trên mặt
//! đất hoặc đang bay thì nằm trước mọi cửa sổ; đứng hay leo trên cửa sổ nào thì bị các cửa sổ nằm trên cửa sổ đó che.

use super::{Rect, WindowInfo};
use std::collections::HashMap;

/// Vùng pet được ở, theo CSS pixel của overlay: tường trái, tường phải, trần, mặt đất.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    pub left: f64,
    pub right: f64,
    pub top: f64,
    pub floor: f64,
}

/// Đoạn [from, to] trên một trục.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Span {
    pub from: f64,
    pub to: f64,
}

/// Mép trên của một cửa sổ: pet đứng, đi trên đó.
#[derive(Debug, Clone, PartialEq)]
pub struct Ledge {
    /// Cửa sổ có mép này.
    pub id: isize,
    pub y: f64,
    /// Cả mép, đã cắt theo màn hình.
    pub from: f64,
    pub to: f64,
    /// Phần không bị cửa sổ nằm trên che, từ trái sang phải.
    pub open: Vec<Span>,
}

/// Cạnh bên của một cửa sổ: pet leo từ phía ngoài.
#[derive(Debug, Clone, PartialEq)]
pub struct Wall {
    pub id: isize,
    /// -1: cạnh trái, pet bám từ bên trái (mặt quay sang phải); 1: cạnh phải.
    pub side: i8,
    pub x: f64,
    /// Từ mép trên tới mép dưới cửa sổ, đã cắt theo màn hình.
    pub from: f64,
    pub to: f64,
    /// Phần không bị cửa sổ nằm trên che, từ trên xuống dưới.
    pub open: Vec<Span>,
}

#[derive(Debug, Clone)]
struct Entry {
    rect: Rect,
    /// Vị trí trong thứ tự chồng, 0 là cửa sổ trên cùng.
    z: usize,
    ledge: Option<Ledge>,
    walls: [Option<Wall>; 2],
}

#[derive(Debug, Clone)]
pub struct Terrain {
    pub bounds: Bounds,
    pub windows: Vec<WindowInfo>,
    pub ledges: Vec<Ledge>,
    /// Theo thứ tự trong `windows` (bỏ cửa sổ trùng id), như `Map` của bản TS.
    entries: Vec<(isize, Entry)>,
    index: HashMap<isize, usize>,
    /// Khung cửa sổ theo thứ tự chồng, trên cùng trước.
    stack: Vec<Rect>,
}

impl Terrain {
    /// `windows` xếp từ trên xuống dưới theo thứ tự chồng.
    pub fn new(bounds: Bounds, windows: Vec<WindowInfo>) -> Self {
        let stack: Vec<Rect> = windows.iter().map(|w| w.rect).collect();
        let mut ledges = Vec::new();
        let mut entries = Vec::new();
        let mut index = HashMap::new();
        for (z, w) in windows.iter().enumerate() {
            if index.contains_key(&w.id) {
                continue;
            }
            let above = &stack[..z];
            let ledge = ledge_of(w.id, w.rect, above, bounds);
            if let Some(ledge) = &ledge {
                ledges.push(ledge.clone());
            }
            let walls = [wall_of(w.id, w.rect, -1, above, bounds), wall_of(w.id, w.rect, 1, above, bounds)];
            index.insert(w.id, entries.len());
            entries.push((w.id, Entry { rect: w.rect, z, ledge, walls }));
        }
        Self { bounds, windows, ledges, entries, index, stack }
    }

    /// Không có cửa sổ nào.
    pub fn empty(bounds: Bounds) -> Self {
        Self::new(bounds, Vec::new())
    }

    fn entry(&self, id: isize) -> Option<&Entry> {
        self.index.get(&id).map(|&i| &self.entries[i].1)
    }

    pub fn window(&self, id: isize) -> Option<Rect> {
        self.entry(id).map(|e| e.rect)
    }

    pub fn ledge(&self, id: isize) -> Option<&Ledge> {
        self.entry(id)?.ledge.as_ref()
    }

    pub fn wall(&self, id: isize, side: i8) -> Option<&Wall> {
        self.entry(id)?.walls[if side < 0 { 0 } else { 1 }].as_ref()
    }

    /// Mọi cạnh leo được, theo thứ tự cửa sổ (trái rồi phải).
    pub fn walls(&self) -> Vec<&Wall> {
        self.entries.iter().flat_map(|(_, e)| e.walls.iter().flatten()).collect()
    }

    /// Các cửa sổ nằm trên cửa sổ `id` trong thứ tự chồng: che mất pet đứng hoặc leo trên `id`.
    pub fn above(&self, id: isize) -> &[Rect] {
        match self.entry(id) {
            Some(entry) => &self.stack[..entry.z],
            None => &[],
        }
    }

    /// Điểm (x, y) có bị cửa sổ nằm trên cửa sổ `id` che không.
    pub fn covered(&self, id: isize, x: f64, y: f64) -> bool {
        self.above(id).iter().any(|r| contains(r, x, y))
    }
}

/// Đoạn mở chứa `x`, hoặc `None` nếu chỗ đó bị che.
pub fn open_span_at(spans: &[Span], x: f64) -> Option<Span> {
    spans.iter().copied().find(|s| x >= s.from && x <= s.to)
}

pub fn contains(rect: &Rect, x: f64, y: f64) -> bool {
    x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height
}

/// Bỏ các đoạn `cuts` ra khỏi `spans` (kết quả xếp tăng dần).
pub fn subtract(spans: &[Span], cuts: &[Span]) -> Vec<Span> {
    let mut result = spans.to_vec();
    for cut in cuts {
        if cut.to <= cut.from {
            continue;
        }
        result = result
            .into_iter()
            .flat_map(|span| {
                if cut.to <= span.from || cut.from >= span.to {
                    return vec![span];
                }
                let mut parts = Vec::new();
                if cut.from > span.from {
                    parts.push(Span { from: span.from, to: cut.from });
                }
                if cut.to < span.to {
                    parts.push(Span { from: cut.to, to: span.to });
                }
                parts
            })
            .collect();
    }
    result.sort_by(|a, b| a.from.total_cmp(&b.from));
    result
}

/// Mép trên nằm trong màn hình, dưới trần và trên mặt đất, thì mới là chỗ đứng.
fn ledge_of(id: isize, rect: Rect, above: &[Rect], bounds: Bounds) -> Option<Ledge> {
    let y = rect.y;
    let from = rect.x.max(bounds.left);
    let to = (rect.x + rect.width).min(bounds.right);
    if to <= from || y <= bounds.top || y >= bounds.floor {
        return None;
    }
    // Hàng pixel ngay trên mép bị che thì chỗ đó không đứng được.
    let cuts: Vec<Span> = above
        .iter()
        .filter(|r| r.y <= y - 1.0 && r.y + r.height > y - 1.0)
        .map(|r| Span { from: r.x, to: r.x + r.width })
        .collect();
    Some(Ledge { id, y, from, to, open: subtract(&[Span { from, to }], &cuts) })
}

fn wall_of(id: isize, rect: Rect, side: i8, above: &[Rect], bounds: Bounds) -> Option<Wall> {
    let x = if side < 0 { rect.x } else { rect.x + rect.width };
    let from = rect.y.max(bounds.top);
    let to = (rect.y + rect.height).min(bounds.floor);
    if to <= from || x <= bounds.left || x >= bounds.right {
        return None;
    }
    // Cột pixel ngay bên ngoài cạnh, phía pet bám.
    let column = if side < 0 { x - 1.0 } else { x };
    let cuts: Vec<Span> = above
        .iter()
        .filter(|r| r.x <= column && r.x + r.width > column)
        .map(|r| Span { from: r.y, to: r.y + r.height })
        .collect();
    Some(Wall { id, side, x, from, to, open: subtract(&[Span { from, to }], &cuts) })
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOUNDS: Bounds = Bounds { left: 0.0, right: 1000.0, top: 0.0, floor: 700.0 };

    fn win(id: isize, x: f64, y: f64, width: f64, height: f64) -> WindowInfo {
        WindowInfo { id, rect: Rect { x, y, width, height } }
    }

    fn span(from: f64, to: f64) -> Span {
        Span { from, to }
    }

    #[test]
    fn mep_tren_la_cho_dung_cat_theo_man_hinh() {
        let terrain = Terrain::new(BOUNDS, vec![win(1, -100.0, 300.0, 400.0, 200.0)]);
        assert_eq!(
            terrain.ledge(1),
            Some(&Ledge { id: 1, y: 300.0, from: 0.0, to: 300.0, open: vec![span(0.0, 300.0)] })
        );
    }

    #[test]
    fn khong_co_mep_khi_cua_so_cham_tran_hoac_nam_duoi_mat_dat() {
        let terrain =
            Terrain::new(BOUNDS, vec![win(1, 0.0, 0.0, 1000.0, 700.0), win(2, 100.0, 700.0, 300.0, 200.0)]);
        assert!(terrain.ledges.is_empty());
        assert!(terrain.ledge(1).is_none());
    }

    #[test]
    fn phan_bi_cua_so_nam_tren_che_thi_khong_dung_duoc() {
        // Cửa sổ 1 nằm trên, đè lên giữa mép trên của cửa sổ 2.
        let terrain =
            Terrain::new(BOUNDS, vec![win(1, 300.0, 200.0, 200.0, 300.0), win(2, 100.0, 300.0, 600.0, 300.0)]);
        assert_eq!(terrain.ledge(2).unwrap().open, vec![span(100.0, 300.0), span(500.0, 700.0)]);
        // Cửa sổ nằm trên thì mép của nó không bị cửa sổ nằm dưới che.
        assert_eq!(terrain.ledge(1).unwrap().open, vec![span(300.0, 500.0)]);
    }

    #[test]
    fn cua_so_nam_tren_nhung_o_han_phia_tren_mep_thi_khong_che_mep() {
        let terrain =
            Terrain::new(BOUNDS, vec![win(1, 300.0, 100.0, 200.0, 150.0), win(2, 100.0, 300.0, 600.0, 300.0)]);
        assert_eq!(terrain.ledge(2).unwrap().open, vec![span(100.0, 700.0)]);
    }

    #[test]
    fn chi_cua_so_nam_tren_moi_che_duoc_pet_dung_tren_cua_so() {
        let terrain = Terrain::new(
            BOUNDS,
            vec![win(1, 0.0, 0.0, 50.0, 50.0), win(2, 100.0, 300.0, 600.0, 300.0), win(3, 0.0, 0.0, 20.0, 20.0)],
        );
        assert_eq!(terrain.above(2), &[Rect { x: 0.0, y: 0.0, width: 50.0, height: 50.0 }]);
        assert!(terrain.above(1).is_empty());
        assert!(terrain.covered(2, 10.0, 10.0));
        assert!(!terrain.covered(1, 10.0, 10.0));
    }

    #[test]
    fn hai_canh_ben_la_tuong_cat_theo_tran_va_mat_dat() {
        let terrain = Terrain::new(BOUNDS, vec![win(1, 200.0, 400.0, 300.0, 500.0)]);
        assert_eq!(
            terrain.wall(1, -1),
            Some(&Wall { id: 1, side: -1, x: 200.0, from: 400.0, to: 700.0, open: vec![span(400.0, 700.0)] })
        );
        assert_eq!(terrain.wall(1, 1).unwrap().x, 500.0);
    }

    #[test]
    fn canh_nam_ngoai_man_hinh_thi_khong_leo_duoc() {
        let terrain = Terrain::new(BOUNDS, vec![win(1, -50.0, 400.0, 300.0, 200.0)]);
        assert!(terrain.wall(1, -1).is_none());
        assert!(terrain.wall(1, 1).is_some());
    }

    #[test]
    fn phan_canh_bi_cua_so_nam_tren_che_thi_khong_leo_duoc() {
        let terrain =
            Terrain::new(BOUNDS, vec![win(1, 100.0, 450.0, 150.0, 100.0), win(2, 200.0, 400.0, 300.0, 300.0)]);
        assert_eq!(terrain.wall(2, -1).unwrap().open, vec![span(400.0, 450.0), span(550.0, 700.0)]);
    }

    #[test]
    fn subtract_cat_bo_cac_doan_chong_len_nhau() {
        assert_eq!(
            subtract(&[span(0.0, 100.0)], &[span(20.0, 40.0), span(30.0, 60.0), span(90.0, 200.0)]),
            vec![span(0.0, 20.0), span(60.0, 90.0)]
        );
    }
}
