use crate::logic::GamePainter;

#[derive(Debug, Clone)]
pub enum Widget {
    Container(usize, Layout, Container),
    Label(usize, Layout, Label),
}

impl Widget {
    pub fn id(&self) -> usize {
        match self {
            Widget::Container(id, _, _) => *id,
            Widget::Label(id, _, _)     => *id,
        }
    }
    pub fn calc_feedback<P>(&self, max_w: u32, max_h: u32, p: &mut P) -> WidgetFeedback where P: GamePainter {
        let pos = p.get_screen_pos(0, 0);
        let (id, (mw, mh)) = match self {
            Widget::Container(id, l, _) => (*id, l.size(max_w, max_h)),
            Widget::Label(id, l, _)     => (*id, l.size(max_w, max_h)),
        };
        WidgetFeedback {
            id,
            x: pos.0 as u32,
            y: pos.1 as u32,
            w: mw,
            h: mh,
        }
    }

    pub fn draw<P>(&self, win: &Window, fb: &mut [WidgetFeedback], max_w: u32, max_h: u32, p: &mut P) -> (u32, u32)
        where P: GamePainter {

        let w_fb = self.calc_feedback(max_w, max_h, p);
        fb[self.id()] = w_fb;
        let (mw, mh) = (w_fb.w, w_fb.h);
        match self {
            Widget::Container(_id, _layout, c) => {
                match c.dir {
                    BoxDir::Vert => {
                        let mut offs = 0;
                        p.push_add_offs(0, offs);
                        for c_id in c.childs.iter() {
                            let (_w, h) =
                                win.widgets[*c_id].draw(
                                    win, fb, mw, mh, p);
                            offs += h as i32;
                            p.pop_offs();
                            p.push_add_offs(0, offs);
                        }
                        p.pop_offs();
                    },
                    BoxDir::Hori => {
                        let mut offs = 0;
                        p.push_add_offs(offs, 0);
                        for c_id in c.childs.iter() {
                            let (w, _h) =
                                win.widgets[*c_id].draw(
                                    win, fb, mw, mh, p);
                            offs += w as i32;
                            p.pop_offs();
                            p.push_add_offs(offs, 0);
                        }
                        p.pop_offs();
                    },
                }
            },
            Widget::Label(_id, _layout, lbl) => {
                let txt = lbl.text.clone();
                let (tw, _th) = p.text_size(&txt);
                if tw > mw {
                    let mut line = String::from("");
                    let mut y = 0;
                    for c in txt.chars() {
                        line.push(c);
                        let (tw, th) = p.text_size(&line);
                        if tw > mw {
                            line.pop();
                            p.draw_text(
                                0, y, mw, lbl.fg_color, Some(lbl.bg_color), &line);
                            line = String::from("");
                            line.push(c);
                            y += th as i32;
                        }
                    }

                    if line.len() > 0 {
                        p.draw_text(
                            0, y, mw, lbl.fg_color, Some(lbl.bg_color), &line);
                    }
                } else {
                    p.draw_text(
                        0, 0,
                        mw, lbl.fg_color, Some(lbl.bg_color), &lbl.text);
                }
            },
        }
        (w_fb.w, w_fb.h)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct WidgetFeedback {
    id: usize,
    x:  u32,
    y:  u32,
    w:  u32,
    h:  u32,
}

impl WidgetFeedback {
    pub fn new() -> Self {
        Self {
            id: 0,
            x: 0,
            y: 0,
            w: 0,
            h: 0,
        }
    }
}

pub struct Window {
    widgets:      std::vec::Vec<Widget>,
    feedback:     std::vec::Vec<WidgetFeedback>,
    child:        usize,
    focus_child:  Option<usize>,
    hover_child:  Option<usize>,
    activ_child:  Option<usize>,
    x:            u32,
    y:            u32,
    w:            u32,
    h:            u32,
    min_w:        u32,
    min_h:        u32,
    needs_redraw: bool,
}

/// All values are in 0.1% scale. that means, to represent 100% you have to
/// supply 1000 to ratio to get the full value.
fn p2r(value: u32, ratio: u32) -> u32 { (value * ratio) / 1000 }

pub enum WindowEvent {
    MousePos(u32, u32),
    Click(u32, u32),
    TextInput(char),
    Backspace,
}

impl Window {
    pub fn draw<P>(&mut self, max_w: u32, max_h: u32, p: &mut P)
        where P: GamePainter {

        let mut feedback = std::vec::Vec::new();
        feedback.resize(self.widgets.len(), WidgetFeedback::new());
        let child    = &self.widgets[self.child];

        p.push_offs(
            p2r(max_w, self.x) as i32,
            p2r(max_h, self.y) as i32);
        child.draw(
            &self, &mut feedback[..],
            p2r(max_w, self.w),
            p2r(max_h, self.h),
            p);
        p.pop_offs();
        self.feedback = feedback;
    }

    pub fn needs_redraw(&self) -> bool { self.needs_redraw }

    pub fn get_label_text(&self, lblref: &str) -> Option<String> {
        for c in self.widgets.iter() {
            match c {
                Widget::Label(_, _, lbl) => {
                    if &lbl.lblref[..] == lblref {
                        return Some(lbl.text.clone());
                    }
                },
                _ => (),
            }
        }

        None
    }

    pub fn set_label_text(&mut self, lblref: &str, text: String) {
        for c in self.widgets.iter_mut() {
            match c {
                Widget::Label(_, _, lbl) => {
                    if &lbl.lblref[..] == lblref {
                        lbl.text = text.clone();
                        self.needs_redraw = true;
                    }
                },
                _ => (),
            }
        }
    }

    pub fn collect_activated_child(&mut self) -> Option<String> {
        if let Some(idx) = self.activ_child {
            match &self.widgets[idx] {
                Widget::Label(_, _, lbl) => { return Some(lbl.lblref.clone()); }
                _ => (),
            }
        }
        None
    }

    pub fn handle_event(&mut self, ev: WindowEvent) -> bool {
        match ev {
            WindowEvent::MousePos(_x, _y) => { true },
            WindowEvent::Click(_x, _y)    => {
                // set self.activ_child ...
                true
            },
            WindowEvent::TextInput(_c)   => { true },
            WindowEvent::Backspace      => { true },
        }
    }
}

#[derive(Debug, Copy, Clone)]
enum BoxDir {
    Vert,
    Hori,
}

#[derive(Debug, Clone)]
pub struct Container {
    dir:        BoxDir,
    childs:     std::vec::Vec<usize>,
}

#[derive(Debug, Copy, Clone)]
enum HAlign {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone)]
pub struct Layout {
    min_w: u32,
    w:     u32,
    min_h: u32,
    h:     u32,
}

impl Layout {
    pub fn size(&self, max_w: u32, max_h: u32) -> (u32, u32) {
        let rw = p2r(max_w, self.w);
        let rh = p2r(max_h, self.h);
        (
            if rw < self.min_w { self.min_w }
            else { rw },
            if rh < self.min_h { self.min_h }
            else { rh },
        )
    }
}


#[derive(Debug, Copy, Clone)]
pub enum LabelStyle {
}

#[derive(Debug, Clone)]
pub struct Label {
    lblref:     String,
    text:       String,
    align:      HAlign,
    editable:   bool,
    clickable:  bool,
    fg_color:   (u8, u8, u8, u8),
    bg_color:   (u8, u8, u8, u8),
}