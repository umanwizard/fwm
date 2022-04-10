use log::info;
use serde::{Deserialize, Serialize};
use x11::xlib::XSetWindowBackground;

pub mod scheme;

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

impl WindowBounds {
    pub fn contains(&self, position: Position) -> bool {
        self.position.x <= position.x
            && self.position.y <= position.y
            && position.x < (self.position.x + self.content.width)
            && position.y < (self.position.y + self.content.height)
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, Serialize, Deserialize, Hash)]
pub enum ItemIdx {
    Window(usize),
    Container(usize),
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum ItemAndData<W, C> {
    Window(usize, W),
    Container(usize, C),
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, Serialize, Deserialize)]
pub struct Window<W> {
    pub bounds: WindowBounds,
    pub parent: Option<usize>,
    pub data: W,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Container<C> {
    strategy: LayoutStrategy,
    children: Vec<(f64, ItemIdx)>,
    parent: Option<usize>, // None for root
    bounds: WindowBounds,
    inter: usize,
    padding: usize,
    data: C,
}

pub enum LayoutData<W, C> {
    Window(W),
    Container(C),
}

pub enum LayoutDataRef<'a, W, C> {
    Window(&'a W),
    Container(&'a C),
}

pub enum LayoutDataMut<'a, W, C> {
    Window(&'a mut W),
    Container(&'a mut C),
}

impl<W, C> LayoutData<W, C> {
    pub fn unwrap_window(self) -> W {
        match self {
            Self::Window(data) => data,
            _ => panic!("Unwrapped wrong variant"),
        }
    }

    pub fn unwrap_container(self) -> C {
        match self {
            Self::Container(data) => data,
            _ => panic!("Unwrapped wrong variant"),
        }
    }
}

impl<'a, W, C> LayoutDataRef<'a, W, C> {
    pub fn unwrap_window(self) -> &'a W {
        match self {
            Self::Window(data) => data,
            _ => panic!("Unwrapped wrong variant"),
        }
    }

    pub fn unwrap_container(self) -> &'a C {
        match self {
            Self::Container(data) => data,
            _ => panic!("Unwrapped wrong variant"),
        }
    }
}

impl<'a, W, C> LayoutDataMut<'a, W, C> {
    pub fn unwrap_window(self) -> &'a mut W {
        match self {
            Self::Window(data) => data,
            _ => panic!("Unwrapped wrong variant"),
        }
    }

    pub fn unwrap_container(self) -> &'a mut C {
        match self {
            Self::Container(data) => data,
            _ => panic!("Unwrapped wrong variant"),
        }
    }
}

pub trait Constructor {
    type Item;

    fn construct(&mut self) -> Self::Item;
}

#[derive(Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct SlotInContainer {
    pub c_idx: usize,
    pub index: usize,
    pub parent_strat: LayoutStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layout<W, C, CCtor> {
    windows: Vec<Option<Window<W>>>,
    containers: Vec<Option<Container<C>>>, // 0 is the root
    root_bounds: WindowBounds,
    default_padding: usize,
    #[serde(skip)]
    cctor: Option<CCtor>,
}

#[derive(Debug, Clone, Eq, PartialEq, Copy, Serialize, Deserialize)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum MoveCursor {
    Split { item: ItemIdx, direction: Direction },
    Into { container: usize, index: usize },
}

impl MoveCursor {
    pub fn item(&self) -> ItemIdx {
        match self {
            MoveCursor::Split { item, .. } => *item,
            MoveCursor::Into { container, .. } => ItemIdx::Container(*container),
        }
    }
}

pub struct DescendantsIter<'a, W, C, CCtor> {
    layout: &'a Layout<W, C, CCtor>,
    next: Option<ItemIdx>,
    orig: ItemIdx,
}

impl<'a, W, C, CCtor> DescendantsIter<'a, W, C, CCtor> {
    pub fn new(layout: &'a Layout<W, C, CCtor>, item: ItemIdx) -> Self {
        Self {
            layout,
            next: Some(item),
            orig: item,
        }
    }
}

trait IterExt {
    type Item;

    fn unwrap_one(self) -> Self::Item;
}

impl<I> IterExt for I
where
    I: IntoIterator,
{
    type Item = <Self as IntoIterator>::Item;

    fn unwrap_one(self) -> Self::Item {
        let mut i = self.into_iter();
        let first = i.next();
        let second = i.next();
        match (first, second) {
            (Some(item), None) => item,
            _ => panic!("Expected exactly one item"),
        }
    }
}

impl<'a, W, C, CCtor> Iterator for DescendantsIter<'a, W, C, CCtor>
where
    W: Serialize + for<'d> Deserialize<'d>,
    C: Serialize + for<'d> Deserialize<'d>,
    CCtor: Constructor<Item = C>,
{
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
impl<W, C, CCtor> Layout<W, C, CCtor>
where
    W: Serialize + for<'d> Deserialize<'d>,
    C: Serialize + for<'d> Deserialize<'d>,
    CCtor: Constructor<Item = C>,
{
    fn try_window(&self, w_idx: usize) -> Option<&Window<W>> {
        self.windows.get(w_idx).and_then(|mw| mw.as_ref())
    }
    fn try_window_mut(&mut self, w_idx: usize) -> Option<&mut Window<W>> {
        self.windows.get_mut(w_idx).and_then(|mw| mw.as_mut())
    }
    fn try_container(&self, c_idx: usize) -> Option<&Container<C>> {
        self.containers.get(c_idx).and_then(|mc| mc.as_ref())
    }
    fn try_container_mut(&mut self, c_idx: usize) -> Option<&mut Container<C>> {
        self.containers.get_mut(c_idx).and_then(|mc| mc.as_mut())
    }

    pub fn is_cursor_valid(&self, cursor: MoveCursor) -> bool {
        match cursor {
            MoveCursor::Split { item, direction: _ } => self.exists(item),
            MoveCursor::Into { container, index } => self
                .containers
                .get(container)
                .and_then(Option::as_ref)
                .map(|c| index <= c.children.len())
                .unwrap_or(false),
        }
    }

    pub fn try_data(&self, item: ItemIdx) -> Option<LayoutDataRef<'_, W, C>> {
        match item {
            ItemIdx::Window(w_idx) => self
                .try_window(w_idx)
                .map(|w| LayoutDataRef::Window(&w.data)),
            ItemIdx::Container(c_idx) => self
                .try_container(c_idx)
                .map(|c| LayoutDataRef::Container(&c.data)),
        }
    }

    pub fn try_data_mut(&mut self, item: ItemIdx) -> Option<LayoutDataMut<'_, W, C>> {
        match item {
            ItemIdx::Window(w_idx) => self
                .try_window_mut(w_idx)
                .map(|w| LayoutDataMut::Window(&mut w.data)),
            ItemIdx::Container(c_idx) => self
                .try_container_mut(c_idx)
                .map(|c| LayoutDataMut::Container(&mut c.data)),
        }
    }

    pub fn try_window_data(&self, w_idx: usize) -> Option<&W> {
        self.try_window(w_idx).map(|w| &w.data)
    }

    pub fn try_container_data(&self, c_idx: usize) -> Option<&C> {
        self.try_container(c_idx).map(|c| &c.data)
    }

    pub fn try_window_data_mut(&mut self, w_idx: usize) -> Option<&mut W> {
        self.try_window_mut(w_idx).map(|w| &mut w.data)
    }

    pub fn try_container_data_mut(&mut self, c_idx: usize) -> Option<&mut C> {
        self.try_container_mut(c_idx).map(|c| &mut c.data)
    }

    pub fn n_children(&self, item: ItemIdx) -> usize {
        match item {
            ItemIdx::Window(_) => 0,
            ItemIdx::Container(c_idx) => self.containers[c_idx].as_ref().unwrap().children.len(),
        }
    }

    pub fn slot_in_container(&self, item: ItemIdx) -> Option<SlotInContainer> {
        self.parent_container(item).map(|p_ctr| {
            let iip = self.index_in_parent(item).unwrap();
            let strat = self.containers[p_ctr].as_ref().unwrap().strategy;
            SlotInContainer {
                c_idx: p_ctr,
                index: iip,
                parent_strat: strat,
            }
        })
    }

    pub fn iter_descendants(&self, item: ItemIdx) -> DescendantsIter<'_, W, C, CCtor> {
        DescendantsIter::new(self, item)
    }

    pub fn window_at(&self, position: Position) -> Option<usize> {
        for (w_idx, w) in self.windows.iter().enumerate() {
            if let Some(Window { bounds, .. }) = w {
                if bounds.contains(position) {
                    return Some(w_idx);
                }
            }
        }
        None
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
                    data: self.cctor.as_mut().expect("Must set cctor!").construct(),
                    padding: self.default_padding,
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
                            data: _,
                            padding: _,
                        } = self.containers[0].take().unwrap();
                        let new_ctr = Container {
                            strategy,
                            children,
                            bounds: Default::default(),
                            inter,
                            parent: Some(0),
                            data: self.cctor.as_mut().expect("Must set cctor!").construct(),
                            padding: self.default_padding,
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
                            data: self.cctor.as_mut().expect("Must set cctor!").construct(),
                            padding: self.default_padding,
                        });
                        0
                    }
                }
            }
        }
    }
    pub fn get_content_length(&self, item: ItemIdx) -> Option<usize> {
        self.slot_in_container(item).map(
            |SlotInContainer {
                 c_idx,
                 index,
                 parent_strat,
             }| {
                let bounds = self.bounds(item);
                match parent_strat {
                    LayoutStrategy::Horizontal => bounds.content.width,
                    LayoutStrategy::Vertical => bounds.content.height,
                }
            },
        )
    }
    pub fn set_content_length(
        &mut self,
        item: ItemIdx,
        new_length: usize,
    ) -> Vec<LayoutAction<W, C>> {
        let mut out = vec![];
        info!("Setting length of {:?} to {}", item, new_length);
        if let Some(SlotInContainer {
            c_idx,
            index,
            parent_strat,
        }) = self.slot_in_container(item)
        {
            let available_length = self.ctr_available_length(c_idx);
            let new_length = new_length.min(available_length);
            let remaining_length = available_length - new_length;
            let children = &mut self.containers[c_idx].as_mut().unwrap().children;
            let total_weight_of_others: f64 = children
                .iter()
                .filter_map(
                    |&(weight, child)| {
                        if child == item {
                            None
                        } else {
                            Some(weight)
                        }
                    },
                )
                .sum();
            for (weight, child) in children {
                if *child == item {
                    *weight = new_length as f64;
                } else {
                    *weight = (*weight / total_weight_of_others) * (remaining_length as f64);
                }
            }
            self.layout(ItemIdx::Container(c_idx), &mut out);
        }
        out
    }
    pub fn ctr_available_length(&self, c_idx: usize) -> usize {
        let strat = self.containers[c_idx].as_ref().unwrap().strategy;
        let AreaSize { height, width } = self.ctr_available_area(c_idx);
        match strat {
            LayoutStrategy::Horizontal => width,
            LayoutStrategy::Vertical => height,
        }
    }
    pub fn ctr_available_area(&self, c_idx: usize) -> AreaSize {
        let ctr = self.containers[c_idx].as_ref().unwrap();
        let strat = ctr.strategy;
        let ctr_bounds = ctr.bounds;
        let total_inter = ctr.inter * (ctr.children.len().saturating_sub(1));
        let available_area = match strat {
            LayoutStrategy::Vertical => AreaSize {
                height: ctr_bounds
                    .content
                    .height
                    .saturating_sub(total_inter + 2 * ctr.padding),
                width: ctr_bounds.content.width.saturating_sub(2 * ctr.padding),
            },
            LayoutStrategy::Horizontal => AreaSize {
                height: ctr_bounds.content.height.saturating_sub(2 * ctr.padding),
                width: ctr_bounds
                    .content
                    .width
                    .saturating_sub(total_inter + 2 * ctr.padding),
            },
        };
        available_area
    }
    fn layout(&mut self, item: ItemIdx, out: &mut Vec<LayoutAction<W, C>>) {
        let c_idx = match item {
            ItemIdx::Container(idx) => idx,
            ItemIdx::Window(_) => return,
        };
        let available_area = self.ctr_available_area(c_idx);
        let ctr = self.containers[c_idx].as_ref().unwrap();
        let strat = ctr.strategy;
        let ctr_bounds = ctr.bounds;
        let begin = match strat {
            LayoutStrategy::Vertical => ctr_bounds.position.y + ctr.padding,
            LayoutStrategy::Horizontal => ctr_bounds.position.x + ctr.padding,
        };
        let total_weight: f64 = ctr.children.iter().map(|(weight, _)| weight).sum();
        let mut next_window_origin = ctr_bounds.position;
        next_window_origin.x += ctr.padding;
        next_window_origin.y += ctr.padding;
        let inter = ctr.inter;
        let padding = ctr.padding;
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
                        height: new_y - next_window_origin.y,
                        width: available_area.width,
                    }
                }
                LayoutStrategy::Horizontal => {
                    let new_x = (begin as f64 + cumsum * available_area.width as f64) as usize;
                    AreaSize {
                        height: available_area.height,
                        width: new_x - next_window_origin.x,
                    }
                }
            };
            let new_bounds = WindowBounds {
                content,
                position: next_window_origin,
            };
            next_window_origin = match strat {
                LayoutStrategy::Vertical => Position {
                    x: next_window_origin.x,
                    y: next_window_origin.y + content.height + inter,
                },
                LayoutStrategy::Horizontal => Position {
                    y: next_window_origin.y,
                    x: next_window_origin.x + content.width + inter,
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
                }
                ItemIdx::Container(c_idx) => {
                    self.containers[c_idx].as_mut().unwrap().bounds = bounds;
                    self.layout(idx, out);
                }
            }
            out.push(LayoutAction::NewBounds { idx, bounds });
        }
    }
    // Doesn't layout. Returns container modified.
    fn insert(&mut self, from: ItemIdx, to: MoveCursor) -> usize {
        let to_ctr = match to {
            MoveCursor::Split { item, direction } => match direction {
                Direction::Up => self.split(from, item, LayoutStrategy::Vertical, true),
                Direction::Down => self.split(from, item, LayoutStrategy::Vertical, false),
                Direction::Left => self.split(from, item, LayoutStrategy::Horizontal, true),
                Direction::Right => self.split(from, item, LayoutStrategy::Horizontal, false),
            },
            MoveCursor::Into {
                container: c_idx,
                index,
            } => {
                let container = self.containers[c_idx].as_mut().unwrap();
                let n_children = container.children.len();
                let avg_weight = if n_children > 0 {
                    container
                        .children
                        .iter()
                        .map(|(weight, _child)| *weight)
                        .sum::<f64>()
                        / (n_children as f64)
                } else {
                    1.0
                };
                container.children.insert(index, (avg_weight, from));
                c_idx
            }
        };
        match from {
            ItemIdx::Container(idx) => self.containers[idx].as_mut().unwrap().parent = Some(to_ctr),
            ItemIdx::Window(idx) => self.windows[idx].as_mut().unwrap().parent = Some(to_ctr),
        };
        // If we created a split, we need to update the old
        // item's parent to be the new container.
        // [XXX] why isn't this handled by `Layout::split` above? I must have had a reason.
        if let MoveCursor::Split { item, .. } = to {
            match item {
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
    /// Returns None for one past end
    pub fn item_from_child_location(&self, cl: ChildLocation) -> Option<ItemIdx> {
        let ChildLocation { container, index } = cl;
        let ctr = self.containers[container].as_ref().unwrap();
        assert!(index <= ctr.children.len());
        ctr.children.get(index).map(|(_, idx)| *idx)
    }
    /// Returns None for the root
    pub fn child_location(&self, item: ItemIdx) -> Option<ChildLocation> {
        self.parent_container(item).map(|container| {
            let index = self.containers[container]
                .as_ref()
                .unwrap()
                .children
                .iter()
                .enumerate()
                .filter_map(|(i, (_, child))| (*child == item).then(|| i))
                .unwrap_one();
            ChildLocation { container, index }
        })
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct ChildLocation {
    pub container: usize,
    pub index: usize,
}

impl<W, C, CCtor> Layout<W, C, CCtor>
where
    W: Serialize + for<'d> Deserialize<'d>,
    C: Serialize + for<'d> Deserialize<'d>,
    CCtor: Constructor<Item = C>,
{
    pub fn equalize_container_children(&mut self, c_idx: usize) -> Vec<LayoutAction<W, C>> {
        for (weight, _child) in &mut self.containers[c_idx].as_mut().unwrap().children {
            *weight = 1.0;
        }
        let mut out = vec![];
        self.layout(ItemIdx::Container(c_idx), &mut out);
        out
    }
    pub fn root_bounds(&self) -> WindowBounds {
        self.root_bounds
    }
    pub fn index_in_parent(&self, item: ItemIdx) -> Option<usize> {
        self.parent_container(item).map(|parent| {
            let parent_ctr = self.containers[parent].as_ref().unwrap();
            parent_ctr
                .children
                .iter()
                .position(|(_, child_idx)| *child_idx == item)
                .unwrap()
        })
    }
    pub fn cursor_before(&self, point: ItemIdx) -> MoveCursor {
        match self.parent_container(point) {
            Some(parent) => MoveCursor::Into {
                container: parent,
                index: self.index_in_parent(point).unwrap(),
            },
            None => MoveCursor::Split {
                item: ItemIdx::Container(0),
                direction: match self.containers[0].as_ref().unwrap().strategy {
                    LayoutStrategy::Horizontal => Direction::Left,
                    LayoutStrategy::Vertical => Direction::Up,
                },
            },
        }
    }
    pub fn new(bounds: WindowBounds, mut cctor: CCtor, default_padding: usize) -> Self {
        let root_data = cctor.construct();
        let mut this = Self {
            windows: Default::default(),
            containers: Default::default(),
            root_bounds: bounds,
            cctor: Some(cctor),
            default_padding,
        };
        this.containers.push(Some(Container {
            bounds,
            children: Default::default(),
            inter: 0,
            parent: None,
            strategy: LayoutStrategy::Horizontal,
            data: root_data,
            padding: default_padding,
        }));
        this
    }
    pub fn windows<'a>(&'a self) -> impl Iterator<Item = &'a Window<W>> {
        self.windows.iter().filter_map(Option::as_ref)
    }
    pub fn resize(&mut self, bounds: WindowBounds) -> Vec<LayoutAction<W, C>>
    where
        C: std::fmt::Debug,
        W: std::fmt::Debug,
    {
        info!("Resizing in wb: {:?}", bounds);
        self.containers[0].as_mut().unwrap().bounds = bounds;
        self.root_bounds = bounds;
        let mut out = vec![];
        self.layout(ItemIdx::Container(0), &mut out);
        out.push(LayoutAction::NewBounds {
            idx: ItemIdx::Container(0),
            bounds,
        });
        out
    }
    pub fn parent_container(&self, item: ItemIdx) -> Option<usize> {
        match item {
            ItemIdx::Window(idx) => Some(self.windows[idx].as_ref().unwrap().parent?),
            ItemIdx::Container(idx) => self.containers[idx].as_ref().unwrap().parent,
        }
    }
    fn topo_next_recursive(&self, item: ItemIdx) -> Option<ItemIdx> {
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
    /// Returns the container that remains (i.e., the GP) if a fuse was done
    fn fuse_if_necessary(
        &mut self,
        parent: usize,
        result: &mut Vec<LayoutAction<W, C>>,
    ) -> Option<usize> {
        if let Some(grandparent) = self.parent_container(ItemIdx::Container(parent)) {
            if self.containers[parent].as_ref().unwrap().children.len() == 1 {
                let parent_ctr = self.containers[parent].take().unwrap();
                let gp_ctr = self.containers[grandparent].as_ref().unwrap();
                result.push(LayoutAction::ItemDestroyed {
                    item: ItemAndData::Container(parent, parent_ctr.data),
                });
                let index_in_gp = gp_ctr
                    .children
                    .iter()
                    .position(|(_, child_idx)| *child_idx == ItemIdx::Container(parent))
                    .unwrap();
                let child = parent_ctr.children[0].1;

                let gp_ctr = self.containers[grandparent].as_mut().unwrap();
                gp_ctr.children[index_in_gp].1 = child;
                *(match child {
                    ItemIdx::Container(idx) => &mut self.containers[idx].as_mut().unwrap().parent,
                    ItemIdx::Window(idx) => &mut self.windows[idx].as_mut().unwrap().parent,
                }) = Some(grandparent);
                return Some(grandparent);
            }
        }
        None
    }
    pub fn destroy(&mut self, item: ItemIdx) -> Vec<LayoutAction<W, C>> {
        let to_destroy = self.iter_descendants(item).collect::<Vec<_>>();
        let parent = self.parent_container(item);
        let index_in_parent = self.index_in_parent(item);
        // Remove items from the layout, and take their data for passing back up
        let mut result = to_destroy
            .iter()
            .copied()
            .map(|descendant| {
                let item = match descendant {
                    ItemIdx::Container(c_idx) => {
                        ItemAndData::Container(c_idx, self.containers[c_idx].take().unwrap().data)
                    }
                    ItemIdx::Window(w_idx) => {
                        ItemAndData::Window(w_idx, self.windows[w_idx].take().unwrap().data)
                    }
                };
                LayoutAction::ItemDestroyed { item }
            })
            .collect::<Vec<_>>();
        match parent {
            None => {
                // we destroyed the root, but there must always be a root.
                self.containers[0] = Some(Container {
                    strategy: LayoutStrategy::Horizontal,
                    children: vec![],
                    parent: None,
                    inter: Default::default(),
                    bounds: self.root_bounds,
                    data: self.cctor.as_mut().expect("Must set cctor!").construct(),
                    padding: self.default_padding,
                });
            }
            Some(mut parent) => {
                let index_in_parent = index_in_parent.unwrap();
                let parent_ctr = self.containers[parent].as_mut().unwrap();
                parent_ctr.children.remove(index_in_parent);
                // fuse if necessary
                if let Some(grandparent) = self.fuse_if_necessary(parent, &mut result) {
                    parent = grandparent;
                }
                self.layout(ItemIdx::Container(parent), &mut result);
            }
        };
        result
    }
    pub fn is_ancestor(&self, ancestor: ItemIdx, mut descendant: ItemIdx) -> bool {
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
            descendant = ItemIdx::Container(parent);
        }
        false
    }
    pub fn alloc_window(&mut self, data: W) -> usize {
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
            data,
        });
        next_idx
    }
    pub fn r#move(&mut self, from: ItemIdx, to: MoveCursor) -> Vec<LayoutAction<W, C>> {
        if self.is_ancestor(from, to.item()) {
            panic!()
        }
        // we know `from` is not the root, because of the ancestry check above.
        // So the unwrap is safe.
        let mut from_parent = self.parent_container(from);
        let mut result = vec![];
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
        let to = match to {
            MoveCursor::Into { container, index }
                if from_parent.is_some()
                    && container == from_parent.unwrap()
                    && index >= idx_in_parent.unwrap() =>
            {
                MoveCursor::Into {
                    container,
                    index: index.saturating_sub(1),
                }
            }
            _ => to,
        };
        let insert_modified = self.insert(from, to);
        if matches!(to, MoveCursor::Split { .. }) {
            // This will have created a new container; notify the client code.
            result.push(LayoutAction::NewBounds {
                idx: ItemIdx::Container(insert_modified),
                bounds: self.containers[insert_modified].as_ref().unwrap().bounds,
            });
        }
        if let Some(fp) = from_parent {
            if let Some(gp) = self.fuse_if_necessary(fp, &mut result) {
                from_parent = Some(gp);
            }
        }
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
        point: Option<Position>,
    ) -> Option<ItemIdx> {
        if from == ItemIdx::Container(0) {
            None
        } else {
            self.navigate2(self.child_location(from).unwrap(), dir, point, false, true)
                .map(|cl| self.item_from_child_location(cl).unwrap())
        }
    }
    pub fn navigate2(
        &self,
        ChildLocation {
            container: parent_container_idx,
            index: mut index_in_parent,
        }: ChildLocation,
        dir: Direction,
        point: Option<Position>,
        // whether we are navigating among the space _between_ items
        // (e.g., for a cursor). If true, the returned location is allowed to
        // have index equal to the container's length.
        between_items: bool,
        // whether to descend into items from the same container
        may_descend: bool,
    ) -> Option<ChildLocation> {
        let orig_parent_container_idx = parent_container_idx;
        //        let mut ancestor = None;
        // let mut cur = from;
        let parent_container = self.containers[parent_container_idx].as_ref().unwrap();
        assert!(index_in_parent <= parent_container.children.len());
        assert!(between_items || index_in_parent < parent_container.children.len());
        let point = point.unwrap_or_else(|| {
            parent_container
                .children
                .get(index_in_parent)
                .map(|&(_weight, child)| self.bounds(child).position)
                .unwrap_or_else(|| match parent_container.strategy {
                    LayoutStrategy::Horizontal => Position {
                        x: parent_container.bounds.position.x
                            + parent_container.bounds.content.width,
                        y: parent_container.bounds.position.y,
                    },
                    LayoutStrategy::Vertical => Position {
                        x: parent_container.bounds.position.x,
                        y: parent_container.bounds.position.y
                            + parent_container.bounds.content.height,
                    },
                })
        });

        let mut parent_container_idx = Some(parent_container_idx);
        let mut ancestor = None;

        while let Some(parent_ctr_idx) = parent_container_idx {
            let parent_ctr = self.containers[parent_ctr_idx].as_ref().unwrap();
            let strat = parent_ctr.strategy;
            let can_go_back = ((dir == Direction::Left && strat == LayoutStrategy::Horizontal)
                || (dir == Direction::Up && strat == LayoutStrategy::Vertical))
                && index_in_parent > 0;
            if can_go_back {
                ancestor = Some(ChildLocation {
                    container: parent_ctr_idx,
                    index: index_in_parent - 1,
                });
                break;
            }
            let can_go_fwd = ((dir == Direction::Right && strat == LayoutStrategy::Horizontal)
                || (dir == Direction::Down && strat == LayoutStrategy::Vertical))
                && index_in_parent < parent_ctr.children.len() + (between_items as usize) - 1;
            if can_go_fwd {
                ancestor = Some(ChildLocation {
                    container: parent_ctr_idx,
                    index: index_in_parent + 1,
                });
                break;
            }
            parent_container_idx = self.parent_container(ItemIdx::Container(parent_ctr_idx));
            if let Some(next) = parent_container_idx {
                index_in_parent = self.containers[next]
                    .as_ref()
                    .unwrap()
                    .children
                    .iter()
                    .enumerate()
                    .find(|(_i, (_weight, child))| *child == ItemIdx::Container(parent_ctr_idx))
                    .map(|(i, (_weight, _child))| i)
                    .unwrap();
            }
        }
        let move_horizontal = dir == Direction::Left || dir == Direction::Right;
        let move_to_first = dir == Direction::Right || dir == Direction::Down;
        if may_descend
            || !matches!(
                ancestor,
                Some(ChildLocation {
                    container,
                    ..
                }) if container == orig_parent_container_idx
            )
        {
            ancestor.map(
                |ChildLocation {
                     mut container,
                     mut index,
                 }| {
                    while let Some(ItemIdx::Container(c_idx)) = self.containers[container]
                        .as_ref()
                        .unwrap()
                        .children
                        .get(index)
                        .map(|(_w, idx)| *idx)
                    {
                        let ctr = self.containers[c_idx].as_ref().unwrap();
                        container = c_idx;
                        index = if move_horizontal == (ctr.strategy == LayoutStrategy::Horizontal) {
                            if move_to_first {
                                0
                            } else if between_items {
                                ctr.children.len()
                            } else {
                                ctr.children.len() - 1
                            }
                        } else {
                            let Position { x, y } = point;
                            let seek_coord = if move_horizontal { y } else { x };
                            let idx = ctr
                                .children
                                .iter()
                                .position(|&(_weight, child)| {
                                    let bounds = self.bounds(child);
                                    let (child_lb, child_ub) = if move_horizontal {
                                        (
                                            bounds.position.y,
                                            bounds.position.y + bounds.content.height,
                                        )
                                    } else {
                                        (
                                            bounds.position.x,
                                            bounds.position.x + bounds.content.width,
                                        )
                                    };
                                    child_lb <= seek_coord && seek_coord < child_ub
                                })
                                .unwrap_or(0);
                            idx
                        }
                    }
                    ChildLocation { container, index }
                },
            )
        } else {
            ancestor
        }
    }
    pub fn children(&self, container: usize) -> &[(f64, ItemIdx)] {
        &self.containers[container].as_ref().unwrap().children
    }
    pub fn nearest_container(&self, item: ItemIdx) -> usize {
        match item {
            ItemIdx::Container(c_idx) => c_idx,
            ItemIdx::Window(_) => {
                self.nearest_container(ItemIdx::Container(self.parent_container(item).unwrap()))
            }
        }
    }
    pub fn try_bounds(&self, item: ItemIdx) -> Option<WindowBounds> {
        match item {
            ItemIdx::Container(c_idx) => self.containers[c_idx].as_ref().map(|c| c.bounds),
            ItemIdx::Window(w_idx) => self.windows[w_idx].as_ref().map(|w| w.bounds),
        }
    }
    pub fn bounds(&self, item: ItemIdx) -> WindowBounds {
        self.try_bounds(item).unwrap()
    }
    /// Get the bounds of the gap before element `index` in the container.
    /// `index` may be equal to the container's length, in which case
    /// this function returns the gap at the end.
    pub fn inter_bounds(&self, container: usize, index: usize) -> WindowBounds {
        let container = self.containers[container].as_ref().unwrap();
        assert!(index <= container.children.len());
        if container.children.is_empty() {
            return container.bounds;
        }
        if index == container.children.len() {
            return match container.strategy {
                LayoutStrategy::Horizontal => WindowBounds {
                    content: AreaSize {
                        height: container.bounds.content.height,
                        width: 0,
                    },
                    position: Position {
                        x: container.bounds.position.x + container.bounds.content.width,
                        y: container.bounds.position.y,
                    },
                },
                LayoutStrategy::Vertical => WindowBounds {
                    content: AreaSize {
                        height: 0,
                        width: container.bounds.content.width,
                    },
                    position: Position {
                        x: container.bounds.position.x,
                        y: container.bounds.position.y + container.bounds.content.height,
                    },
                },
            };
        }
        let total_inter = container.inter * container.children.len().saturating_sub(1);
        let total_weight: f64 = container
            .children
            .iter()
            .map(|(weight, _child)| weight)
            .sum();
        let main_dim_bound = match container.strategy {
            LayoutStrategy::Horizontal => container.bounds.content.width,
            LayoutStrategy::Vertical => container.bounds.content.height,
        };
        // XXX - padding should be configurable on all four sides
        let content_size = main_dim_bound - total_inter - 2 * container.padding;
        // The distance from the
        // beginning of the container
        // to the end of the `i-1`th child
        // (or 0, when i is 0)
        let mut cum_distance = container.padding as f64;
        for i in 0..index {
            let normalized = container.children[i].0 / total_weight;
            cum_distance += normalized * (content_size as f64) + container.inter as f64;
        }
        match container.strategy {
            LayoutStrategy::Horizontal => WindowBounds {
                content: AreaSize {
                    height: container.bounds.content.height - 2 * container.padding,
                    width: container.inter,
                },
                position: Position {
                    x: container.bounds.position.x + cum_distance as usize,
                    y: container.bounds.position.y + container.padding,
                },
            },
            LayoutStrategy::Vertical => WindowBounds {
                content: AreaSize {
                    height: container.inter,
                    width: container.bounds.content.width - 2 * container.padding,
                },
                position: Position {
                    x: container.bounds.position.x + container.padding,
                    y: container.bounds.position.y + cum_distance as usize,
                },
            },
        }
    }

    pub fn exists(&self, idx: ItemIdx) -> bool {
        match idx {
            ItemIdx::Container(c_idx) => self
                .containers
                .get(c_idx)
                .and_then(|maybe_ctr| maybe_ctr.as_ref())
                .is_some(),
            ItemIdx::Window(w_idx) => self
                .windows
                .get(w_idx)
                .and_then(|maybe_w| maybe_w.as_ref())
                .is_some(),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub enum LayoutAction<W, C> {
    /// An item has moved or been created.
    NewBounds { idx: ItemIdx, bounds: WindowBounds },
    /// An item has been destroyed.
    ItemDestroyed { item: ItemAndData<W, C> },
    /// A window still exists, but is no longer visible.
    ItemHidden { idx: ItemIdx },
}
