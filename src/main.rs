use gdk::ModifierType;
use gdk::WindowExt;
use gio::prelude::*;
use gtk::prelude::*;
use gtk::{Allocation, Application, ApplicationWindow, DrawingArea};

use ::fwm::AreaSize;
use ::fwm::Direction;
use ::fwm::ItemIdx;
use ::fwm::Layout;
use ::fwm::LayoutAction;
use ::fwm::MoveCursor;
use ::fwm::Position;
use ::fwm::WindowBounds;

use rand::distributions::{Distribution, Standard};
use rand::thread_rng;
use rand::Rng;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

const POINT_LINE_WIDTH: f64 = 40.0;

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Distribution<Rgb> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Rgb {
        let (r, g, b) = rng.gen();
        Rgb { r, g, b }
    }
}

struct WmState {
    pub windows: HashMap<usize, (Rgb, WindowBounds)>,
    pub layout: Layout,
    pub point: ItemIdx,
    pub cursor: Option<MoveCursor>,
}

impl WmState {
    /// Run a closure, and then invalidate the proper rectangles if it
    /// caused the bounds of the point or cursor to change.
    pub fn do_and_recompute<F>(&mut self, closure: F, window: Option<&gdk::Window>)
    where
        F: FnOnce(&mut Self),
    {
        let old_point_bounds = self.layout.bounds(self.point);
        let old_cursor_bounds = self.cursor.map(|cursor| self.layout.bounds(cursor.item()));
        closure(self);
        let new_point_bounds = self.layout.bounds(self.point);
        let new_cursor_bounds = self.cursor.map(|cursor| self.layout.bounds(cursor.item()));

        if let Some(window) = window {
            change_point(Some(old_point_bounds), Some(new_point_bounds), window);
            change_cursor(old_cursor_bounds, new_cursor_bounds, window);
        }
    }
    pub fn update_for_action(&mut self, action: LayoutAction, window: Option<&gdk::Window>) {
        match action {
            LayoutAction::NewWindowBounds { idx, bounds } => {
                self.windows.get_mut(&idx).unwrap().1 = bounds;
                let WindowBounds {
                    position: Position { x, y },
                    content: AreaSize { height, width },
                } = bounds;
                if let Some(window) = window {
                    window.invalidate_rect(
                        Some(&gdk::Rectangle {
                            x: x as i32,
                            y: y as i32,
                            height: height as i32,
                            width: width as i32,
                        }),
                        true,
                    );
                }
            }
            LayoutAction::WindowDestroyed { idx } => {
                let (_rgb, old_bounds) = self.windows.remove(&idx).unwrap();
                if let Some(window) = window {
                    window.invalidate_rect(Some(&wb_to_r(old_bounds)), true);
                }
            }
            LayoutAction::WindowHidden { idx: _ } => unimplemented!(),
        }
    }
}

fn a_to_wb(a: Allocation) -> WindowBounds {
    let Allocation {
        x,
        y,
        width,
        height,
    } = a;
    assert!(x >= 0);
    assert!(y >= 0);
    assert!(width >= 0);
    assert!(height >= 0);
    let (x, y, width, height) = (x as usize, y as usize, width as usize, height as usize);

    WindowBounds {
        content: AreaSize { height, width },
        position: Position { x, y },
    }
}

fn wb_to_r(wb: WindowBounds) -> gdk::Rectangle {
    let WindowBounds {
        content: AreaSize { height, width },
        position: Position { x, y },
    } = wb;
    gdk::Rectangle {
        x: x as i32,
        y: y as i32,
        height: height as i32,
        width: width as i32,
    }
}

fn change_point(old: Option<WindowBounds>, new: Option<WindowBounds>, window: &gdk::Window) {
    if old == new {
        return;
    }
    let (r1, r2) = (old.map(|old| wb_to_r(old)), new.map(|new| wb_to_r(new)));
    if let Some(mut r1) = r1 {
        r1.x = r1.x.saturating_sub((POINT_LINE_WIDTH / 2.0) as i32);
        r1.y = r1.y.saturating_sub((POINT_LINE_WIDTH / 2.0) as i32);
        r1.width += POINT_LINE_WIDTH as i32;
        r1.height += POINT_LINE_WIDTH as i32;
        window.invalidate_rect(Some(&r1), true);
    }
    if let Some(mut r2) = r2 {
        r2.x = r2.x.saturating_sub((POINT_LINE_WIDTH / 2.0) as i32);
        r2.y = r2.y.saturating_sub((POINT_LINE_WIDTH / 2.0) as i32);
        r2.width += POINT_LINE_WIDTH as i32;
        r2.height += POINT_LINE_WIDTH as i32;
        window.invalidate_rect(Some(&r2), true);
    }
}

// TODO - change cursor should do something different.
use change_point as change_cursor;

fn main() {
    let application =
        Application::new(Some("com.github.gtk-rs.examples.basic"), Default::default())
            .expect("failed to initialize GTK application");

    application.connect_activate(|app| {
        let window = ApplicationWindow::new(app);
        window.set_title("First GTK Program");

        let state = Rc::new(RefCell::new(WmState {
            windows: Default::default(),
            layout: Layout::new_in_bounds(Default::default()),
            point: ItemIdx::Container(0),
            cursor: None,
        }));
        window.connect_key_press_event({
            let state = state.clone();
            move |w, event| {
                let uchar = event.get_keyval().to_unicode();
                let mut borrow = state.borrow_mut();
                println!("{:?} {:?}", event.get_state(), uchar);
                let state = event.get_state();
                let ctrl = state.contains(ModifierType::CONTROL_MASK);
                //let shift = state.contains(ModifierType::SHIFT_MASK);
                if ctrl {
                    if uchar == Some('\r') {
                        borrow.do_and_recompute(
                            |wm| {
                                let window = wm.layout.alloc_window();
                                wm.windows
                                    .insert(window, (thread_rng().gen(), Default::default()));
                                let container = wm.layout.nearest_container(wm.point);
                                let n_ctr_children = wm.layout.children(container).len();
                                let actions = wm.layout.r#move(
                                    ItemIdx::Window(window),
                                    MoveCursor::Into {
                                        container,
                                        index: n_ctr_children,
                                    },
                                );
                                for a in actions.iter().copied() {
                                    wm.update_for_action(a, w.get_window().as_ref());
                                }
                                wm.point = ItemIdx::Window(window);
                            },
                            w.get_window().as_ref(),
                        );
                    } else if uchar == Some('v') {
                        borrow.do_and_recompute(
                            |wm| {
                                let window = wm.layout.alloc_window();
                                wm.windows
                                    .insert(window, (thread_rng().gen(), Default::default()));
                                let point = wm.point;
                                let actions = wm.layout.r#move(
                                    ItemIdx::Window(window),
                                    MoveCursor::Split {
                                        item: point,
                                        direction: Direction::Down,
                                    },
                                );
                                for a in actions.iter().copied() {
                                    wm.update_for_action(a, w.get_window().as_ref());
                                }
                                wm.point = ItemIdx::Window(window);
                            },
                            w.get_window().as_ref(),
                        );
                    } else if uchar == Some('m') {
                        borrow.do_and_recompute(
                            |wm| {
                                let window = wm.layout.alloc_window();
                                wm.windows
                                    .insert(window, (thread_rng().gen(), Default::default()));
                                let point = wm.point;
                                let actions = wm.layout.r#move(
                                    ItemIdx::Window(window),
                                    MoveCursor::Split {
                                        item: point,
                                        direction: Direction::Right,
                                    },
                                );
                                for a in actions.iter().copied() {
                                    wm.update_for_action(a, w.get_window().as_ref());
                                }
                                wm.point = ItemIdx::Window(window);
                            },
                            w.get_window().as_ref(),
                        );
                    } else if matches!(uchar, Some('h' | 'j' | 'k' | 'l')) {
                        borrow.do_and_recompute(
                            |wm| {
                                let point = wm.point;
                                let direction = match uchar.unwrap() {
                                    'h' => Direction::Left,
                                    'k' => Direction::Up,
                                    'l' => Direction::Right,
                                    'j' => Direction::Down,
                                    _ => unreachable!(),
                                };
                                if let Some(new_point) = wm.layout.navigate(point, direction, None)
                                {
                                    wm.point = new_point;
                                }
                            },
                            w.get_window().as_ref(),
                        );
                    } else if matches!(uchar, Some('H' | 'J' | 'K' | 'L')) {
                        borrow.do_and_recompute(
                            |wm| {
				let cursor = wm.cursor.unwrap_or_else(|| wm.layout.cursor_before(wm.point));
				todo!()
                            },
                            w.get_window().as_ref(),
                        );
                    } else if uchar == Some('"') {
                        borrow.do_and_recompute(
                            |wm| {
                                let point = wm.point;
                                let new_point = wm.layout.topological_next(point);
                                let actions = wm.layout.destroy(point);
                                let new_point =
                                    new_point.unwrap_or_else(|| wm.layout.topological_last());
                                wm.point = new_point;
                                for a in actions.iter().copied() {
                                    wm.update_for_action(a, w.get_window().as_ref());
                                }
                            },
                            w.get_window().as_ref(),
                        );
                    } else if uchar == Some('a') {
                        borrow.do_and_recompute(
                            |wm| {
                                let point = wm.point;
                                if let Some(parent) = wm.layout.parent_container(point) {
                                    let new_point = ItemIdx::Container(parent);
                                    wm.point = new_point;
                                }
                            },
                            w.get_window().as_ref(),
                        );
                    } else if uchar == Some('p') {
                        println!("{}", serde_json::to_string_pretty(&borrow.layout).unwrap());
                    }
                }
                Inhibit(true)
            }
        });
        let da = DrawingArea::new();
        da.connect_size_allocate({
            let state = state.clone();
            move |da, allocation| {
                let mut borrow = state.borrow_mut();
                let actions = borrow.layout.resize(a_to_wb(*allocation));
                for a in actions.iter().copied() {
                    borrow.update_for_action(a, da.get_window().as_ref());
                }
            }
        });
        da.connect_draw({
            let state = state.clone();
            move |_, cr| {
                let borrow = state.borrow();
                for (
                    Rgb { r, g, b },
                    WindowBounds {
                        content: AreaSize { height, width },
                        position: Position { x, y },
                    },
                ) in borrow.windows.values()
                {
                    cr.set_source_rgb(*r as f64 / 255.0, *g as f64 / 255.0, *b as f64 / 255.0);
                    cr.rectangle(*x as f64, *y as f64, *width as f64, *height as f64);
                    cr.fill();
                }
                cr.set_source_rgb(0.537, 0.812, 0.941);
                cr.set_line_width(POINT_LINE_WIDTH);
                let point = borrow.point;
                let WindowBounds {
                    content: AreaSize { height, width },
                    position: Position { x, y },
                } = borrow.layout.bounds(point);
                cr.rectangle(x as f64, y as f64, width as f64, height as f64);
                cr.stroke();
                if let Some(cursor) = borrow.cursor {
                    cr.set_source_rgb(1.0, 0.0, 0.0);
                    cr.set_line_width(POINT_LINE_WIDTH);

                    match cursor {
                        MoveCursor::Split { item, direction } => {
                            let WindowBounds {
                                content:
                                    AreaSize {
                                        mut height,
                                        mut width,
                                    },
                                position: Position { mut x, mut y },
                            } = borrow.layout.bounds(item);
                            match direction {
                                Direction::Up => {
                                    height /= 2;
                                }
                                Direction::Down => {
                                    height /= 2;
                                    y += height;
                                }
                                Direction::Left => {
                                    width /= 2;
                                }
                                Direction::Right => {
                                    width /= 2;
                                    x += width;
                                }
                            }
                            cr.rectangle(x as f64, y as f64, width as f64, height as f64);
                        }
                        MoveCursor::Into { container, index } => {
                            let WindowBounds {
                                content: AreaSize { height, width },
                                position: Position { x, y },
                            } = borrow.layout.inter_bounds(container, index);
                            cr.rectangle(x as f64, y as f64, width as f64, height as f64);
                        }
                    }
                    cr.stroke();
                }
                Inhibit(true)
            }
        });
        window.add(&da);

        window.show_all();
    });

    application.run(&[]);
}
