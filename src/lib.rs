use serde::{Deserialize, Serialize};

#[derive(Debug, Eq, PartialEq, Copy, Clone, Serialize, Deserialize)]
pub enum LayoutStrategy {
    /// [ * | * | * | * ]. Some call this "vertical", notably vim
    Horizontal,
    /// [ *
    ///   _
    ///   *
    ///   _
    ///   *
    ///   _
    ///   * ]. Some call this "horizontal", notably vim.
    Vertical,
    // /// Only show the first window (navigate by rotating windows).
    // OnlyFirst,
}

#[derive(Debug, Default, Eq, PartialEq, Copy, Clone, Serialize, Deserialize, Hash)]
pub struct AreaSize {
    pub height: usize,
    pub width: usize,
}

#[derive(Debug, Default, Eq, PartialEq, Copy, Clone, Serialize, Deserialize, Hash)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

#[derive(Debug, Default, Eq, PartialEq, Copy, Clone, Serialize, Deserialize, Hash)]
pub struct WindowBounds {
    pub content: AreaSize,
    pub position: Position,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, Serialize, Deserialize)]
pub enum ItemIdx {
    Window(usize),
    Container(usize),
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, Serialize, Deserialize)]
pub struct Window {
    pub bounds: WindowBounds,
    pub parent: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Container {
    strategy: LayoutStrategy,
    children: Vec<(f64, ItemIdx)>,
    parent: Option<usize>, // None for root
    bounds: WindowBounds,
    inter: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layout {
    windows: Vec<Option<Window>>,
    containers: Vec<Option<Container>>, // 0 is the root
    root_bounds: WindowBounds,
}

#[derive(Debug, Clone, Eq, PartialEq, Copy)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

/// Used for the destination of a move
#[derive(Debug, Clone, Eq, PartialEq, Copy)]
pub enum MoveAction {
    Split(Direction),
    ToIndex(usize),
}

struct DescendantsIter<'a> {
    layout: &'a Layout,
    next: Option<ItemIdx>,
    orig: ItemIdx,
}

impl<'a> DescendantsIter<'a> {
    pub fn new(layout: &'a Layout, item: ItemIdx) -> Self {
        Self {
            layout,
            next: Some(item),
            orig: item,
        }
    }
}

impl<'a> Iterator for DescendantsIter<'a> {
    type Item = ItemIdx;

    fn next(&mut self) -> Option<Self::Item> {
        self.next.take().map(|next| {
            self.next = match next {
                ItemIdx::Container(c_idx) => self
                    .layout
                    .children(c_idx)
                    .get(0)
                    .map(|&(_weight, child)| child),
                ItemIdx::Window(_) => {
                    let mut exhausted = next;
                    loop {
                        if exhausted == self.orig {
                            break None;
                        }
                        let parent = match self.layout.parent_container(exhausted) {
                            Some(i) => i,
                            None => break None,
                        };
                        let parent_ctr = self.layout.containers[parent].as_ref().unwrap();
                        let index_in_parent = parent_ctr
                            .children
                            .iter()
                            .position(|&(_weight, child)| child == exhausted)
                            .unwrap();
                        if index_in_parent < parent_ctr.children.len() - 1 {
                            break Some(parent_ctr.children[index_in_parent + 1].1);
                        } else {
                            exhausted = ItemIdx::Container(parent);
                        }
                    }
                }
            };
            next
        })
    }
}
impl Layout {
    fn n_children(&self, item: ItemIdx) -> usize {
        match item {
            ItemIdx::Window(_) => 0,
            ItemIdx::Container(c_idx) => self.containers[c_idx].as_ref().unwrap().children.len(),
        }
    }
    fn iter_descendants(&self, item: ItemIdx) -> DescendantsIter<'_> {
        DescendantsIter::new(self, item)
    }
    /// Put a container where `split` was, and put `inserted` and `split` into that container, their order controlled by `inserted_first`.
    /// Returns the topmost modified container, but does not itself do layout.
    fn split(
        &mut self,
        inserted: ItemIdx,
        split: ItemIdx,
        strategy: LayoutStrategy,
        inserted_first: bool,
    ) -> usize {
        match self.parent_container(split) {
            Some(parent) => {
                let next_c_idx = self
                    .containers
                    .iter()
                    .position(Option::is_none)
                    .unwrap_or_else(|| {
                        self.containers.push(None);
                        self.containers.len() - 1
                    });
                let index_in_parent = self.containers[parent]
                    .as_ref()
                    .unwrap()
                    .children
                    .iter()
                    .position(|&(_weight, child)| child == split)
                    .unwrap();
                let bounds = self.bounds(split);
                self.containers[next_c_idx] = Some(Container {
                    strategy,
                    children: if inserted_first {
                        vec![(1.0, inserted), (1.0, split)]
                    } else {
                        vec![(1.0, split), (1.0, inserted)]
                    },
                    parent: Some(parent),
                    bounds,
                    inter: 0, // TODO - this should be configurable.
                });
                let ctr = self.containers[parent].as_mut().unwrap();
                ctr.children[index_in_parent].1 = ItemIdx::Container(next_c_idx);
                next_c_idx
            }
            None => {
                let root = self.containers[0].as_mut().unwrap();
                match root.children.len() {
                    0 => {
                        root.strategy = strategy;
                        root.children.push((1.0, inserted));
                        0
                    }
                    1 => {
                        let child = root.children[0].1;
                        self.split(inserted, child, strategy, inserted_first)
                    }
                    _ => {
                        let next_c_idx = self
                            .containers
                            .iter()
                            .position(Option::is_none)
                            .unwrap_or_else(|| {
                                self.containers.push(None);
                                self.containers.len() - 1
                            });
                        let Container {
                            strategy,
                            children,
                            bounds,
                            inter,
                            parent: _,
                        } = self.containers[0].take().unwrap();
                        let new_ctr = Container {
                            strategy,
                            children,
                            bounds: Default::default(),
                            inter,
                            parent: Some(0),
                        };
                        self.containers[next_c_idx] = Some(new_ctr);
                        self.containers[0] = Some(Container {
                            strategy: LayoutStrategy::Horizontal,
                            children: if inserted_first {
                                vec![(1.0, inserted), (1.0, ItemIdx::Container(next_c_idx))]
                            } else {
                                vec![(1.0, ItemIdx::Container(next_c_idx)), (1.0, inserted)]
                            },
                            parent: None,
                            bounds,
                            inter: 0,
                        });
                        0
                    }
                }
            }
        }
    }
    fn layout(&mut self, item: ItemIdx, out: &mut Vec<LayoutAction>) {
        let c_idx = match item {
            ItemIdx::Container(idx) => idx,
            ItemIdx::Window(_) => return,
        };
        let ctr = self.containers[c_idx].as_ref().unwrap();
        let strat = ctr.strategy;
        let ctr_bounds = ctr.bounds;
        let total_inter = ctr.inter * (ctr.children.len().saturating_sub(1));
        let available_area = match strat {
            LayoutStrategy::Vertical => AreaSize {
                height: ctr_bounds.content.height - total_inter,
                width: ctr_bounds.content.width,
            },
            LayoutStrategy::Horizontal => AreaSize {
                height: ctr_bounds.content.height,
                width: ctr_bounds.content.width - total_inter,
            },
        };
        let begin = match strat {
            LayoutStrategy::Vertical => ctr_bounds.position.y + ctr.inter,
            LayoutStrategy::Horizontal => ctr_bounds.position.x + ctr.inter,
        };
        let total_weight: f64 = ctr.children.iter().map(|(weight, _)| weight).sum();
        let mut cursor = ctr_bounds.position;
        let inter = ctr.inter;
        let mut to_fix = vec![];
        let mut cumsum = 0.0;
        for &(weight, child) in ctr.children.iter() {
            let normalized: f64 = weight / total_weight;
            cumsum += normalized;
            let old_bounds = self.bounds(child);
            let content = match strat {
                LayoutStrategy::Vertical => {
                    let new_y = (begin as f64 + cumsum * available_area.height as f64) as usize;
                    AreaSize {
                        height: new_y - cursor.y,
                        width: available_area.width,
                    }
                }
                LayoutStrategy::Horizontal => {
                    let new_x = (begin as f64 + cumsum * available_area.width as f64) as usize;
                    AreaSize {
                        height: available_area.height,
                        width: new_x - cursor.x,
                    }
                }
            };
            let new_bounds = WindowBounds {
                content,
                position: cursor,
            };
            cursor = match strat {
                LayoutStrategy::Vertical => Position {
                    x: cursor.x,
                    y: cursor.y + content.height + inter,
                },
                LayoutStrategy::Horizontal => Position {
                    y: cursor.y,
                    x: cursor.x + content.width + inter,
                },
            };

            if new_bounds != old_bounds {
                to_fix.push((child, new_bounds));
            }
        }
        for (idx, bounds) in to_fix {
            match idx {
                ItemIdx::Window(w_idx) => {
                    self.windows[w_idx].as_mut().unwrap().bounds = bounds;
                    out.push(LayoutAction::NewWindowBounds { idx: w_idx, bounds });
                }
                ItemIdx::Container(c_idx) => {
                    self.containers[c_idx].as_mut().unwrap().bounds = bounds;
                    self.layout(idx, out);
                }
            }
        }
    }
    // Doesn't layout. Returns container modified.
    fn insert(&mut self, from: ItemIdx, to: ItemIdx, at: MoveAction) -> usize {
        let to_ctr = match at {
            MoveAction::Split(dir) => match dir {
                Direction::Up => self.split(from, to, LayoutStrategy::Vertical, true),
                Direction::Down => self.split(from, to, LayoutStrategy::Vertical, false),
                Direction::Left => self.split(from, to, LayoutStrategy::Horizontal, true),
                Direction::Right => self.split(from, to, LayoutStrategy::Horizontal, false),
            },
            MoveAction::ToIndex(to_idx) => {
                let to_ctr = match to {
                    ItemIdx::Window(_) => panic!(),
                    ItemIdx::Container(idx) => idx,
                };
                self.containers[to_ctr]
                    .as_mut()
                    .unwrap()
                    .children
                    .insert(to_idx, (1.0, from));
                to_ctr
            }
        };
        match from {
            ItemIdx::Container(idx) => self.containers[idx].as_mut().unwrap().parent = Some(to_ctr),
            ItemIdx::Window(idx) => self.windows[idx].as_mut().unwrap().parent = Some(to_ctr),
        };
        if matches!(at, MoveAction::Split(_)) {
            match to {
                ItemIdx::Container(idx) => {
                    if idx != 0 {
                        self.containers[idx].as_mut().unwrap().parent = Some(to_ctr)
                    }
                }
                ItemIdx::Window(idx) => self.windows[idx].as_mut().unwrap().parent = Some(to_ctr),
            };
        }
        to_ctr
    }
}

impl Layout {
    pub fn new_in_bounds(bounds: WindowBounds) -> Self {
        let mut this = Self {
            windows: Default::default(),
            containers: Default::default(),
            root_bounds: bounds,
        };
        this.containers.push(Some(Container {
            bounds,
            children: Default::default(),
            inter: 0,
            parent: None,
            strategy: LayoutStrategy::Horizontal,
        }));
        this
    }
    pub fn windows<'a>(&'a self) -> impl Iterator<Item = &'a Window> {
        self.windows.iter().filter_map(Option::as_ref)
    }
    pub fn resize(&mut self, bounds: WindowBounds) -> Vec<LayoutAction> {
        self.containers[0].as_mut().unwrap().bounds = bounds;
        self.root_bounds = bounds;
        let mut out = vec![];
        self.layout(ItemIdx::Container(0), &mut out);
        out
    }
    pub fn parent_container(&self, item: ItemIdx) -> Option<usize> {
        match item {
            ItemIdx::Window(idx) => Some(self.windows[idx].unwrap().parent?),
            ItemIdx::Container(idx) => self.containers[idx].as_ref().unwrap().parent,
        }
    }
    fn topo_next_recursive(&self, item: ItemIdx) -> Option<ItemIdx> {
        println!("in tnr with item {:?}", item);
        self.parent_container(item).and_then(|parent| {
            let parent_ctr = self.containers[parent].as_ref().unwrap();
            let idx_in_parent = parent_ctr
                .children
                .iter()
                .position(|&(_weight, child)| child == item)
                .unwrap();
            parent_ctr
                .children
                .get(idx_in_parent + 1)
                .map(|(_weight, child)| child)
                .copied()
                .or_else(|| self.topo_next_recursive(ItemIdx::Container(parent)))
        })
    }
    pub fn topological_next(&self, item: ItemIdx) -> Option<ItemIdx> {
        println!("{}", serde_json::to_string_pretty(self).unwrap());
        self.topo_next_recursive(item)
    }
    pub fn topological_last(&self) -> ItemIdx {
        let mut cur = ItemIdx::Container(0);
        loop {
            match cur {
                ItemIdx::Container(c_idx) => {
                    let cur_ctr = self.containers[c_idx].as_ref().unwrap();
                    match cur_ctr.children.last().copied() {
                        Some((_weight, next)) => cur = next,
                        None => return cur,
                    }
                }
                ItemIdx::Window(_) => return cur,
            }
        }
    }
    pub fn destroy(&mut self, item: ItemIdx) -> Vec<LayoutAction> {
        let to_destroy = self.iter_descendants(item).collect::<Vec<_>>();
        let mut result = to_destroy
            .iter()
            .copied()
            .filter_map(|descendant| match descendant {
                ItemIdx::Window(idx) => Some(LayoutAction::WindowDestroyed { idx }),
                ItemIdx::Container(_) => None,
            })
            .collect::<Vec<_>>();
        let parent = self.parent_container(item);
        for item in to_destroy.iter().copied() {
            match item {
                ItemIdx::Window(idx) => self.windows[idx] = None,
                ItemIdx::Container(idx) => self.containers[idx] = None,
            }
        }
        match parent {
            None => {
                // we destroyed the root, but there must always be a root.
                self.containers[0] = Some(Container {
                    strategy: LayoutStrategy::Horizontal,
                    children: vec![],
                    parent: None,
                    inter: Default::default(),
                    bounds: self.root_bounds,
                });
            }
            Some(mut parent) => {
                let parent_ctr = self.containers[parent].as_mut().unwrap();
                let index_in_parent = parent_ctr
                    .children
                    .iter()
                    .position(|(_, child_idx)| *child_idx == item)
                    .unwrap();
                parent_ctr.children.remove(index_in_parent);
                // fuse if necessary
                if let Some(grandparent) = self.parent_container(ItemIdx::Container(parent)) {
                    let parent_ctr = self.containers[parent].as_ref().unwrap();
                    let gp_ctr = self.containers[grandparent].as_ref().unwrap();
                    if parent_ctr.children.len() == 1 {
                        let index_in_gp = gp_ctr
                            .children
                            .iter()
                            .position(|(_, child_idx)| *child_idx == ItemIdx::Container(parent))
                            .unwrap();
                        let child = parent_ctr.children[0].1;
                        let gp_ctr = self.containers[grandparent].as_mut().unwrap();
                        gp_ctr.children[index_in_gp].1 = child;
                        self.containers[parent] = None;
                        *(match child {
                            ItemIdx::Container(idx) => {
                                &mut self.containers[idx].as_mut().unwrap().parent
                            }
                            ItemIdx::Window(idx) => &mut self.windows[idx].as_mut().unwrap().parent,
                        }) = Some(grandparent);
                        parent = grandparent;
                    }
                }
                self.layout(ItemIdx::Container(parent), &mut result);
            }
        };
        result
    }
    pub fn is_ancestor(&self, ancestor: ItemIdx, descendant: ItemIdx) -> bool {
        if ancestor == descendant {
            return true;
        }
        let a_ctr = match ancestor {
            ItemIdx::Container(idx) => idx,
            ItemIdx::Window(_) => return false,
        };
        while let Some(parent) = self.parent_container(descendant) {
            if parent == a_ctr {
                return true;
            }
        }
        false
    }
    pub fn alloc_window(&mut self) -> usize {
        let next_idx = self
            .windows
            .iter()
            .position(Option::is_none)
            .unwrap_or_else(|| {
                self.windows.push(None);
                self.windows.len() - 1
            });
        self.windows[next_idx] = Some(Window {
            bounds: Default::default(),
            parent: None,
        });
        next_idx
    }
    pub fn move_(&mut self, from: ItemIdx, to: ItemIdx, at: MoveAction) -> Vec<LayoutAction> {
        if self.is_ancestor(from, to) {
            panic!()
        }
        // we know `from` is not the root, because of the ancestry check above.
        // So the unwrap is safe.
        let from_parent = self.parent_container(from);
        let idx_in_parent = if let Some(from_parent) = from_parent {
            let parent_ctr = self.containers[from_parent].as_mut().unwrap();
            let idx_in_parent = parent_ctr
                .children
                .iter()
                .position(|&(_weight, child)| child == from)
                .unwrap();
            parent_ctr.children.remove(idx_in_parent);
            Some(idx_in_parent)
        } else {
            None
        };
        let at = match at {
            MoveAction::ToIndex(idx)
                if from_parent.is_some()
                    && to == ItemIdx::Container(from_parent.unwrap())
                    && idx >= idx_in_parent.unwrap() =>
            {
                MoveAction::ToIndex(idx.saturating_sub(1))
            }
            _ => at,
        };
        let insert_modified = self.insert(from, to, at);
        let mut result = vec![];
        self.layout(ItemIdx::Container(insert_modified), &mut result);
        if from_parent.is_some()
            && !self.is_ancestor(
                ItemIdx::Container(insert_modified),
                ItemIdx::Container(from_parent.unwrap()),
            )
        {
            self.layout(ItemIdx::Container(from_parent.unwrap()), &mut result);
        }
        result
    }
    pub fn navigate(
        &self,
        from: ItemIdx,
        dir: Direction,
        cursor: Option<Position>,
    ) -> Option<ItemIdx> {
        let mut ancestor = None;
        let mut cur = from;
        let cursor = cursor.unwrap_or_else(|| self.bounds(from).position);
        while let Some(parent) = self.parent_container(cur) {
            let parent_ctr = self.containers[parent].as_ref().unwrap();
            let strat = parent_ctr.strategy;
            let can_go_back = ((dir == Direction::Left && strat == LayoutStrategy::Horizontal)
                || (dir == Direction::Up && strat == LayoutStrategy::Vertical))
                && parent_ctr.children[0].1 != cur;
            if can_go_back {
                let index_in_parent = parent_ctr
                    .children
                    .iter()
                    .position(|(_weight, child)| *child == cur)
                    .unwrap();
                ancestor = Some(parent_ctr.children[index_in_parent - 1].1);
                break;
            }
            let can_go_fwd = ((dir == Direction::Right && strat == LayoutStrategy::Horizontal)
                || (dir == Direction::Down && strat == LayoutStrategy::Vertical))
                && parent_ctr.children.last().unwrap().1 != cur;
            if can_go_fwd {
                let index_in_parent = parent_ctr
                    .children
                    .iter()
                    .position(|(_weight, child)| *child == cur)
                    .unwrap();
                ancestor = Some(parent_ctr.children[index_in_parent + 1].1);
                break;
            }
            cur = ItemIdx::Container(parent);
        }
        let move_horizontal = (dir == Direction::Left || dir == Direction::Right);
        let move_to_first = (dir == Direction::Right || dir == Direction::Down);
        ancestor.map(|mut cur| {
            while let ItemIdx::Container(c_idx) = cur {
                let ctr = self.containers[c_idx].as_ref().unwrap();
                cur = if move_horizontal == (ctr.strategy == LayoutStrategy::Horizontal) {
                    if move_to_first {
                        ctr.children[0].1
                    } else {
                        ctr.children.iter().last().unwrap().1
                    }
                } else {
                    let Position { x, y } = cursor;
                    let seek_coord = if move_horizontal { y } else { x };
                    let idx = ctr
                        .children
                        .iter()
                        .position(|&(_weight, child)| {
                            let bounds = self.bounds(child);
                            let (child_lb, child_ub) = if move_horizontal {
                                (bounds.position.y, bounds.position.y + bounds.content.height)
                            } else {
                                (bounds.position.x, bounds.position.x + bounds.content.width)
                            };
                            child_lb <= seek_coord && seek_coord < child_ub
                        })
                        .unwrap_or(0);
                    ctr.children[idx].1
                }
            }
            cur
        })
    }
    pub fn children(&self, container: usize) -> &[(f64, ItemIdx)] {
        &self.containers[container].as_ref().unwrap().children
    }
    pub fn nearest_container(&self, item: ItemIdx) -> usize {
        match item {
            ItemIdx::Container(c_idx) => c_idx,
            ItemIdx::Window(w_idx) => {
                self.nearest_container(ItemIdx::Container(self.parent_container(item).unwrap()))
            }
        }
    }
    pub fn bounds(&self, item: ItemIdx) -> WindowBounds {
        match item {
            ItemIdx::Container(idx) => self.containers[idx].as_ref().unwrap().bounds,
            ItemIdx::Window(idx) => self.windows[idx].as_ref().unwrap().bounds,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, Serialize, Deserialize)]
pub enum LayoutAction {
    /// A window has moved or been created.
    NewWindowBounds { idx: usize, bounds: WindowBounds },
    /// A window has been destroyed.
    WindowDestroyed { idx: usize },
    /// A window still exists, but is no longer visible.
    WindowHidden { idx: usize },
}
