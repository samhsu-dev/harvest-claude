use std::collections::{HashSet, VecDeque};

use crate::types::TilePos;

/// BFS on a 4-connected grid from `from` to `to`.
///
/// `own_seat` is temporarily treated as walkable (the character's assigned seat
/// may be in the blocked set). Path excludes `from`, includes `to`.
/// Returns `None` if unreachable. Returns empty `Vec` if `from == to`.
pub fn bfs(
    walkable: &HashSet<TilePos>,
    from: TilePos,
    to: TilePos,
    own_seat: Option<TilePos>,
) -> Option<Vec<TilePos>> {
    if from == to {
        return Some(Vec::new());
    }

    let mut visited = HashSet::new();
    visited.insert(from);

    // parent map: for each visited tile, store where we came from
    let mut parent: std::collections::HashMap<TilePos, TilePos> = std::collections::HashMap::new();

    let mut queue = VecDeque::new();
    queue.push_back(from);

    while let Some(current) = queue.pop_front() {
        for neighbor in neighbors(current) {
            if visited.contains(&neighbor) {
                continue;
            }
            let passable = walkable.contains(&neighbor) || own_seat.is_some_and(|s| s == neighbor);
            if !passable {
                continue;
            }

            visited.insert(neighbor);
            parent.insert(neighbor, current);

            if neighbor == to {
                return Some(reconstruct_path(&parent, from, to));
            }

            queue.push_back(neighbor);
        }
    }

    None
}

/// 4-connected neighbors (up, down, left, right), filtering out underflow.
fn neighbors(pos: TilePos) -> impl Iterator<Item = TilePos> {
    let (col, row) = pos;
    let candidates: [(i32, i32); 4] = [
        (col as i32, row as i32 - 1),
        (col as i32, row as i32 + 1),
        (col as i32 - 1, row as i32),
        (col as i32 + 1, row as i32),
    ];
    candidates
        .into_iter()
        .filter(|&(c, r)| c >= 0 && r >= 0 && c <= u16::MAX as i32 && r <= u16::MAX as i32)
        .map(|(c, r)| (c as u16, r as u16))
}

// Trace parent chain from `to` back to `from`, excluding `from`.
fn reconstruct_path(
    parent: &std::collections::HashMap<TilePos, TilePos>,
    from: TilePos,
    to: TilePos,
) -> Vec<TilePos> {
    let mut path = Vec::new();
    let mut current = to;
    while current != from {
        path.push(current);
        // Safety: every tile except `from` has a parent entry
        current = parent[&current];
    }
    path.reverse();
    path
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::bfs;

    #[test]
    fn same_tile_returns_empty() {
        let walkable: HashSet<_> = [(1, 1)].into_iter().collect();
        let result = bfs(&walkable, (1, 1), (1, 1), None);
        assert_eq!(result, Some(vec![]));
    }

    #[test]
    fn adjacent_tile() {
        let walkable: HashSet<_> = [(0, 0), (1, 0)].into_iter().collect();
        let result = bfs(&walkable, (0, 0), (1, 0), None);
        assert_eq!(result, Some(vec![(1, 0)]));
    }

    #[test]
    fn unreachable_returns_none() {
        let walkable: HashSet<_> = [(0, 0), (2, 0)].into_iter().collect();
        let result = bfs(&walkable, (0, 0), (2, 0), None);
        assert_eq!(result, None);
    }

    #[test]
    fn path_excludes_from_includes_to() {
        let walkable: HashSet<_> = [(0, 0), (1, 0), (2, 0)].into_iter().collect();
        let result = bfs(&walkable, (0, 0), (2, 0), None).unwrap();
        assert!(!result.contains(&(0, 0)));
        assert!(result.contains(&(2, 0)));
    }

    #[test]
    fn own_seat_unblocks_destination() {
        // to=(1,0) is not in walkable, but is own_seat
        let walkable: HashSet<_> = [(0, 0)].into_iter().collect();
        let result = bfs(&walkable, (0, 0), (1, 0), Some((1, 0)));
        assert_eq!(result, Some(vec![(1, 0)]));
    }

    #[test]
    fn l_shaped_path() {
        let walkable: HashSet<_> = [(0, 0), (1, 0), (1, 1), (1, 2)].into_iter().collect();
        let result = bfs(&walkable, (0, 0), (1, 2), None).unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(*result.last().unwrap(), (1, 2));
    }
}
