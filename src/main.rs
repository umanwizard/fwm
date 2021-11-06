use gdk::EventKey;
use gdk::ModifierType;
use gdk::WindowExt;
use gio::prelude::*;
use gtk::prelude::*;
use gtk::{Allocation, Application, ApplicationWindow, Button, DrawingArea};

use ::fwm::AreaSize;
use ::fwm::Direction;
use ::fwm::ItemIdx;
use ::fwm::Layout;
use ::fwm::LayoutAction;
use ::fwm::MoveAction;
use ::fwm::Position;
use ::fwm::WindowBounds;

use rand::distributions::{Distribution, Standard};
use rand::thread_rng;
use rand::Rng;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

const CURSOR_LINE_WIDTH: f64 = 40.0;

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
    pub cursor: ItemIdx,
}

impl WmState {
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
            LayoutAction::WindowHidden { idx } => unimplemented!(),
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

fn change_cursor(old: WindowBounds, new: WindowBounds, window: gdk::Window) {
    if old == new {
        return;
    }
    let (mut r1, mut r2) = (wb_to_r(old), wb_to_r(new));
    r1.x = r1.x.saturating_sub((CURSOR_LINE_WIDTH / 2.0) as i32);
    r1.y = r1.y.saturating_sub((CURSOR_LINE_WIDTH / 2.0) as i32);
    r2.x = r2.x.saturating_sub((CURSOR_LINE_WIDTH / 2.0) as i32);
    r2.y = r2.y.saturating_sub((CURSOR_LINE_WIDTH / 2.0) as i32);
    r1.width += CURSOR_LINE_WIDTH as i32;
    r1.height += CURSOR_LINE_WIDTH as i32;
    r2.width += CURSOR_LINE_WIDTH as i32;
    r2.height += CURSOR_LINE_WIDTH as i32;
    window.invalidate_rect(Some(&r1), true);
    window.invalidate_rect(Some(&r2), true);
}

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
            cursor: ItemIdx::Container(0),
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
                        let window = borrow.layout.alloc_window();
                        borrow
                            .windows
                            .insert(window, (thread_rng().gen(), Default::default()));
                        let container = borrow.layout.nearest_container(borrow.cursor);
                        let n_ctr_children = borrow.layout.children(container).len();
                        let cursor = borrow.cursor;
                        let old_cursor_bounds = borrow.layout.bounds(cursor);
                        let actions = borrow.layout.move_(
                            ItemIdx::Window(window),
                            ItemIdx::Container(container),
                            MoveAction::ToIndex(n_ctr_children),
                        );
                        for a in actions.iter().copied() {
                            borrow.update_for_action(a, w.get_window().as_ref());
                        }
                        borrow.cursor = ItemIdx::Window(window);
                        let new_cursor_bounds = borrow.layout.bounds(borrow.cursor);
                        if let Some(window) = w.get_window() {
                            if old_cursor_bounds != new_cursor_bounds {
                                change_cursor(old_cursor_bounds, new_cursor_bounds, window);
                            }
                        }
                    } else if uchar == Some('v') {
                        let window = borrow.layout.alloc_window();
                        borrow
                            .windows
                            .insert(window, (thread_rng().gen(), Default::default()));
                        let cursor = borrow.cursor;
                        let old_cursor_bounds = borrow.layout.bounds(cursor);
                        let actions = borrow.layout.move_(
                            ItemIdx::Window(window),
                            cursor,
                            MoveAction::Split(Direction::Down),
                        );
                        for a in actions.iter().copied() {
                            borrow.update_for_action(a, w.get_window().as_ref());
                        }
                        borrow.cursor = ItemIdx::Window(window);
                        let new_cursor_bounds = borrow.layout.bounds(borrow.cursor);
                        if let Some(window) = w.get_window() {
                            if old_cursor_bounds != new_cursor_bounds {
                                change_cursor(old_cursor_bounds, new_cursor_bounds, window);
                            }
                        }
                    } else if uchar == Some('m') {
                        let window = borrow.layout.alloc_window();
                        borrow
                            .windows
                            .insert(window, (thread_rng().gen(), Default::default()));
                        let cursor = borrow.cursor;
                        let old_cursor_bounds = borrow.layout.bounds(cursor);
                        let actions = borrow.layout.move_(
                            ItemIdx::Window(window),
                            cursor,
                            MoveAction::Split(Direction::Right),
                        );
                        for a in actions.iter().copied() {
                            borrow.update_for_action(a, w.get_window().as_ref());
                        }
                        borrow.cursor = ItemIdx::Window(window);
                        let new_cursor_bounds = borrow.layout.bounds(borrow.cursor);
                        if let Some(window) = w.get_window() {
                            if old_cursor_bounds != new_cursor_bounds {
                                change_cursor(old_cursor_bounds, new_cursor_bounds, window);
                            }
                        }
                    } else if matches!(uchar, Some('h'| 'j' | 'k' | 'l')) {
                        let cursor = borrow.cursor;
                        let direction = match uchar.unwrap() {
                            'h' => Direction::Left,
                            'k' => Direction::Up,
                            'l' => Direction::Right,
                            'j' => Direction::Down,
                            _ => unreachable!(),
                        };
                        if let Some(new_cursor) = borrow.layout.navigate(cursor, direction, None) {
                            borrow.cursor = new_cursor;
                            let old_cursor_bounds = borrow.layout.bounds(cursor);
                            let new_cursor_bounds = borrow.layout.bounds(new_cursor);
                            if let Some(window) = w.get_window() {
                                change_cursor(old_cursor_bounds, new_cursor_bounds, window);
                            }
                        }
                    } else if uchar == Some('"') {
                        let cursor = borrow.cursor;
                        let old_cursor_bounds = borrow.layout.bounds(cursor);
                        let new_cursor = borrow.layout.topological_next(cursor);
                        let actions = borrow.layout.destroy(cursor);
                        let new_cursor =
                            new_cursor.unwrap_or_else(|| borrow.layout.topological_last());
                        let new_cursor_bounds = borrow.layout.bounds(new_cursor);
                        borrow.cursor = new_cursor;
                        for a in actions.iter().copied() {
                            borrow.update_for_action(a, w.get_window().as_ref());
                        }
                        if let Some(window) = w.get_window() {
                            change_cursor(old_cursor_bounds, new_cursor_bounds, window);
                        }
                    } else if uchar == Some('a') {
                        let cursor = borrow.cursor;
                        if let Some(parent) = borrow.layout.parent_container(cursor) {
                            let new_cursor = ItemIdx::Container(parent);
                            borrow.cursor = new_cursor;
                            let old_cursor_bounds = borrow.layout.bounds(cursor);
                            let new_cursor_bounds = borrow.layout.bounds(new_cursor);
                            if let Some(window) = w.get_window() {
                                change_cursor(old_cursor_bounds, new_cursor_bounds, window);
                            }
                        }
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
                cr.set_line_width(CURSOR_LINE_WIDTH);
                let cursor = borrow.cursor;
                let WindowBounds {
                    content: AreaSize { height, width },
                    position: Position { x, y },
                } = borrow.layout.bounds(cursor);
                cr.rectangle(x as f64, y as f64, width as f64, height as f64);
                cr.stroke();
                Inhibit(true)
            }
        });
        window.add(&da);

        window.show_all();
    });

    application.run(&[]);
}
