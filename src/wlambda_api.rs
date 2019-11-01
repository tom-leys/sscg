use std::rc::Rc;
use std::cell::RefCell;
use crate::logic::*;
use crate::gui;
use wlambda::{VVal, StackAction, VValUserData, GlobalEnv, EvalContext, SymbolTable};

fn vval_to_system(v: VVal) -> Result<Rc<RefCell<System>>, StackAction> {
    match v {
        VVal::Usr(mut ud) => {
            if let Some(sys) = ud.as_any().downcast_ref::<SystemWlWrapper>() {
                Ok(sys.0.clone())
            } else {
                Err(StackAction::panic_msg(
                    format!("{} is not a system", ud.s())))
            }
        },
        _ => {
            Err(StackAction::panic_msg(
                format!("{} is not a system", v.s())))
        }
    }
}

pub fn get_system_state(gs: &GameState, id: ObjectID) -> Option<VVal> {
    let system : Rc<RefCell<System>> = gs.get_system(id)?;

    let ret = VVal::vec();
    ret.push(SystemWlWrapper::vval_from(system));

    Some(ret)
}

pub fn get_ship_state(gs: &GameState, id: ObjectID) -> Option<VVal> {
    let ship   : Rc<RefCell<Ship>>   = gs.get_ship(id)?;
    let system : Rc<RefCell<System>> = gs.get_system(ship.borrow().system)?;

    let entity =
        system.borrow_mut().get_entity_close_to(
            ship.borrow().pos.0,
            ship.borrow().pos.1);

    let ret = VVal::vec();
    ret.push(ShipWlWrapper::vval_from(ship));
    ret.push(SystemWlWrapper::vval_from(system));

    if let Some(ent) = entity {
        ret.push(EntityWlWrapper::vval_from(ent));
    }

    Some(ret)
}


#[derive(Clone)]
pub struct GameStateWlWrapper(Rc<RefCell<GameState>>);
impl GameStateWlWrapper {
    pub fn vval_from(r: Rc<RefCell<GameState>>) -> VVal {
        VVal::Usr(Box::new(GameStateWlWrapper(r)))
    }
}

impl VValUserData for GameStateWlWrapper {
    fn s(&self) -> String { format!("$<GameState>") }
    fn set_key(&self, key: &VVal, val: VVal) {
        self.0.borrow_mut().state.set_key(key, val);
    }
    fn get_key(&self, key: &str) -> Option<VVal> {
        match key {
            _  => self.0.borrow().state.get_key(key),
        }
    }
    fn call(&self, args: &[VVal]) -> Result<VVal, StackAction> {
        if args.len() < 1 {
            return Err(StackAction::panic_msg(
                format!("{} called with too few arguments", self.s())));
        }
        match &args[0].s_raw()[..] {
            "add_entity" => {
                if args.len() < 5 {
                    return Err(StackAction::panic_msg(
                        format!("`{} :add_entity` called with too few arguments",
                                self.s())));
                }
                let sys = vval_to_system(args[1].clone())?;
                Ok(EntityWlWrapper::vval_from(
                    self.0.borrow_mut().system_add_entity(
                        sys,
                        args[2].i() as i32,
                        args[3].i() as i32,
                        args[4].clone())))
            },
            "add_system" => {
                if args.len() < 4 {
                    return Err(StackAction::panic_msg(
                        format!("`{} :add_system` called with too few arguments",
                                self.s())));
                }
                Ok(SystemWlWrapper::vval_from(
                    self.0.borrow_mut().add_system(
                        args[1].i() as i32,
                        args[2].i() as i32,
                        args[3].clone())))
            },
            "list_by_type" => {
                if args.len() < 2 {
                    return Err(StackAction::panic_msg(
                        format!("`{} :list_systems` called with too few arguments",
                                self.s())));
                }
                let typ = args[1].s_raw();
                let out = VVal::vec();
                for o in self.0.borrow_mut().object_registry.borrow_mut().objects.iter() {
                    match o {
                        Object::Entity(e) => {
                            if typ == "entity" {
                                out.push(EntityWlWrapper::vval_from(e.clone()));
                            }
                        },
                        Object::System(s) => {
                            if typ == "system" {
                                out.push(SystemWlWrapper::vval_from(s.clone()));
                            }
                        },
                        Object::Ship(s)   => {
                            if typ == "ship" {
                                out.push(ShipWlWrapper::vval_from(s.clone()));
                            }
                        },
                        _ => ()
                    }
                }
                Ok(out)
            },
            "object_by_id" => {
                if args.len() < 2 {
                    return Err(StackAction::panic_msg(
                        format!("`{} :object_by_id` called with too few arguments",
                                self.s())));
                }
                let o =
                    self.0.borrow_mut()
                        .object_registry.borrow_mut()
                        .get(args[1].i() as ObjectID);
                if let Some(obj) = o {
                    Ok(match obj {
                        Object::Entity(e) => EntityWlWrapper::vval_from(e),
                        Object::System(s) => SystemWlWrapper::vval_from(s),
                        Object::Ship(s)   => ShipWlWrapper::vval_from(s),
                        Object::None      => VVal::Nul,
                    })
                } else {
                    Ok(VVal::Nul)
                }
            },
            _ => Ok(VVal::Nul)
        }
    }
    fn as_any(&mut self) -> &mut dyn std::any::Any { self }
    fn clone_ud(&self) -> Box<dyn wlambda::vval::VValUserData> {
        Box::new(self.clone())
    }
}

#[derive(Clone)]
pub struct EntityWlWrapper(Rc<RefCell<Entity>>);
impl EntityWlWrapper {
    pub fn vval_from(r: Rc<RefCell<Entity>>) -> VVal {
        VVal::Usr(Box::new(EntityWlWrapper(r)))
    }
}

impl VValUserData for EntityWlWrapper {
    fn s(&self) -> String { format!("$<Entity:{}>", self.0.borrow().id) }
    fn i(&self) -> i64 { self.0.borrow().id as i64 }
    fn set_key(&self, key: &VVal, val: VVal) {
        match &key.s_raw()[..] {
            "name" => { self.0.borrow_mut().name = val.s_raw(); },
            _      => { self.0.borrow_mut().state.set_key(key, val); }
        }
    }
    fn get_key(&self, key: &str) -> Option<VVal> {
        match key {
            "id"   => Some(VVal::Int(self.0.borrow().id as i64)),
            "name" => Some(VVal::new_str(&self.0.borrow().name)),
            "typ" => {
                Some(VVal::new_str(
                    match self.0.borrow().typ {
                        SystemObject::Station       => "station",
                        SystemObject::AsteroidField => "asteroid_field",
                    }))
            },
            _    => self.0.borrow().state.get_key(key),
        }
    }
    fn call(&self, args: &[VVal]) -> Result<VVal, StackAction> {
        if args.len() < 1 {
            return Err(StackAction::panic_msg(
                format!("{} called with too few arguments", self.s())));
        }
        match &args[0].s_raw()[..] {
            _            => Ok(VVal::Nul)
        }
    }
    fn as_any(&mut self) -> &mut dyn std::any::Any { self }
    fn clone_ud(&self) -> Box<dyn wlambda::vval::VValUserData> {
        Box::new(self.clone())
    }
}

#[derive(Clone)]
pub struct ShipWlWrapper(Rc<RefCell<Ship>>);
impl ShipWlWrapper {
    pub fn vval_from(r: Rc<RefCell<Ship>>) -> VVal {
        VVal::Usr(Box::new(ShipWlWrapper(r)))
    }
}

impl VValUserData for ShipWlWrapper {
    fn s(&self) -> String { format!("$<Ship:{}>", self.0.borrow().id) }
    fn i(&self) -> i64 { self.0.borrow().id as i64 }
    fn set_key(&self, key: &VVal, val: VVal) {
        self.0.borrow_mut().state.set_key(key, val);
    }
    fn get_key(&self, key: &str) -> Option<VVal> {
        // println!("GET KEY: {} : STTE: {}", key, self.0.borrow().state.s());
        match key {
            "id"        => Some(VVal::Int(self.0.borrow().id as i64)),
            "system_id" => Some(VVal::Int(self.0.borrow().system as i64)),
            _ => self.0.borrow().state.get_key(key),
        }
    }
    fn call(&self, args: &[VVal]) -> Result<VVal, StackAction> {
        if args.len() < 1 {
            return Err(StackAction::panic_msg(
                format!("{} called with too few arguments", self.s())));
        }
        match &args[0].s_raw()[..] {
            "set_notification" => {
                if args.len() < 2 {
                    return Err(StackAction::panic_msg(
                        format!("`{} :set_system` called with too few arguments",
                                self.s())));
                }

                let txt = args[1].s_raw();

                self.0.borrow_mut().set_notification(txt);
                Ok(VVal::Bol(true))
            },
            "set_system" => {
                if args.len() < 2 {
                    return Err(StackAction::panic_msg(
                        format!("`{} :set_system` called with too few arguments",
                                self.s())));
                }

                let sys = vval_to_system(args[1].clone())?;

                self.0.borrow_mut().system = sys.borrow().id;
                Ok(VVal::Bol(true))
            },
            _ => Ok(VVal::Nul)
        }
    }
    fn as_any(&mut self) -> &mut dyn std::any::Any { self }
    fn clone_ud(&self) -> Box<dyn wlambda::vval::VValUserData> {
        Box::new(self.clone())
    }
}

#[derive(Clone)]
struct SystemWlWrapper(Rc<RefCell<System>>);
impl SystemWlWrapper {
    pub fn vval_from(r: Rc<RefCell<System>>) -> VVal {
        VVal::Usr(Box::new(SystemWlWrapper(r)))
    }
}

impl VValUserData for SystemWlWrapper {
    fn s(&self) -> String { format!("$<System:{}>", self.0.borrow().id) }
    fn i(&self) -> i64 { self.0.borrow().id as i64 }
    fn set_key(&self, key: &VVal, val: VVal) {
        self.0.borrow_mut().state.set_key(key, val);
    }
    fn get_key(&self, key: &str) -> Option<VVal> {
        match key {
            "id"    => Some(VVal::Int(self.0.borrow().id as i64)),
            _       => self.0.borrow().state.get_key(key),
        }
    }
    fn call(&self, args: &[VVal]) -> Result<VVal, StackAction> {
        if args.len() < 1 {
            return Err(StackAction::panic_msg(
                format!("{} called with too few arguments", self.s())));
        }
        match &args[0].s_raw()[..] {
            "foo" => { Ok(VVal::Bol(true)) },
            _     => Ok(VVal::Nul)
        }
    }
    fn as_any(&mut self) -> &mut dyn std::any::Any { self }
    fn clone_ud(&self) -> Box<dyn wlambda::vval::VValUserData> {
        Box::new(self.clone())
    }
}

pub struct WindowManager {
    pub windows: std::vec::Vec<Option<gui::Window>>,
    pub ev_cbs: std::vec::Vec<VVal>,
}

impl WindowManager {
    pub fn new() -> Self {
        Self {
            windows: std::vec::Vec::new(),
            ev_cbs: std::vec::Vec::new(),
        }
    }

    pub fn handle_activated_childs(&mut self, wl_ctx: &mut EvalContext) {
        for (w, cb) in self.windows.iter_mut().zip(self.ev_cbs.iter()) {
            if let Some(w) = w {
                if let Some(lblref) = w.collect_activated_child() {
                    let args = vec![VVal::new_str_mv(lblref.to_string())];
                    if let Err(e) = wl_ctx.clone().call(cb, &args) {
                        println!("ERROR IN WM CB: {}", e);
                    }
                }
            }
        }
    }

    pub fn set_label_text(&mut self, idx: usize, lblref: &str, text: String) {
        if idx >= self.windows.len() {
            return;
        }
        if self.windows[idx].is_none() {
            return;
        }

        if let Some(e) = self.windows.get_mut(idx) {
            e.as_mut().unwrap().set_label_text(lblref, text);
        }
    }

    pub fn get_window_state(&self, idx: usize) -> VVal {
        if idx >= self.windows.len() {
            return VVal::Nul;
        }
        if self.windows[idx].is_none() {
            return VVal::Nul;
        }

        if let Some(e) = self.windows.get(idx) {
            let txts = e.as_ref().unwrap().collect_label_texts();
            let s = VVal::map();
            let v = VVal::map();
            for (lblref, text) in txts.into_iter() {
                v.set_map_key(lblref, VVal::new_str_mv(text));
            }
            s.set_map_key("labels".to_string(), v);
            s
        } else {
            VVal::Nul
        }
    }

    pub fn get_label_text(&self, idx: usize, lblref: &str) -> VVal {
        if idx >= self.windows.len() {
            return VVal::Nul;
        }
        if self.windows[idx].is_none() {
            return VVal::Nul;
        }

        if let Some(e) = self.windows.get(idx) {
            let r = e.as_ref().unwrap().get_label_text(lblref);
            if r.is_some() {
                VVal::new_str_mv(r.unwrap())
            } else {
                VVal::Nul
            }
        } else {
            VVal::Nul
        }
    }

    pub fn new_ref() -> std::rc::Rc<std::cell::RefCell<Self>> {
        std::rc::Rc::new(std::cell::RefCell::new(Self::new()))
    }

    pub fn delete(&mut self, idx: usize) {
        if idx >= self.windows.len() { return; }
        self.windows[idx] = None;
        self.ev_cbs[idx] = VVal::Nul;
    }

    pub fn set(&mut self, idx: usize, win: gui::Window, cb: VVal) -> usize {
        if idx >= self.windows.len() {
            self.windows.resize(idx + 1, None);
            self.ev_cbs.resize(idx + 1, VVal::Nul);
        }
        println!("SET WINDOW {}", idx);
        self.windows[idx] = Some(win);
        self.ev_cbs[idx]  = cb;
        idx
    }
}

#[derive(Clone)]
pub struct WindowManagerWlWrapper(Rc<RefCell<WindowManager>>);
impl WindowManagerWlWrapper {
    pub fn vval_from(r: Rc<RefCell<WindowManager>>) -> VVal {
        VVal::Usr(Box::new(WindowManagerWlWrapper(r)))
    }
}

fn shiftaddup4(u: u8) -> u8 { (u << 4) | u }

fn color_hex24tpl(s: &str) -> (u8, u8, u8, u8) {
    match s.len() {
        8 => (
            u8::from_str_radix(&s[0..2], 16).unwrap_or(0),
            u8::from_str_radix(&s[2..4], 16).unwrap_or(0),
            u8::from_str_radix(&s[4..6], 16).unwrap_or(0),
            u8::from_str_radix(&s[6..8], 16).unwrap_or(255)
        ),
        6 => (
            u8::from_str_radix(&s[0..2], 16).unwrap_or(0),
            u8::from_str_radix(&s[2..4], 16).unwrap_or(0),
            u8::from_str_radix(&s[4..6], 16).unwrap_or(0),
            255
        ),
        4 => (
            shiftaddup4(u8::from_str_radix(&s[0..1], 16).unwrap_or(0)),
            shiftaddup4(u8::from_str_radix(&s[1..2], 16).unwrap_or(0)),
            shiftaddup4(u8::from_str_radix(&s[2..3], 16).unwrap_or(0)),
            shiftaddup4(u8::from_str_radix(&s[3..4], 16).unwrap_or(0xF)),
        ),
        3 => (
            shiftaddup4(u8::from_str_radix(&s[0..1], 16).unwrap_or(0)),
            shiftaddup4(u8::from_str_radix(&s[1..2], 16).unwrap_or(0)),
            shiftaddup4(u8::from_str_radix(&s[2..3], 16).unwrap_or(0)),
            255
        ),
        _ => (255, 0, 255, 255),
    }
}

fn vval2size(v: VVal) -> gui::Size {
    let mut s = gui::Size {
        min_w: 0,
        min_h: 0,
        w:     1000,
        h:     1000,
        margin: 0,
    };

    s.min_w  = v.get_key("min_w") .unwrap_or(VVal::Int(0)).i() as u32;
    s.min_h  = v.get_key("min_h") .unwrap_or(VVal::Int(0)).i() as u32;
    s.w      = v.get_key("w")     .unwrap_or(VVal::Int(0)).i() as u32;
    s.h      = v.get_key("h")     .unwrap_or(VVal::Int(0)).i() as u32;
    s.margin = v.get_key("margin").unwrap_or(VVal::Int(0)).i() as u32;

    s
}

fn vval2widget(v: VVal, win: &mut gui::Window) -> usize {
    let mut childs = vec![];
    if let Some(VVal::Lst(l)) = v.get_key("childs") {
        for w in l.borrow().iter() {
            childs.push(vval2widget(w.clone(), win));
        }
    }

    match &v.get_key("t").unwrap_or(VVal::Nul).s_raw()[..] {
        "vbox" => {
            return win.add_layout(
                vval2size(v.clone()),
                gui::BoxDir::Vert(
                    v.get_key("spacing").unwrap_or(VVal::Int(0)).i() as u32),
                &childs);
        },
        "hbox" => {
            return win.add_layout(
                vval2size(v.clone()),
                gui::BoxDir::Hori(
                    v.get_key("spacing").unwrap_or(VVal::Int(0)).i() as u32),
                &childs);
        },
        _ => ()
    }

    let lbl =
        gui::Label::new(
            &v.get_key("text").unwrap_or(VVal::new_str("")).s_raw(),
            color_hex24tpl(
                &v.get_key("fg").unwrap_or(VVal::new_str("")).s_raw()),
            color_hex24tpl(
                &v.get_key("bg").unwrap_or(VVal::new_str("")).s_raw()))
        .lblref(&v.get_key("ref").unwrap_or(VVal::new_str("")).s_raw());

    let lbl = match &v.get_key("t").unwrap_or(VVal::Nul).s_raw()[..] {
        "l_button" => lbl.left().clickable(),
        "r_button" => lbl.right().clickable(),
        "c_button" => lbl.center().clickable(),
        "field" => {
            lbl.left().editable(
                &v.get_key("regex").unwrap_or(VVal::new_str(".*")).s_raw())
        },
        "c_label" => lbl.center(),
        "l_label" => lbl.left(),
        "r_label" => lbl.right(),
        _ => lbl,
    };

    win.add_label(vval2size(v.clone()), lbl)
}

fn vval2win(v: VVal) -> gui::Window {
    let mut w = gui::Window::new();
    w.x     = v.get_key("x").unwrap_or(VVal::Int(0)).i() as i32;
    w.y     = v.get_key("y").unwrap_or(VVal::Int(0)).i() as i32;
    w.w     = v.get_key("w").unwrap_or(VVal::Int(500)).i() as i32;
    w.h     = v.get_key("h").unwrap_or(VVal::Int(500)).i() as i32;
    w.title = v.get_key("title").unwrap_or(VVal::new_str("Unnamed")).s_raw();
    if let Some(tc) = v.get_key("title_color") {
        w.title_color = color_hex24tpl(&tc.s_raw());
    }
    let id = vval2widget(v.get_key("child").unwrap_or(VVal::Nul), &mut w);
    w.child = id;

    w
}

impl VValUserData for WindowManagerWlWrapper {
    fn s(&self) -> String { format!("$<WindowManager>") }
    fn i(&self) -> i64 { 0 }
    fn call(&self, args: &[VVal]) -> Result<VVal, StackAction> {
        if args.len() < 1 {
            return Err(StackAction::panic_msg(
                format!("{} called with too few arguments", self.s())));
        }
        match &args[0].s_raw()[..] {
            "set_window" => {
                if args.len() < 4 {
                    return Err(StackAction::panic_msg(
                        format!("`{} :set_window` called with too few arguments",
                                self.s())));
                }


                let idx = args[1].i();
                if !args[2].is_none() {
                    let win = vval2win(args[2].clone());
                    let cb  = args[3].clone();

                    self.0.borrow_mut().set(idx as usize, win, cb);
                } else {
                    self.0.borrow_mut().delete(idx as usize);
                }

                Ok(VVal::Bol(true))
            },
            "set_label" => {
                if args.len() < 3 {
                    return Err(StackAction::panic_msg(
                        format!("`{} :set_label` called with too few arguments",
                                self.s())));
                }

                let idx    = args[1].i();
                let lblref = args[2].s_raw();
                let txt    = args[3].s_raw();
                self.0.borrow_mut().set_label_text(idx as usize, &lblref, txt);

                Ok(VVal::Bol(true))
            },
            "get_label" => {
                if args.len() < 2 {
                    return Err(StackAction::panic_msg(
                        format!("`{} :get_label` called with too few arguments",
                                self.s())));
                }

                let idx    = args[1].i();
                let lblref = args[2].s_raw();
                Ok(self.0.borrow_mut().get_label_text(idx as usize, &lblref))
            },
            "get_state" => {
                if args.len() < 1 {
                    return Err(StackAction::panic_msg(
                        format!("`{} :get_label` called with too few arguments",
                                self.s())));
                }

                let idx = args[1].i();
                Ok(self.0.borrow_mut().get_window_state(idx as usize))
            },
            _ => Ok(VVal::Nul)
        }
    }
    fn as_any(&mut self) -> &mut dyn std::any::Any { self }
    fn clone_ud(&self) -> Box<dyn wlambda::vval::VValUserData> {
        Box::new(self.clone())
    }
}

