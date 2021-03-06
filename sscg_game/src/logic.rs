use wlambda::VVal;
use std::rc::Rc;
use std::cell::RefCell;
use vector2d::Vector2D;

const TICK_RES : i32 = 1000 / 25;

pub type ObjectID = usize;

pub type EventCallback = dyn Fn(&Rc<RefCell<GameState>>, VVal);

pub fn sys2screen(v: i32) -> i32 { (v * 1280) / 10000 }
pub fn screen2sys(v: i32) -> i32 { (v * 10000) / 1280 }

#[derive(Clone)]
pub struct GameState {
    pub object_registry:    Rc<RefCell<ObjectRegistry>>,
    pub event_router:       Rc<RefCell<EventRouter>>,
    pub active_ship_id:     ObjectID,
    pub state:              VVal,
}

impl GameState {
    pub fn new_ref() -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(GameState {
            object_registry: Rc::new(RefCell::new(ObjectRegistry::new())),
            event_router:    Rc::new(RefCell::new(EventRouter::new())),
            state:           VVal::map(),
            active_ship_id:  0,
        }))
    }

    pub fn serialize(&self) -> VVal {
        let objreg = self.object_registry.borrow().serialize();
        let v = VVal::vec();
        v.push(VVal::new_str("sscg_savegame"));
        v.push(VVal::Int(0)); // version
        v.push(self.state.clone());
        v.push(VVal::Int(self.active_ship_id as i64));
        v.push(objreg);
        return v;
    }

    pub fn deserialize(&mut self, v: VVal) {
        self.object_registry.borrow_mut().deserialize(v.at(4).unwrap_or(VVal::Nul));
        self.state          = v.at(2).unwrap_or(VVal::Nul);
        self.active_ship_id = v.at(3).unwrap_or(VVal::Nul).i() as ObjectID;
    }

    pub fn get_ship(&self, id: ObjectID) -> Option<Rc<RefCell<Ship>>> {
        match self.object_registry.borrow_mut().get(id) {
            Some(Object::Ship(s)) => Some(s.clone()),
            _ => None,
        }
    }

    pub fn get_system(&self, id: ObjectID) -> Option<Rc<RefCell<System>>> {
        match self.object_registry.borrow_mut().get(id) {
            Some(Object::System(s)) => Some(s.clone()),
            _ => None,
        }
    }

    pub fn add_system(&self, x: i32, y: i32, state: VVal) -> Rc<RefCell<System>> {
        for o in self.object_registry.borrow().objects.iter() {
            match o {
                Object::System(s) => {
                    return s.clone();
                },
                _ => (),
            }
        }

        let mut sys = System::new(x, y);
        sys.state = state;
        self.object_registry.borrow_mut().add_system(sys)
    }

    pub fn system_add_entity(
        &self, sys: Rc<RefCell<System>>,
        x: i32, y: i32, state: VVal) -> Rc<RefCell<Entity>> {

        let typ =
            match &state.get_key("type").unwrap_or(VVal::Nul).s_raw()[..] {
                "station"           => SystemObject::Station,
                "asteroid_field"    => SystemObject::AsteroidField,
                _                   => SystemObject::AsteroidField,
            };

        let mut ent = Entity::new(typ);
        ent.state = state;

        let e = self.object_registry.borrow_mut().add_entity(ent);
        sys.borrow_mut().add(x, y, e.clone());
        e
    }

    pub fn reg_cb<F>(&self, ev: String, f: F)
        where F: 'static + Fn(&Rc<RefCell<GameState>>, VVal) {
        self.event_router.borrow_mut().reg_cb(ev, f);
    }

    pub fn update(&self, frame_time_ms: f64) {
        let mut os = self.object_registry.borrow_mut();
        let mut er = self.event_router.borrow_mut();
        os.update(frame_time_ms, &mut *er);
    }
}

#[derive(Debug, Clone)]
pub enum Object {
    None,
    Entity(Rc<RefCell<Entity>>),
    System(Rc<RefCell<System>>),
    Ship(Rc<RefCell<Ship>>),
}

impl Object {
    pub fn id(&self) -> ObjectID {
        match self {
            Object::None      => 0,
            Object::Entity(e) => e.borrow().id,
            Object::System(s) => s.borrow().id,
            Object::Ship(s)   => s.borrow().id,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ObjectRegistry {
    pub objects:        std::vec::Vec<Object>,
    tick_count:         i32,
    tick_time_ms:       f64,
}

impl ObjectRegistry {
    pub fn new() -> Self {
        ObjectRegistry {
            objects: std::vec::Vec::new(),
            tick_count:   0,
            tick_time_ms: 0.0,
        }
    }

    pub fn serialize(&self) -> VVal {
        let v = VVal::vec();
        v.push(VVal::Int(self.objects.len() as i64));

        let objs = VVal::vec();
        for o in self.objects.iter() {
            objs.push(match o {
                Object::Entity(e) => e.borrow().serialize(),
                Object::System(e) => e.borrow().serialize(),
                Object::Ship(e)   => e.borrow().serialize(),
                _                 => VVal::Nul,
            });
        }
        v.push(objs);

        v
    }

    fn vval_to_object(&mut self, v: VVal) -> Object {
        let typ : String = v.at(0).unwrap_or(VVal::Nul).s_raw();
        match &typ[..] {
            "ship"   => Object::Ship(Rc::new(RefCell::new(Ship::deserialize(self, v)))),
            "system" => Object::System(Rc::new(RefCell::new(System::deserialize(self, v)))),
            _ => Object::None,
        }
    }

    pub fn set_object_at(&mut self, idx: usize, o: Object) {
        println!("SET OBJ {} = {:?}", idx, o);
        self.objects[idx] = o;
    }

    pub fn deserialize(&mut self, s: VVal) {
        self.objects = std::vec::Vec::new();
        self.tick_time_ms = 0.0;

        self.objects.resize(
            s.at(0).unwrap_or(VVal::Int(0)).i() as usize,
            Object::None);

        if let VVal::Lst(m) = s.at(1).unwrap_or(VVal::Nul) {
            for v in m.borrow().iter() {
                let o = self.vval_to_object(v.clone());
                match o {
                    Object::None => (),
                    _ => self.set_object_at(o.id(), o),
                }
            }
        }
    }

    pub fn update(&mut self, dt: f64, er: &mut EventRouter) {
        self.tick_time_ms += dt;
        //d// println!("UPD: {} {}", dt, self.tick_time_ms);
        while self.tick_time_ms > 25.0 {
            self.tick(er);
            self.tick_time_ms = self.tick_time_ms - 25.0;
        }
    }

    pub fn tick(&mut self, er: &mut EventRouter) {
        self.tick_count += 1;
        if self.tick_count > TICK_RES {
            self.tick_count = 0;
            er.emit("tick".to_string(), VVal::Nul);
        }

        for o in self.objects.iter() {
            match o {
                Object::Ship(s)   => s.borrow_mut().tick(er),
                Object::System(s) => s.borrow_mut().tick(er),
                _ => (),
            }
        }
    }

    pub fn all_entities_need_redraw(&mut self) {
        for o in self.objects.iter_mut() {
            match o {
                Object::Entity(e) => { e.borrow_mut().does_need_redraw(); },
                _ => (),
            }
        }
    }

    pub fn add_entity(&mut self, mut e: Entity) -> Rc<RefCell<Entity>> {
        e.set_id(self.objects.len());
        let r = Rc::new(RefCell::new(e));
        self.objects.push(Object::Entity(r.clone()));
        r
    }

    pub fn add_ship(&mut self, mut s: Ship) -> Rc<RefCell<Ship>> {
        s.set_id(self.objects.len());
        let r = Rc::new(RefCell::new(s));
        self.objects.push(Object::Ship(r.clone()));
        r
    }

    pub fn add_system(&mut self, mut s: System) -> Rc<RefCell<System>> {
        s.set_id(self.objects.len());
        let r = Rc::new(RefCell::new(s));
        self.objects.push(Object::System(r.clone()));
        r
    }

    pub fn get(&self, id: ObjectID) -> Option<Object> {
        if let Some(o) = self.objects.get(id) {
            match o {
                Object::None      => None,
                Object::Entity(_) => Some(o.clone()),
                Object::System(_) => Some(o.clone()),
                Object::Ship(_)   => Some(o.clone()),
            }
        } else {
            None
        }
    }
}

pub struct EventRouter {
    callbacks: std::collections::HashMap<String, std::vec::Vec<Rc<EventCallback>>>,
    event_queue: std::vec::Vec<(String, VVal)>,
}

impl EventRouter {
    pub fn new() -> Self {
        EventRouter {
            event_queue: std::vec::Vec::new(),
            callbacks:   std::collections::HashMap::new(),
        }
    }

    pub fn reg_cb<F>(&mut self, ev: String, f: F)
        where F: 'static + Fn(&Rc<RefCell<GameState>>, VVal) {

        if let Some(cbs) = self.callbacks.get_mut(&ev) {
            cbs.push(Rc::new(f));
        } else {
            let mut cbs : std::vec::Vec<Rc<EventCallback>> = std::vec::Vec::new();
            cbs.push(Rc::new(f));
            self.callbacks.insert(ev, cbs);
        }
    }

    pub fn emit(&mut self, ev: String, args: VVal) {
        if self.callbacks.get(&ev).is_none() {
            let a2 = VVal::vec();
            a2.push(VVal::new_str_mv(ev));
            a2.push(args);
            self.event_queue.push(("*".to_string(), a2));
        } else {
            self.event_queue.push((ev, args));
        }
    }

    pub fn get_events(&mut self, vec: &mut Vec<(Rc<EventCallback>, VVal)>) {
        while !self.event_queue.is_empty() {
            let ev = self.event_queue.pop().unwrap();
            if let Some(cbs) = self.callbacks.get_mut(&ev.0) {
                for c in cbs.iter() {
                    vec.push((c.clone(), ev.1.clone()));
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Hash, Copy)]
pub enum FontSize {
    Normal,
    Small,
}

pub trait GamePainter {
    fn push_offs(&mut self, xo: i32, yo: i32);
    fn push_add_offs(&mut self, xo: i32, yo: i32);

    fn declare_cache_draw(&mut self, xo: i32, yo: i32, w: u32, h: u32, id: usize, repaint: bool);
    fn done_cache_draw(&mut self);

    fn pop_offs(&mut self);
    fn get_screen_pos(&self, xo: i32, yo: i32) -> (i32, i32);
    fn disable_clip_rect(&mut self);
    fn set_clip_rect(&mut self, xo: i32, yo: i32, w: u32, h: u32);
    fn draw_rect(&mut self, xo: i32, yo: i32, w: u32, h: u32,
                 color: (u8, u8, u8, u8));
    fn draw_rect_filled(&mut self, xo: i32, yo: i32, w: u32, h: u32,
                        color: (u8, u8, u8, u8));
    fn draw_texture(&mut self, idx: usize, xo: i32, yo: i32, w: u32, h: u32);
    fn draw_dot(&mut self, xo: i32, yo: i32, r: u32, color: (u8, u8, u8, u8));
    fn draw_circle(&mut self, xo: i32, yo: i32, r: u32, color: (u8, u8, u8, u8));
    fn draw_line(&mut self, xo: i32, yo: i32, x2o: i32, y2o: i32, t: u32,
                 color: (u8, u8, u8, u8));
    fn text_size(&mut self, txt: &str, fs: FontSize) -> (u32, u32);
    fn texture_crop(&mut self, idx: usize, xo: i32, yo: i32, w: u32, h: u32);
    fn texture(&mut self, idx: usize, xo: i32, yo: i32, centered: bool);
    fn texture_size(&mut self, idx: usize) -> (u32, u32);
    fn draw_text(&mut self, xo: i32, yo: i32, max_w: u32,
                 fg: (u8, u8, u8, u8),
                 bg: Option<(u8, u8, u8, u8)>,
                 align: i32,
                 txt: &str,
                 fs: FontSize);
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum SystemObject {
    Station,
    AsteroidField,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Course {
    pub from: (i32, i32),
    pub to:   (i32, i32),
}

impl Course {
    pub fn new(x_from: i32, y_from: i32, x_to: i32, y_to: i32) -> Self {
        Course {
            from: (x_from, y_from),
            to: (x_to, y_to),
        }
    }

    pub fn rotation_quadrant(&self) -> i32 {
        let v = Vector2D::new(
            (self.to.0 - self.from.0) as f32,
            (self.to.1 - self.from.1) as f32).normalise();
        let ang = v.angle();
        let ang = if ang < 0.0 { ang + 2.0 * std::f32::consts::PI } else { ang };
        let a = ang / (2.0 * std::f32::consts::PI);
        (a * 8.0).round() as i32
    }

    pub fn interpolate(&self, v: f64) -> (i32, i32) {
        let xd = ((self.to.0 as f64 * v) + (self.from.0 as f64 * (1.0 - v))) as i32;
        let yd = ((self.to.1 as f64 * v) + (self.from.1 as f64 * (1.0 - v))) as i32;
        (xd, yd)
    }

    pub fn distance(&self) -> i32 {
        ((  (self.from.0 - self.to.0).pow(2)
         + (self.from.1 - self.to.1).pow(2)) as f64).sqrt() as i32
    }
}

// 10 ticks == 1 second
#[derive(Debug, Clone)]
pub struct Ship {
    pub id:             ObjectID,
    pub system:         ObjectID,
    pub name:           String,
    pub notify_txt:     String,
    pub pos:            (i32, i32),
    pub speed_t:        i32, // 100:1 => speed_t * 10 is speed per second
    pub state:          VVal,
    course_progress:    i32, // 100:1
    course:             Option<Course>,
    tick_count:         i32,
}

impl Ship {
    pub fn new(name: String) -> Self {
        Ship {
            name,
            course_progress:    0,
            speed_t:            1000,
            course:             None,
            pos:                (0, 0),
            system:             0,
            id:                 0,
            state:              VVal::map(),
            tick_count:         0,
            notify_txt:         String::from(""),
        }
    }

    pub fn deserialize(_or: &mut ObjectRegistry, v: VVal) -> Self {
        let mut s = Self::new("".to_string());
        s.id                = v.at(2).unwrap_or(VVal::Int(0)).i() as ObjectID;
        s.system            = v.at(3).unwrap_or(VVal::Int(0)).i() as ObjectID;
        s.name              = v.at(4).unwrap_or(VVal::new_str("")).s_raw();
        s.pos.0             = v.at(5).unwrap_or(VVal::Int(0)).i() as i32;
        s.pos.1             = v.at(6).unwrap_or(VVal::Int(0)).i() as i32;
        s.speed_t           = v.at(7).unwrap_or(VVal::Int(0)).i() as i32;
        s.course_progress   = v.at(8).unwrap_or(VVal::Int(0)).i() as i32;
        if let Some(VVal::Lst(l)) = v.at(9) {
            let mut c = Course::new(0, 0, 0, 0);
            c.from.0 = l.borrow().get(0).unwrap().i() as i32;
            c.from.1 = l.borrow().get(1).unwrap().i() as i32;
            c.to.0   = l.borrow().get(2).unwrap().i() as i32;
            c.to.1   = l.borrow().get(3).unwrap().i() as i32;
            s.course = Some(c);
        } else {
            s.course = None;
        }
        s.tick_count        = v.at(10).unwrap_or(VVal::Int(0)).i() as i32;
        s.state             = v.at(11).unwrap_or(VVal::Nul);
        s
    }

    pub fn serialize(&self) -> VVal {
        let v = VVal::vec();
        v.push(VVal::new_str("ship"));
        v.push(VVal::Int(0)); // version
        v.push(VVal::Int(self.id      as i64));
        v.push(VVal::Int(self.system  as i64));
        v.push(VVal::new_str(&self.name));
        v.push(VVal::Int(self.pos.0   as i64));
        v.push(VVal::Int(self.pos.1   as i64));
        v.push(VVal::Int(self.speed_t as i64));
        v.push(VVal::Int(self.course_progress as i64));
        if let Some(c) = self.course {
            let cv = VVal::vec();
            cv.push(VVal::Int(c.from.0 as i64));
            cv.push(VVal::Int(c.from.1 as i64));
            cv.push(VVal::Int(c.to.0 as i64));
            cv.push(VVal::Int(c.to.1 as i64));
            v.push(cv);
        } else {
            v.push(VVal::Nul);
        }
        v.push(VVal::Int(self.tick_count  as i64));
        v.push(self.state.clone());
        v
    }

    pub fn set_id(&mut self, id: ObjectID) { self.id = id; }

    pub fn set_system(&mut self, sys_id: ObjectID) { self.system = sys_id; }

    pub fn set_notification(&mut self, not: String) { self.notify_txt = not; }

    pub fn set_course_to(&mut self, x: i32, y: i32) {
        self.course = Some(Course::new(self.pos.0, self.pos.1, x, y));
        self.course_progress = 0;
    }

    pub fn tick(&mut self, er: &mut EventRouter) {
        let mut tick_now = false;

        if let Some(_) = self.course {
            let started = self.course_progress == 0;
            self.course_progress += self.speed_t;
            let d = self.course.unwrap().distance() * 100;
            if self.course_progress >= d {
                self.pos = self.course.unwrap().to;
                self.course = None;
                self.state.set_map_key(
                    "_state".to_string(), VVal::new_str("arrived"));
                tick_now = true;

            } else {
                if started {
                    tick_now = true;
                    self.state.set_map_key(
                        "_state".to_string(), VVal::new_str("started"));
                } else {
                    self.state.set_map_key(
                        "_state".to_string(), VVal::new_str("flying"));
                }

                self.pos = self.course.unwrap().interpolate(
                    self.course_progress as f64 / d as f64);
            }

//            println!("SHIP: pos={:?} dis={} cp={}", self.pos, d, self.course_progress);
        } else {
            self.state.set_map_key(
                "_state".to_string(), VVal::new_str("stopped"));
        }

        self.tick_count += 1;
        if self.tick_count > TICK_RES {
            self.tick_count = 0;
            tick_now = true;
        }

        if tick_now {
            er.emit("ship_tick".to_string(),
                VVal::Int(self.id as i64));
        }
    }

    pub fn draw<P>(&mut self, p: &mut P) where P: GamePainter {
        let x = sys2screen(self.pos.0);
        let y = sys2screen(self.pos.1);

        let a = 
            if let Some(c) = self.course {
                c.rotation_quadrant()
            } else {
                1
            };

        if let Some(c) = self.course {
            p.draw_line(
                x, y, sys2screen(c.to.0), sys2screen(c.to.1),
                1, (190, 190, 190, 255));
        }
        p.texture(3 + ((8 - a) as usize + 3) % 8, x, y, true);

        if self.notify_txt.len() > 0 {
            p.draw_text(
                x - 100, y + 10, 200,
                (255, 0, 255, 255), None,
                0, &self.notify_txt, FontSize::Normal);
        }
    }
}

#[derive(Debug, Clone)]
pub struct Entity {
    pub id:             ObjectID,
    pub typ:            SystemObject,
    pub x:              i32,
    pub y:              i32,
    pub state:          VVal,
    pub name:           String,
    draw_pos:           (i32, i32),
    is_highlighted:     bool,
    redraw:             bool,
}

impl Entity {
    pub fn new(typ: SystemObject) -> Self {
        Entity {
            typ,
            id:             0,
            draw_pos:       (0, 0),
            x:              0,
            y:              0,
            state:          VVal::map(),
            is_highlighted: false,
            redraw:         true,
            name:           String::from(""),
        }
    }

    pub fn does_need_redraw(&mut self) { self.redraw = true; }

    pub fn deserialize(_or: &mut ObjectRegistry, v: VVal) -> Self {
        let mut s = Self::new(SystemObject::Station);
        s.id            = v.at(2).unwrap_or(VVal::Int(0)).i() as ObjectID;
        s.typ           = match v.at(3).unwrap_or(VVal::Int(0)).i() {
                              0 => SystemObject::Station,
                              1 => SystemObject::AsteroidField,
                              _ => SystemObject::Station,
                          };
        s.x             = v.at(4).unwrap_or(VVal::Int(0)).i() as i32;
        s.y             = v.at(5).unwrap_or(VVal::Int(0)).i() as i32;
        s.name          = v.at(6).unwrap_or(VVal::new_str("")).s_raw();
        s.state         = v.at(7).unwrap_or(VVal::Nul);
        s
    }

    pub fn serialize(&self) -> VVal {
        let v = VVal::vec();
        v.push(VVal::new_str("entity"));
        v.push(VVal::Int(0)); // version
        v.push(VVal::Int(self.id  as i64));
        v.push(VVal::Int(self.typ as i64));
        v.push(VVal::Int(self.x   as i64));
        v.push(VVal::Int(self.y   as i64));
        v.push(VVal::new_str(&self.name));
        v.push(self.state.clone());
        v
    }

    pub fn set_id(&mut self, id: ObjectID) { self.id = id; }

    fn draw<P>(&mut self, p: &mut P) where P: GamePainter {
        let t_id =
            match self.typ {
                SystemObject::Station       => 2,
                SystemObject::AsteroidField => 0,
            };
        let q = p.texture_size(t_id);

        // TODO: Offset rendering completely for fitting text.
        let tw : i32 = (q.0 / 2) as i32;

        p.declare_cache_draw(-tw, -tw, 256, 156, self.id as usize, self.redraw);
        if self.redraw {
            self.redraw = false;
            p.texture(t_id, 0, 0, false);
            p.draw_text(1, tw + (tw / 2) + 20 + 2, 2 * tw as u32, (0, 0, 0, 255),       None, 0, &self.name, FontSize::Normal);
            p.draw_text(0, tw + (tw / 2) + 20,     2 * tw as u32, (255, 255, 255, 255), None, 0, &self.name, FontSize::Normal);
            if self.is_highlighted {
                p.draw_circle(tw, tw, 30, (255, 0, 0, 255));
            }
        }
        p.done_cache_draw();
        self.draw_pos = p.get_screen_pos(0, 0);
    }

    fn set_highlight(&mut self, h: bool) {
        if self.is_highlighted != h {
            // TODO: FIXME: The entity is redrawn all the time, because
            //              try_highlight_entity_close_to does set all highlights
            //              to `false` before checking which entity is highlighted.
            self.redraw = true;
        }
        self.is_highlighted = h;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MouseScreenSystemPos {
    screen_x0:  i32,
    screen_x1:  i32,
    screen_y0:  i32,
    screen_y1:  i32,
    system_x0:  i32,
    system_x1:  i32,
    system_y0:  i32,
    system_y1:  i32,
}

impl MouseScreenSystemPos {
    pub fn new() -> Self {
        Self {
            screen_x0: 0,
            screen_x1: 0,
            screen_y0: 0,
            screen_y1: 0,
            system_x0: 0,
            system_x1: 0,
            system_y0: 0,
            system_y1: 0,
        }
    }

    pub fn mouse2system(&self, x: i32, y: i32) -> Option<(i32, i32)> {
        if !(   x >= self.screen_x0
             && x <= self.screen_x1
             && y >= self.screen_y0
             && y <= self.screen_y1) {
            return None;
        }

        println!("CLICK {}:{} => {:?}", x, y, self);

        let scr_w = self.screen_x1 - self.screen_x0;
        let scr_h = self.screen_y1 - self.screen_y0;
        let sys_w = self.system_x1 - self.system_x0;
        let sys_h = self.system_y1 - self.system_y0;

        if scr_w == 0 { return None; }
        if scr_h == 0 { return None; }

        let xsys = self.system_x0 + ((x - self.screen_x0) * sys_w) / scr_w;
        let ysys = self.system_y0 + ((y - self.screen_y0) * sys_h) / scr_h;

        Some((xsys, ysys))
    }
}

#[derive(Debug, Clone)]
pub struct System {
    pub id:     ObjectID,
    pub x:      i32,
    pub y:      i32,
    pub state:  VVal,
    objects:    std::vec::Vec<Rc<RefCell<Entity>>>,
    tick_count: i32,
}

impl System {
    pub fn new(x: i32, y: i32) -> Self {
        System {
            id: 0,
            x,
            y,
            objects: std::vec::Vec::new(),
            tick_count: 0,
            state: VVal::map()
        }
    }

    pub fn deserialize(or: &mut ObjectRegistry, v: VVal) -> Self {
        let mut s = Self::new(0, 0);
        s.id            = v.at(2).unwrap_or(VVal::Int(0)).i() as ObjectID;
        s.x             = v.at(3).unwrap_or(VVal::Int(0)).i() as i32;
        s.y             = v.at(4).unwrap_or(VVal::Int(0)).i() as i32;
        s.tick_count    = v.at(5).unwrap_or(VVal::Int(0)).i() as i32;
        s.state         = v.at(6).unwrap_or(VVal::Nul);
        if let Some(VVal::Lst(l)) = v.at(7) {
            for o in l.borrow().iter() {
                let e = Rc::new(RefCell::new(Entity::deserialize(or, o.clone())));
                let id = e.borrow().id;
                or.set_object_at(id, Object::Entity(e.clone()));
                s.objects.push(e);
            }
        }
        s
    }

    pub fn serialize(&self) -> VVal {
        let v = VVal::vec();
        v.push(VVal::new_str("system"));
        v.push(VVal::Int(0)); // version
        v.push(VVal::Int(self.id          as i64));
        v.push(VVal::Int(self.x           as i64));
        v.push(VVal::Int(self.y           as i64));
        v.push(VVal::Int(self.tick_count  as i64));
        v.push(self.state.clone());
        let o = VVal::vec();
        for obj in self.objects.iter() {
            o.push(obj.borrow().serialize());
        }
        v.push(o);
        v
    }

    pub fn set_id(&mut self, id: ObjectID) { self.id = id; }

    pub fn tick(&mut self, er: &mut EventRouter) {
        self.tick_count += 1;
        if self.tick_count > TICK_RES {
            self.tick_count = 0;
            er.emit("system_tick".to_string(), VVal::Int(self.id as i64));
        }
    }

    pub fn add(&mut self, x: i32, y: i32, e: Rc<RefCell<Entity>>) {
        e.borrow_mut().x = x;
        e.borrow_mut().y = y;
        self.objects.push(e);
    }

    pub fn clear_entity_highlights(&mut self) {
        for e in self.objects.iter_mut() {
            e.borrow_mut().set_highlight(false);
        }
    }

    pub fn try_highlight_entity_close_to(&mut self, x_screen: i32, y_screen: i32) {
        self.clear_entity_highlights();
        if let Some(e) = self.get_entity_close_to_screen(x_screen, y_screen) {
            e.borrow_mut().set_highlight(true);
        }
    }

    pub fn get_entity_close_to(&mut self, x: i32, y: i32) -> Option<Rc<RefCell<Entity>>> {
        let mut closest_i : i32 = -1;
        let mut last_dist : i32 = 99999;

        for (i, ent) in self.objects.iter().enumerate() {
            let ent_r = ent.borrow_mut();
            let d : i32 = (ent_r.x - x).pow(2)
                        + (ent_r.y - y).pow(2);
            if d < last_dist {
                last_dist = d;
                closest_i = i as i32;
            }
        }

        if last_dist < 2_i32.pow(2) {
            return self.objects.get(closest_i as usize).cloned();
        }
        return None;
    }

    pub fn get_entity_close_to_screen(&mut self, x_screen: i32, y_screen: i32) -> Option<Rc<RefCell<Entity>>> {
        let mut closest_i : i32 = -1;
        let mut last_dist : i32 = 99999;

        for (i, ent) in self.objects.iter().enumerate() {
            let ent_r = ent.borrow_mut();
            let d : i32 = (ent_r.draw_pos.0 - x_screen).pow(2)
                        + (ent_r.draw_pos.1 - y_screen).pow(2);
            if d < last_dist {
                last_dist = d;
                closest_i = i as i32;
            }
        }

        if last_dist < 20_i32.pow(2) {
            return self.objects.get(closest_i as usize).cloned();
        }
        return None;
    }

    pub fn draw<P>(&mut self, ship: &mut Ship, _scroll: &(i32, i32), p: &mut P) -> MouseScreenSystemPos where P: GamePainter {
        let mut mssp = MouseScreenSystemPos {
            screen_x0:  0,
            screen_x1:  1280,
            screen_y0:  0,
            screen_y1:  0,
            system_x0:  0,
            system_x1:  0,
            system_y0:  0,
            system_y1:  0,
        };

        p.set_clip_rect(0, 0, 1280, 600);

//        p.push_add_offs(0, -(scroll.1 * (1280 - 600)) / 1000);
//
        let tex_y_size = 1800;
        let tex_y_pad  = (tex_y_size - 1280) / 4;

        let scroll =
            if ship.system == self.id {
                let y : i32 = sys2screen(ship.pos.1);
                let mut scroll = y - (600 / 2);
                if scroll < 0 { scroll = 0; }
                if scroll > (1280 - 600) { scroll = 1280 - 600; }
                scroll
            } else {
                0
            };

        let screen_0 = p.get_screen_pos(0, 0);
        let screen_1 = p.get_screen_pos(1280, 600);
        mssp.screen_x0 = screen_0.0;
        mssp.screen_y0 = screen_0.1;
        mssp.screen_x1 = screen_1.0;
        mssp.screen_y1 = screen_1.1;
        mssp.system_x0 = screen2sys(0);
        mssp.system_y0 = screen2sys(scroll);
        mssp.system_x1 = screen2sys(1280);
        mssp.system_y1 = screen2sys(scroll + 600);

        p.push_add_offs(0, -scroll);
        p.texture_crop(1, 0, -(scroll * tex_y_pad) / (1280 - 300), 1280, 1800);

        p.draw_line(0, 0, 1280, 0, 10,       (255, 0, 0, 255));
        p.draw_line(0, 0, 0, 1280, 10,       (255, 0, 0, 255));
        p.draw_line(1280, 0, 1280, 1280, 10, (255, 0, 0, 255));
        p.draw_line(0, 1280, 1280, 1280, 10, (255, 0, 0, 255));

        for ent in self.objects.iter_mut() {
            p.push_add_offs(
                sys2screen(ent.borrow().x),
                sys2screen(ent.borrow().y));
            ent.borrow_mut().draw(p);
            p.pop_offs();
        }

        if ship.system == self.id {
            ship.draw(p);
        }
        p.pop_offs();
        p.disable_clip_rect();

        mssp
    }
}
