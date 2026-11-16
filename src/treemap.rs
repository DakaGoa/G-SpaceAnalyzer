use eframe::egui;
use egui::{pos2, vec2, Rect};

use crate::model::{NodeId, Tree};

#[derive(Clone, Debug)]
pub struct Tile {
    pub node: NodeId,
    pub rect: Rect,
    pub depth: u8,
    pub parent: Option<usize>,
    pub is_dir: bool,
    pub expanded: bool,
    pub group_count: u32,
    pub size: u64,
}

#[derive(Clone, Debug, Default)]
pub struct TreemapLayout {
    pub tiles: Vec<Tile>,
    pub truncated: bool,
}

impl TreemapLayout {
    pub fn hit_test(&self, point: egui::Pos2) -> Option<usize> {
        self.tiles
            .iter()
            .enumerate()
            .rev()
            .find(|(_, tile)| tile.rect.contains(point))
            .map(|(idx, _)| idx)
    }
}

struct Item {
    node: NodeId,
    size: u64,
    group_count: u32,
}

pub fn build(tree: &Tree, root: NodeId, rect: Rect, max_tiles: usize) -> TreemapLayout {
    let mut layout = TreemapLayout::default();
    if rect.width() < 20.0 || rect.height() < 20.0 {
        return layout;
    }
    let inner = rect.shrink(3.0);
    layout_children(
        tree,
        root,
        inner,
        0,
        None,
        &mut layout,
        max_tiles,
        0,
    );
    layout
}

#[allow(clippy::too_many_arguments)]
fn layout_children(
    tree: &Tree,
    node_id: NodeId,
    rect: Rect,
    depth: u8,
    parent_tile: Option<usize>,
    layout: &mut TreemapLayout,
    max_tiles: usize,
    recursion: u8,
) {
    if depth > 12 || recursion > 12 || layout.tiles.len() >= max_tiles {
        layout.truncated = true;
        return;
    }
    let node = tree.node(node_id);
    if node.children.is_empty() {
        if depth > 0 {
            layout.tiles.push(Tile {
                node: node_id,
                rect,
                depth,
                parent: parent_tile,
                is_dir: node.is_dir,
                expanded: false,
                group_count: 0,
                size: node.size,
            });
        }
        return;
    }

    let mut items: Vec<Item> = node
        .children
        .iter()
        .filter(|id| tree.node(**id).size > 0)
        .map(|id| Item {
            node: *id,
            size: tree.node(*id).size,
            group_count: 0,
        })
        .collect();
    if items.is_empty() {
        if depth > 0 {
            layout.tiles.push(Tile {
                node: node_id,
                rect,
                depth,
                parent: parent_tile,
                is_dir: node.is_dir,
                expanded: false,
                group_count: 0,
                size: node.size,
            });
        }
        return;
    }
    // Keep the map readable: collapse a long tail into one "smaller items" tile.
    if items.len() > 140 {
        let keep = 119;
        let tail = items.split_off(keep);
        let tail_size: u64 = tail.iter().map(|item| item.size).sum();
        let tail_count = tail.len() as u32;
        items.push(Item {
            node: node_id,
            size: tail_size,
            group_count: tail_count,
        });
    }
    let values: Vec<f64> = items.iter().map(|item| item.size as f64).collect();
    let cell_rects = squarify(&values, rect);
    if cell_rects.len() != items.len() {
        return;
    }

    for (item, cell) in items.iter().zip(cell_rects) {
        if layout.tiles.len() >= max_tiles {
            layout.truncated = true;
            return;
        }
        let gap = if depth == 0 { 2.0 } else { 1.15 };
        let tile_rect = cell.shrink(gap);
        if tile_rect.width() < 1.0 || tile_rect.height() < 1.0 {
            continue;
        }
        if item.group_count > 0 {
            layout.tiles.push(Tile {
                node: item.node,
                rect: tile_rect,
                depth: depth + 1,
                parent: parent_tile,
                is_dir: tree.node(item.node).is_dir,
                expanded: false,
                group_count: item.group_count,
                size: item.size,
            });
            continue;
        }
        let child = tree.node(item.node);
        let can_expand = child.is_dir
            && !child.children.is_empty()
            && tile_rect.width() > 14.0
            && tile_rect.height() > 14.0
            && tile_rect.area() > 420.0
            && depth < 8;
        if child.is_dir {
            let tile_index = layout.tiles.len();
            layout.tiles.push(Tile {
                node: item.node,
                rect: tile_rect,
                depth: depth + 1,
                parent: parent_tile,
                is_dir: true,
                expanded: can_expand,
                group_count: 0,
                size: item.size,
            });
            if can_expand {
                let inner = tile_rect.shrink(1.0);
                layout_children(
                    tree,
                    item.node,
                    inner,
                    depth + 1,
                    Some(tile_index),
                    layout,
                    max_tiles,
                    recursion + 1,
                );
            }
        } else {
            layout.tiles.push(Tile {
                node: item.node,
                rect: tile_rect,
                depth: depth + 1,
                parent: parent_tile,
                is_dir: false,
                expanded: false,
                group_count: 0,
                size: item.size,
            });
        }
    }
}

/// Squarified treemap layout. Values are positive and need not be sorted, but
/// sorting them descending gives the best aspect ratios.
pub fn squarify(values: &[f64], rect: Rect) -> Vec<Rect> {
    let mut out = vec![Rect::NOTHING; values.len()];
    if values.is_empty() || rect.width() <= 0.0 || rect.height() <= 0.0 {
        return out;
    }
    let total: f64 = values.iter().filter(|value| **value > 0.0).sum();
    if total <= 0.0 {
        return out;
    }
    let mut order: Vec<usize> = (0..values.len())
        .filter(|idx| values[*idx] > 0.0)
        .collect();
    order.sort_unstable_by(|a, b| {
        values[*b]
            .partial_cmp(&values[*a])
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let scale = (rect.width() as f64 * rect.height() as f64) / total;
    let mut free = rect;
    let mut row: Vec<(usize, f64)> = Vec::new();
    let mut index = 0usize;

    while index < order.len() {
        let item = (order[index], values[order[index]] * scale);
        let side = free.width().min(free.height()) as f64;
        if row.is_empty() || worst_ratio(&row, side) >= worst_ratio_with(&row, item.1, side) {
            row.push(item);
            index += 1;
        } else {
            place_row(&mut row, &mut free, &mut out);
        }
    }
    if !row.is_empty() {
        place_row(&mut row, &mut free, &mut out);
    }
    out
}

fn worst_ratio(row: &[(usize, f64)], side: f64) -> f64 {
    if row.is_empty() || side <= 0.0 {
        return f64::INFINITY;
    }
    let sum: f64 = row.iter().map(|(_, area)| *area).sum();
    if sum <= 0.0 {
        return f64::INFINITY;
    }
    let max = row
        .iter()
        .map(|(_, area)| *area)
        .fold(f64::MIN, f64::max);
    let min = row
        .iter()
        .map(|(_, area)| *area)
        .fold(f64::MAX, f64::min);
    ((side * side * max) / (sum * sum)).max((sum * sum) / (side * side * min))
}

fn worst_ratio_with(row: &[(usize, f64)], extra: f64, side: f64) -> f64 {
    if side <= 0.0 {
        return f64::INFINITY;
    }
    let mut sum: f64 = row.iter().map(|(_, area)| *area).sum::<f64>() + extra;
    if sum <= 0.0 {
        return f64::INFINITY;
    }
    let mut max = extra;
    let mut min = extra;
    for (_, area) in row {
        max = max.max(*area);
        min = min.min(*area);
    }
    ((side * side * max) / (sum * sum)).max((sum * sum) / (side * side * min))
}

fn place_row(row: &mut Vec<(usize, f64)>, free: &mut Rect, out: &mut [Rect]) {
    if row.is_empty() {
        return;
    }
    let sum: f64 = row.iter().map(|(_, area)| *area).sum();
    if sum <= 0.0 {
        row.clear();
        return;
    }
    if free.width() >= free.height() {
        let strip_width = (sum / free.height() as f64) as f32;
        let strip_width = strip_width.clamp(0.0, free.width());
        let mut y = free.top();
        for (idx, area) in row.iter() {
            let height = (area / strip_width.max(0.001) as f64) as f32;
            out[*idx] = Rect::from_min_max(
                pos2(free.left(), y),
                pos2(free.left() + strip_width, (y + height).min(free.bottom())),
            );
            y += height;
        }
        free.min.x += strip_width;
    } else {
        let strip_height = (sum / free.width() as f64) as f32;
        let strip_height = strip_height.clamp(0.0, free.height());
        let mut x = free.left();
        for (idx, area) in row.iter() {
            let width = (area / strip_height.max(0.001) as f64) as f32;
            out[*idx] = Rect::from_min_max(
                pos2(x, free.top()),
                pos2((x + width).min(free.right()), free.top() + strip_height),
            );
            x += width;
        }
        free.min.y += strip_height;
    }
    row.clear();
}

/// A tiny geometry helper used by the overview illustration.
pub fn art_blocks(rect: Rect) -> [Rect; 5] {
    let gap = 4.0;
    let w = rect.width();
    let h = rect.height();
    let col = (w - gap * 2.0) * 0.56;
    let right = w - col - gap * 2.0;
    [
        Rect::from_min_size(rect.min, vec2(col, h)),
        Rect::from_min_size(pos2(rect.left() + col + gap, rect.top()), vec2(right, h * 0.42)),
        Rect::from_min_size(
            pos2(rect.left() + col + gap, rect.top() + h * 0.42 + gap),
            vec2(right, h * 0.24),
        ),
        Rect::from_min_size(
            pos2(rect.left() + col + gap, rect.top() + h * 0.66 + gap * 2.0),
            vec2(right, h * 0.34 - gap * 2.0),
        ),
        Rect::from_min_size(pos2(rect.left(), rect.top() + h * 0.58), vec2(col, h * 0.42)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::demo_tree;

    fn area(rect: Rect) -> f64 {
        rect.width().max(0.0) as f64 * rect.height().max(0.0) as f64
    }

    #[test]
    fn squarify_fills_the_rect() {
        let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 500.0));
        let values = [500.0, 250.0, 120.0, 80.0, 30.0, 20.0];
        let rects = squarify(&values, rect);
        let total: f64 = values.iter().sum();
        for (value, rect) in values.iter().zip(&rects) {
            let expected = area(*rect) / area(rects[0].union(*rect)).max(1.0);
            assert!(expected > 0.0);
            assert!(area(*rect) > 0.0);
            let ratio = area(*rect) / (rect.width() as f64 * rect.height() as f64);
            assert!((ratio - 1.0).abs() < 0.001);
            assert!(area(*rect) / total > 0.0);
        }
        let union = rects
            .iter()
            .fold(Rect::NOTHING, |acc, item| acc.union(*item));
        assert!((union.width() - rect.width()).abs() < 2.0);
        assert!((union.height() - rect.height()).abs() < 2.0);
        for i in 0..rects.len() {
            for j in (i + 1)..rects.len() {
                let overlap = rects[i].intersect(rects[j]);
                assert!(overlap.width() <= 0.01 || overlap.height() <= 0.01);
            }
        }
    }

    #[test]
    fn demo_layout_has_tiles_and_hit_testing() {
        let tree = demo_tree();
        let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 620.0));
        let layout = build(&tree, tree.root, rect, 8000);
        assert!(layout.tiles.len() > 20);
        assert!(layout.tiles.iter().any(|tile| !tile.is_dir));
        let hit = layout.hit_test(rect.center());
        assert!(hit.is_some());
        for tile in &layout.tiles {
            assert!(rect.contains_rect(tile.rect));
            assert!(tile.size > 0);
        }
    }
}
