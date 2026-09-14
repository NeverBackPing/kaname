// Compositor Architecture:
// The compositor consists of several windows
// Each window contains a root node
// A node can either contain a panel, or a split screen

use core::fmt::{self, Display};

use crate::{
    drivers::{
        cp437,
        keyboard::{self, Key, KeyEvent, NavKey},
        vga::{self, Color},
    },
    shell,
    terminal::Terminal,
};

const MAX_WINDOWS: usize = 12;
const MAX_PANES: usize = 4 * MAX_WINDOWS; // 4 panes per window max
const MAX_NODES: usize = 7 * MAX_WINDOWS; // 7 nodes required for a 4-panel split

const MAX_INPUT: usize = 64;

#[derive(Clone, Copy)]
pub struct Rect {
    x: usize,
    y: usize,
    w: usize,
    h: usize,
}

impl Display for Rect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!(
            "{}x{} at {}x{}",
            self.w, self.h, self.x, self.y
        ))
    }
}

impl Rect {
    fn merge_with(&self, other: Rect) -> Rect {
        if self.x < other.x || self.y < other.y {
            Rect {
                x: self.x,
                y: self.y,
                w: other.w + other.x - self.x,
                h: other.h + other.y - self.y,
            }
        } else {
            Rect {
                x: other.x,
                y: other.y,
                w: self.w + self.x - other.x,
                h: self.h + self.y - other.y,
            }
        }
    }
    pub fn is_full_width(&self) -> bool {
        self.w == vga::WIDTH
    }
}

#[derive(Clone, Copy)]
struct Segment {
    axis: Axis,
    offset: usize,
    start: usize,
    end: usize,
}

impl Segment {
    fn intersect(&self, h: &Segment) -> Option<(usize, usize)> {
        if h.start <= self.offset
            && self.offset < h.end
            && self.start <= h.offset
            && h.offset < self.end
        {
            Some((self.offset, h.offset))
        } else {
            None
        }
    }
}

#[derive(Clone)]
struct Pane {
    term: Terminal,
    rect: Rect,
    readonly: bool,
    input: [u8; MAX_INPUT],
    input_len: usize,
    parent: NodeId,
}

impl Pane {
    fn init(rect: Rect, parent: NodeId) -> Self {
        let mut pane = Self {
            term: Terminal::new(rect.x, rect.y, rect.w, rect.h),
            rect,
            readonly: false,
            input: [0; MAX_INPUT],
            input_len: 0,
            parent,
        };
        pane.prompt();
        pane
    }

    fn prompt(&mut self) {
        self.term.put_raw(b'>', Color::Red, Color::White);
        self.term.put_raw(b' ', Color::Red, Color::White);
        self.term.flush();
    }

    fn handle_key(&mut self, event: KeyEvent) {
        match event.key {
            Key::Nav(NavKey::PageUp) => self.term.scroll_up(),
            Key::Nav(NavKey::PageDown) => self.term.scroll_down(),
            _ => {}
        }
        if self.readonly {
            return;
        }
        match event.key {
            Key::Char(b'\n') => {
                self.enter();
            }
            Key::Char(0x08) => {
                self.backspace();
            }
            Key::Char(c) => {
                self.input_char(c);
                self.term.put_raw(c, Color::Black, Color::White);
                self.term.flush();
            }
            _ => {}
        }
        // TODO
    }

    fn input_char(&mut self, c: u8) {
        if self.input_len >= MAX_INPUT {
            return;
        }
        self.input[self.input_len] = c;
        self.input_len += 1;
        // display character
    }

    fn backspace(&mut self) {
        if self.input_len == 0 {
            return;
        }
        self.input_len -= 1;
        self.input[self.input_len] = 0;
        self.term.backspace();
        self.term.flush();
    }

    fn enter(&mut self) {
        self.term.write("\n");
        self.term.flush();
        // display newline
        if let Ok(command) = core::str::from_utf8(&self.input[..self.input_len]) {
            shell::execute_cmd(command);
        }
        self.clear();
        self.prompt();
    }

    fn clear(&mut self) {
        self.input = [0; MAX_INPUT];
        self.input_len = 0;
    }
}

#[derive(Clone, Copy, PartialEq)]
struct PaneId(usize);

#[derive(Clone, Copy, PartialEq)]
struct NodeId(usize);

#[derive(Clone, Copy, PartialEq)]
struct WindowId(usize);

#[derive(Clone, Copy, PartialEq)]
enum Axis {
    Horizontal,
    Vertical,
}

#[derive(Clone)]
struct NodeEntry {
    parent: Option<NodeId>,
    kind: Node,
    depth: u8,
}

#[derive(Clone)]
enum Node {
    Leaf(PaneId),
    Split { axis: Axis, a: NodeId, b: NodeId },
}

#[derive(Clone)]
struct Window {
    root: NodeId,
    focused_pane: PaneId,
}

pub struct Compositor {
    prefix_armed: bool,
    active_window: WindowId,
    panes: [Option<Pane>; MAX_PANES],
    nodes: [Option<NodeEntry>; MAX_NODES],
    windows: [Option<Window>; MAX_WINDOWS],
}

impl Compositor {
    pub const fn new() -> Self {
        Self {
            prefix_armed: false,
            panes: [const { None }; MAX_PANES],
            nodes: [const { None }; MAX_NODES],
            windows: [const { None }; MAX_WINDOWS],
            active_window: WindowId(0),
        }
    }

    #[inline]
    fn get_next_node(&mut self) -> NodeId {
        for i in 0..MAX_NODES {
            if self.nodes[i].is_none() {
                self.nodes[i] = Some(NodeEntry {
                    parent: None,
                    // Sentinel value: should be immediately replaced
                    kind: Node::Leaf(PaneId(usize::MAX)),
                    depth: 0,
                });
                return NodeId(i);
            }
        }
        unreachable!();
    }

    #[inline]
    fn get_next_pane(&mut self) -> PaneId {
        for i in 0..MAX_PANES {
            if self.panes[i].is_none() {
                return PaneId(i);
            }
        }
        unreachable!();
    }

    fn pane(&self, id: PaneId) -> &Pane {
        self.panes[id.0].as_ref().unwrap()
    }
    fn pane_mut(&mut self, id: PaneId) -> &mut Pane {
        self.panes[id.0].as_mut().unwrap()
    }
    fn node(&self, id: NodeId) -> &NodeEntry {
        self.nodes[id.0].as_ref().unwrap()
    }
    fn node_mut(&mut self, id: NodeId) -> &mut NodeEntry {
        self.nodes[id.0].as_mut().unwrap()
    }
    fn window(&self, id: WindowId) -> &Window {
        self.windows[id.0].as_ref().unwrap()
    }
    fn window_mut(&mut self, id: WindowId) -> &mut Window {
        self.windows[id.0].as_mut().unwrap()
    }

    fn active_window(&self) -> &Window {
        self.window(self.active_window)
    }
    fn active_window_mut(&mut self) -> &mut Window {
        self.window_mut(self.active_window)
    }

    fn active_pane(&self) -> &Pane {
        self.pane(self.active_window().focused_pane)
    }
    fn active_pane_mut(&mut self) -> &mut Pane {
        self.pane_mut(self.active_window().focused_pane)
    }

    fn remove_node(&mut self, id: NodeId) {
        self.nodes[id.0] = None;
    }
    fn remove_pane(&mut self, id: PaneId) {
        self.panes[id.0] = None;
    }
    fn remove_window(&mut self, id: WindowId) {
        self.windows[id.0] = None;
    }

    fn new_pane(&mut self, rect: Rect, parent: NodeId) -> PaneId {
        let pane_id = self.get_next_pane();
        self.panes[pane_id.0] = Some(Pane::init(rect, parent));
        pane_id
    }

    pub fn init(&mut self) {
        let root_id = self.get_next_node();
        let pane_id = self.new_pane(
            Rect {
                x: 0,
                y: 0,
                w: vga::WIDTH,
                h: vga::HEIGHT - 1,
            },
            root_id,
        );
        self.nodes[root_id.0] = Some(NodeEntry {
            parent: None,
            kind: Node::Leaf(pane_id),
            depth: 0,
        });
        self.windows[0] = Some(Window {
            root: root_id,
            focused_pane: pane_id,
        });
        self.render();
    }

    fn forward_keypress(&mut self, event: KeyEvent) {
        self.active_pane_mut().handle_key(event);
    }

    fn process_command(&mut self, event: KeyEvent) {
        self.prefix_armed = false;
        if is_prefix_key(event) {
            self.forward_keypress(event);
            return;
        }
        match event.key {
            Key::Char(b'v') => self.split_pane(Axis::Vertical),
            Key::Char(b's') => self.split_pane(Axis::Horizontal),
            Key::Char(b'q') => self.close_pane(),
            // Axis names the split divide line, not the move direction, hence the reverse mapping
            Key::Char(b'j' | b'k') => self.move_focus(Axis::Horizontal),
            Key::Char(b'h' | b'l') => self.move_focus(Axis::Vertical),
            // Key::Nav(NavKey::ArrowUp) => self.resize_pane_relative()
            _ => {}
        }
    }

    fn get_pane_size(&self, id: PaneId) -> Rect {
        self.pane(id).rect
    }

    fn resize_pane(&mut self, id: PaneId, rect: Rect) {
        let pane = self.pane_mut(id);
        pane.rect = rect;
        pane.term.resize(rect.x, rect.y, rect.w, rect.h);
        pane.term.flush();
    }

    fn move_focus(&mut self, axis: Axis) {
        // V { {x} hsplit {} } vsplit { {} hsplit {} } -> { {} hsplit {} } vsplit { {x} hsplit {} }
        // V { {} hsplit {x} } vsplit { {} hsplit {} } -> { {} hsplit {} } vsplit { {} hsplit {x} }
        // V { {} hsplit {} } vsplit { {x} hsplit {} } -> { {x} hsplit {} } vsplit { {} hsplit {} }
        // V { {} hsplit {} } vsplit { {} hsplit {x} } -> { {} hsplit {x} } vsplit { {} hsplit {} }
        //
        // H { {x} vsplit {} } hsplit { {} vsplit {} } -> { {} vsplit {} } hsplit { {x} vsplit {} }
        // H { {} vsplit {x} } hsplit { {} vsplit {} } -> { {} vsplit {} } hsplit { {x} vsplit {} }
        // H { {} vsplit {} } hsplit { {x} vsplit {} } -> { {} vsplit {} } hsplit { {x} vsplit {} }
        // H { {} vsplit {} } hsplit { {} vsplit {x} } -> { {} vsplit {} } hsplit { {x} vsplit {} }

        // V { {x} vsplit {} } hsplit { {} vsplit {} } -> { {} vsplit {x} } hsplit { {} vsplit {} }
        // H { {x} hsplit {} } vsplit { {} hsplit {} } -> { {} hsplit {x} } vsplit { {} hsplit {} }

        let pane = self.active_pane();
        let pane_node = self.node(pane.parent);

        if let Some(parent_node_id) = pane_node.parent {
            let parent_node = self.node(parent_node_id);
            if let Node::Split {
                axis: split_axis,
                a,
                b,
            } = parent_node.kind
                && axis == split_axis
            {
                let id = if a == pane.parent { b } else { a };
                let Node::Leaf(pane_id) = self.node(id).kind else {
                    unreachable!()
                };
                self.active_window_mut().focused_pane = pane_id;
            }
        }
    }

    fn split_pane(&mut self, axis: Axis) {
        let pane_id = self.active_window().focused_pane;
        let node_id = self.active_pane().parent;

        if self.node(node_id).depth > 1 {
            return;
        }

        let new_leaf_id = self.get_next_node();
        let new_node_id = self.get_next_node();

        let mut rect = self.get_pane_size(pane_id);
        let mut new_rect = rect;
        match axis {
            Axis::Vertical => {
                new_rect.x = rect.x + rect.w / 2 + 1;
                new_rect.w = rect.w - rect.w / 2 - 1;
                rect.w /= 2;
            }
            Axis::Horizontal => {
                new_rect.y = rect.y + rect.h / 2 + 1;
                new_rect.h = rect.h - rect.h / 2 - 1;
                rect.h /= 2;
            }
        }

        self.resize_pane(pane_id, rect);
        let new_pane_id = self.new_pane(new_rect, new_leaf_id);
        self.active_pane_mut().parent = new_node_id;

        self.nodes[new_leaf_id.0] = Some(NodeEntry {
            kind: Node::Leaf(new_pane_id),
            parent: Some(node_id),
            depth: self.node(node_id).depth + 1,
        });
        self.nodes[new_node_id.0] = Some(NodeEntry {
            parent: Some(node_id),
            kind: Node::Leaf(pane_id),
            depth: self.node(node_id).depth + 1,
        });
        self.node_mut(node_id).kind = Node::Split {
            axis,
            a: new_node_id,
            b: new_leaf_id,
        };

        let window = self.active_window_mut();
        window.focused_pane = new_pane_id;
        self.render();
    }

    fn close_pane(&mut self) {
        let pane_id = self.active_window().focused_pane;
        let pane_node_id = self.active_pane().parent;
        let pane_node = self.node(pane_node_id);
        match pane_node.parent {
            // Replace parent node (Split) with next pane Leaf
            Some(node_id) => {
                let node = self.node(node_id);
                if let Node::Split { axis: _, a, b } = node.kind {
                    let other_node_id = if a == pane_node_id { b } else { a };
                    let other_node_kind = self.node(other_node_id).kind.clone();
                    if let Node::Leaf(other_pane_id) = other_node_kind {
                        let closing_rect = self.get_pane_size(pane_id);
                        self.active_window_mut().focused_pane = other_pane_id;
                        let other_pane = self.pane(other_pane_id);
                        self.resize_pane(other_pane_id, closing_rect.merge_with(other_pane.rect));
                        self.pane_mut(other_pane_id).parent = node_id;
                    }
                    self.node_mut(node_id).kind = other_node_kind;
                    self.remove_node(other_node_id);
                    self.remove_node(pane_node_id);
                    self.remove_pane(pane_id);
                    self.render();
                    return;
                }
                unreachable!();
            }
            // Close window
            None => {
                self.remove_node(pane_node_id);
                self.remove_pane(pane_id);
                self.remove_window(self.active_window);
            }
        }
        self.render();
    }

    fn switch_window(&mut self, id: WindowId) {
        if self.windows[id.0].is_none() {
            // Init new window
            let root_id = self.get_next_node();
            let pane_id = self.new_pane(
                Rect {
                    x: 0,
                    y: 0,
                    w: vga::WIDTH,
                    h: vga::HEIGHT - 1,
                },
                root_id,
            );

            self.nodes[root_id.0] = Some(NodeEntry {
                kind: Node::Leaf(pane_id),
                parent: None,
                depth: 0,
            });

            self.windows[id.0] = Some(Window {
                root: root_id,
                focused_pane: pane_id,
            });
        }
        self.active_window = id;
        self.render();
    }

    fn touches_active_pane(&self, (x, y): (usize, usize)) -> bool {
        let rect = self.active_pane().rect;
        let vertical_edge = (x + 1 == rect.x || x == rect.x + rect.w)
            && (rect.y - 1..rect.y + rect.h + 1).contains(&y);
        let horizontal_edge = (y + 1 == rect.y || y == rect.y + rect.h)
            && (rect.x - 1..rect.x + rect.w + 1).contains(&x);
        vertical_edge || horizontal_edge
    }

    fn collect_segments(
        &self,
        node: NodeId,
        segs: &mut [Option<Segment>; 3],
        count: &mut usize,
    ) -> Rect {
        match self.node(node).kind {
            Node::Leaf(pane_id) => self.pane(pane_id).rect,
            Node::Split { axis, a, b } => {
                let ra = self.collect_segments(a, segs, count);
                let rb = self.collect_segments(b, segs, count);
                let (offset, start, end) = match axis {
                    Axis::Vertical => {
                        let start = if ra.y == 0 { 0 } else { ra.y - 1 };
                        let end = if ra.y + ra.h >= vga::HEIGHT - 1 {
                            ra.y + ra.h
                        } else {
                            ra.y + ra.h + 1
                        };
                        (rb.x - 1, start, end)
                    }
                    Axis::Horizontal => {
                        let start = if ra.x == 0 { 0 } else { ra.x - 1 };
                        let end = if ra.x + ra.w >= vga::WIDTH - 1 {
                            ra.x + ra.w
                        } else {
                            ra.x + ra.w + 1
                        };
                        (rb.y - 1, start, end)
                    }
                };
                segs[*count] = Some(Segment {
                    axis,
                    offset,
                    start,
                    end,
                });
                *count += 1;
                ra.merge_with(rb)
            }
        }
    }

    fn render_node(&mut self, id: NodeId) {
        let node = self.node(id);
        match node.kind {
            Node::Leaf(pane_id) => {
                self.pane_mut(pane_id).term.flush();
            }
            Node::Split { axis: _, a, b } => {
                self.render_node(a);
                self.render_node(b);
            }
        }
    }

    fn render_segment(&mut self) {}

    fn render_statusbar(&mut self) {
        let bg = if self.prefix_armed {
            Color::Cyan
        } else {
            Color::Green
        };
        for x in 0..vga::WIDTH {
            vga::put_entry_at(vga::entry(b' ', Color::White, bg), x, vga::HEIGHT - 1);
        }
    }

    fn render(&mut self) {
        let root_id = self.active_window().root;
        self.render_node(root_id);
        let mut segs: [Option<Segment>; 3] = [None; 3];
        let mut count = 0;
        self.collect_segments(root_id, &mut segs, &mut count);

        for seg in segs.into_iter().flatten() {
            for i in seg.start..seg.end {
                let (x, y, c) = if seg.axis == Axis::Vertical {
                    (seg.offset, i, '│')
                } else {
                    (i, seg.offset, '─')
                };
                let fg = if self.touches_active_pane((x, y)) {
                    Color::Blue
                } else {
                    Color::Green
                };
                vga::put_entry_at(vga::entry(cp437::encode(c), fg, Color::White), x, y);
            }
        }

        // fix junctions
        for i in 0..count {
            for j in (i + 1)..count {
                let (a, b) = (segs[i].as_ref().unwrap(), segs[j].as_ref().unwrap());
                if a.axis == b.axis {
                    continue;
                }
                let (v, h) = if a.axis == Axis::Vertical {
                    (a, b)
                } else {
                    (b, a)
                };
                if let Some((x, y)) = v.intersect(h) {
                    let c = get_junction_char(v, h);
                    let fg = if self.touches_active_pane((x, y)) {
                        Color::Blue
                    } else {
                        Color::Green
                    };
                    vga::put_entry_at(vga::entry(cp437::encode(c), fg, Color::White), x, y);
                }
            }
        }

        self.render_statusbar();
    }

    pub fn handle_keyboard(&mut self) {
        while let Some(event) = keyboard::get_key() {
            if self.prefix_armed {
                self.process_command(event);
                continue;
            }
            if let Key::Function(f) = event.key {
                self.switch_window(WindowId(f as usize));
                continue;
            }
            if is_prefix_key(event) {
                self.prefix_armed = true;
                continue;
            }
            self.forward_keypress(event);
        }
        self.render_statusbar();
    }
}

impl fmt::Write for Compositor {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.active_pane_mut().term.write(s);
        self.render();
        Ok(())
    }
}

fn get_junction_char(v: &Segment, h: &Segment) -> char {
    let up = v.start < h.offset;
    let down = v.end > h.offset + 1;
    let left = h.start < v.offset;
    let right = h.end > v.offset + 1;
    match (up, down, left, right) {
        (true, true, true, true) => '┼',
        (true, true, true, false) => '┤',
        (true, true, false, true) => '├',
        (false, true, true, true) => '┬',
        (true, false, true, true) => '┴',
        _ => unreachable!(),
    }
}

#[inline]
fn is_prefix_key(event: KeyEvent) -> bool {
    event.mods.ctrl && matches!(event.key, Key::Char(b'a'))
}

static mut COMPOSITOR: Compositor = Compositor::new();

fn get_compositor() -> &'static mut Compositor {
    unsafe { (&raw mut COMPOSITOR).as_mut().unwrap() }
}

pub fn put_raw(c: u8, fg: vga::Color, bg: vga::Color) {
    get_compositor().active_pane_mut().term.put_raw(c, fg, bg);
}

pub fn get_terminfo() -> Rect {
    let comp = get_compositor();
    comp.get_pane_size(comp.active_window().focused_pane)
}

pub fn init() {
    get_compositor().init();
}

pub fn handle_keyboard() {
    get_compositor().handle_keyboard();
}

pub fn clear() {
    let compositor = get_compositor();
    let pane = compositor.active_pane_mut();
    pane.term.clear();
    pane.term.flush();
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use fmt::Write;

    get_compositor().write_fmt(args).unwrap();
}

#[doc(hidden)]
pub fn _log(_args: fmt::Arguments) {
    // get_compositor().write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! println {
    () => {
        $crate::compositor::_print(
            format_args!("\n"),
        )
    };

    ($($arg:tt)*) => {
        $crate::compositor::_print(
            format_args!("{}\n", format_args!($($arg)*)),
        )
    };
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::compositor::_print(
            format_args!($($arg)*),
        )
    };
}

// TODO replace with serial output
#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
        $crate::compositor::_log(
            format_args!("{}\n", format_args!($($arg)*)),
        )
    };
}
