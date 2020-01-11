use crate::state::SSCG;
#[macro_use]
use gdnative::*;
use crate::voxeltree::*;
use crate::gd_voxel_impl::*;
use wlambda::VVal;

#[derive(NativeClass)]
#[inherit(gdnative::Spatial)]
pub struct VoxStruct {
    meshes:           std::vec::Vec<MeshInstance>,
    collision_shapes: std::vec::Vec<(StaticBody, i64)>,
    vol:              Vol<u8>,
    octrees:          std::vec::Vec<Octree<u8>>,
    color_map:        ColorMap,

    cursor:         [u16; 3],
}

unsafe impl Send for VoxStruct { }

const VOL_SIZE    : usize = 128;
const SUBVOL_SIZE : usize = 16;
const SUBVOLS     : usize = VOL_SIZE / SUBVOL_SIZE;

fn vval2colors(clr: VVal) -> ColorMap {
    let mut colors = [[0.0; 3]; 256];
    use sscg::wlambda_api::color_hex24tpl;
    for (i, c) in clr.iter().enumerate() {
        let tpl = color_hex24tpl(&c.s_raw());
        colors[i] = [
            tpl.0 as f32 / 255.0,
            tpl.1 as f32 / 255.0,
            tpl.2 as f32 / 255.0,
        ];
    }
    ColorMap::new_from(colors)
}

#[methods]
impl VoxStruct {
    fn _init(_owner: Spatial) -> Self {
        Self {
            meshes:           vec![],
            collision_shapes: vec![],
            vol:              Vol::new(VOL_SIZE),
            octrees:          vec![],
            color_map:        ColorMap::new_gray(),
            cursor:           [0, 0, 0],
        }
    }

    #[export]
    fn on_wlambda_init(&mut self, mut owner: Spatial) {
        println!("FOFO");
        let d = std::time::Instant::now();
        let (sysid, entid) = self.parent_info(&mut owner);
        lock_sscg!(sscg);
        let ret = sscg.call_cb("on_draw_voxel_structure", &vec![sysid, entid]);
        if !ret.is_none() {
            sscg.vox_painters
                .borrow()[ret.v_i(0) as usize]
                .borrow()
                .write_into_u8_vol(ret.v_i(1) as usize, &mut self.vol);
            if !ret.v_(2).is_none() {
                self.color_map = vval2colors(ret.v_(2));
            }
            self.load_vol(owner);
            println!("Reloaded voxel volume, took {} ms", d.elapsed().as_millis());
        }
    }

    #[export]
    fn _ready(&mut self, mut owner: Spatial) {
        use wlambda::util::*;
        self.vol.fill(0, 0, 0, VOL_SIZE as u16, VOL_SIZE as u16, VOL_SIZE as u16, 100.into());
//        let mut sm = SplitMix64::new(3489492);
//        for z in 0..VOL_SIZE {
//            for y in 0..VOL_SIZE {
//                for x in 0..VOL_SIZE {
//                    if u64_to_open01(sm.next_u64()) > 0.01 {
//                        self.vol.set(x as u16, y as u16, z as u16, 0.into());
//                    } else {
//                        let color = (u64_to_open01(sm.next_u64()) * 256.0) as u8;
//                        self.vol.set(x as u16, y as u16, z as u16, color.into());
//                    }
//                }
//            }
//        }

        println!("filled...");

//        let vv = self.vol.serialize();
//        println!("BYTES: {}", vv.len());
//        self.vol.deserialize(&vv);

        let voxel_material =
            ResourceLoader::godot_singleton().load(
                GodotString::from_str("res://scenes/entities/materials/voxel_material.tres"),
                GodotString::from_str("ShaderMaterial"),
                false).unwrap().cast::<Material>().unwrap();

        let mut i = 0;
        for z in 0..SUBVOLS {
            for y in 0..SUBVOLS {
                for x in 0..SUBVOLS {
                    self.meshes.push(MeshInstance::new());

                    let mut sb = StaticBody::new();

                    unsafe {
                        self.meshes[i].set_material_override(Some(voxel_material.clone()));
                        let mut t = self.meshes[i].get_transform();
                        t.origin.x = (x * SUBVOL_SIZE) as f32;
                        t.origin.y = (y * SUBVOL_SIZE) as f32;
                        t.origin.z = (z * SUBVOL_SIZE) as f32;
                        self.meshes[i].set_transform(t);
                        self.meshes[i].set_layer_mask_bit(0, false);
                        self.meshes[i].set_layer_mask_bit(1, true);
                        sb.set_transform(t);

                        owner.add_child(self.meshes[i].cast::<Node>(), false);

                        let sb_obj = Object::from_sys(sb.cast::<Object>().unwrap().to_sys());
                        let id = sb.create_shape_owner(Some(sb_obj));
                        self.collision_shapes.push((sb, id));

                        owner.add_child(sb.cast::<Node>(), false);
                    }

                    self.octrees.push(Octree::new_from_size(SUBVOL_SIZE));

                    i += 1;
                }
            }
        }
        println!("initialized godot objects");

        self.load_vol(owner);
        println!("loaded godot objects");
    }

    fn serialize_vol(&mut self) -> Vec<u8> {
        for z in 0..VOL_SIZE {
            let iz  = z / SUBVOL_SIZE;
            let izi = z % SUBVOL_SIZE;

            for y in 0..VOL_SIZE {
                let iy  = y / SUBVOL_SIZE;
                let iyi = y % SUBVOL_SIZE;

                for x in 0..VOL_SIZE {
                    let ix  = x / SUBVOL_SIZE;
                    let ixi = x % SUBVOL_SIZE;

                    self.vol.set(x as u16, y as u16, z as u16,
                        self.octrees[
                              iz * (SUBVOLS * SUBVOLS)
                            + iy * SUBVOLS
                            + ix].get(
                                ixi as u16,
                                iyi as u16,
                                izi as u16));
                }
            }
        }

        self.vol.serialize()
    }

    #[export]
    fn load_vol(&mut self, mut _owner: Spatial) {
        for z in 0..VOL_SIZE {
            let iz  = z / SUBVOL_SIZE;
            let izi = z % SUBVOL_SIZE;

            for y in 0..VOL_SIZE {
                let iy  = y / SUBVOL_SIZE;
                let iyi = (SUBVOL_SIZE - 1) - (y % SUBVOL_SIZE);

                for x in 0..VOL_SIZE {
                    let ix  = x / SUBVOL_SIZE;
                    let ixi = x % SUBVOL_SIZE;

                    self.octrees[
                          iz * (SUBVOLS * SUBVOLS)
                        + iy * SUBVOLS
                        + ix].set(
                            ixi as u16,
                            iyi as u16,
                            izi as u16,
                            *self.vol.at(Pos {
                              x: x as u16,
                              y: y as u16,
                              z: z as u16
                            }));
                }
            }
        }

        for z in 0..SUBVOLS {
            for y in 0..SUBVOLS {
                for x in 0..SUBVOLS {
                    self.reload_at(
                        x * SUBVOL_SIZE,
                        y * SUBVOL_SIZE,
                        z * SUBVOL_SIZE);
                }
            }
        }
    }

    fn get_octree_at(&mut self, x: usize, y: usize, z: usize) -> (&mut Octree<u8>, [u16; 3]) {
        let iz  = z / SUBVOL_SIZE;
        let izi = z % SUBVOL_SIZE;

        let iy  = y / SUBVOL_SIZE;
        let iyi = y % SUBVOL_SIZE;

        let ix  = x / SUBVOL_SIZE;
        let ixi = x % SUBVOL_SIZE;

        let sub_idx =
              iz * (SUBVOLS * SUBVOLS)
            + iy * SUBVOLS
            + ix;

        (&mut self.octrees[sub_idx], [ixi as u16, iyi as u16, izi as u16])
    }

    #[export]
    fn mine_info_at_cursor(&mut self, mut _owner: Spatial) -> Variant {
        let (ot, pos) =
            self.get_octree_at(
                self.cursor[0] as usize,
                self.cursor[1] as usize,
                self.cursor[2] as usize);
        let v = ot.get_inv_y(pos[0], pos[1], pos[2]);
        let mut dict = gdnative::Dictionary::new();
        dict.set(&Variant::from_str("material"), &Variant::from_i64(v.color as i64));
        dict.set(&Variant::from_str("time"),     &Variant::from_f64(1.2));
        dict.set(&Variant::from_str("x"),        &Variant::from_i64(self.cursor[0] as i64));
        dict.set(&Variant::from_str("y"),        &Variant::from_i64(self.cursor[1] as i64));
        dict.set(&Variant::from_str("z"),        &Variant::from_i64(self.cursor[2] as i64));
        Variant::from_dictionary(&dict)
    }

    #[export]
    fn looking_at(&mut self, owner: Spatial, x: f64, y: f64, z: f64) -> bool {
        unsafe {
            let mut c =
                owner.get_child(0)
                     .and_then(|n| n.cast::<Spatial>())
                     .unwrap();
            let mut c2 =
                owner.get_child(1)
                     .and_then(|n| n.cast::<Spatial>())
                     .unwrap();

            let mut t = c.get_transform();
            t.origin.x = x.floor() as f32 + 0.5;
            t.origin.y = y.floor() as f32 + 0.5;
            t.origin.z = z.floor() as f32 + 0.5;
            c.set_transform(t);
            c2.set_transform(t);

            self.cursor = [
                t.origin.x as u16,
                t.origin.y as u16,
                t.origin.z as u16,
            ];

            let (ot, pos) =
                self.get_octree_at(
                    self.cursor[0] as usize,
                    self.cursor[1] as usize,
                    self.cursor[2] as usize);
            let v = ot.get_inv_y(pos[0], pos[1], pos[2]);
            v.color != 0

//            {
//                let (ot, pos) =
//                    self.get_octree_at(
//                        self.cursor[0] as usize,
//                        self.cursor[1] as usize,
//                        self.cursor[2] as usize);
//                let v = ot.get_inv_y(pos[0], pos[1], pos[2]);
//                if v.color == 0 {
//                    c.hide();
//                } else {
//                    c.show();
//                }
//            }
        }
    }

    #[export]
    fn set_marker_status(&mut self, owner: Spatial, show: bool, mining: bool) {
        unsafe {
            let mut looking_cursor =
                owner.get_child(0)
                     .and_then(|n| n.cast::<Spatial>())
                     .unwrap();
            let mut mining_cursor =
                owner.get_child(1)
                     .and_then(|n| n.cast::<Spatial>())
                     .unwrap();

            if show {
                if mining {
                    looking_cursor.hide();
                    mining_cursor.show();
                } else {
                    looking_cursor.show();
                    mining_cursor.hide();
                }
            } else {
                looking_cursor.hide();
                mining_cursor.hide();
            }
        }
    }

    fn parent_info(&self, owner: &mut Spatial) -> (VVal, VVal) {
        unsafe {
            let sysid =
                owner.get_parent().unwrap().get(
                    GodotString::from_str("system_id")).to_i64();
            let entid =
                owner.get_parent().unwrap().get(
                    GodotString::from_str("entity_id")).to_i64();
            (VVal::Int(sysid), VVal::Int(entid))
        }
    }

    #[export]
    fn spawn_mine_pop_at_cursor(&mut self, owner: Spatial, color: u8) {
        unsafe {
            let mut part =
                owner.get_child(2)
                     .and_then(|n| n.cast::<Particles>())
                     .unwrap();

            let mut m = part.get_material_override().unwrap()
                            .cast::<SpatialMaterial>().unwrap();
            let cm = self.color_map;
            let clr = cm.map(color);
            m.set_albedo(clr);
            m.set_emission(clr);
            part.set_material_override(m.cast::<Material>());

            let mut t = part.get_transform();
            t.origin.x = self.cursor[0] as f32 + 0.5;
            t.origin.y = self.cursor[1] as f32 + 0.5;
            t.origin.z = self.cursor[2] as f32 + 0.5;
            part.set_transform(t);

            part.show();
            part.set_one_shot(true);
            part.set_emitting(true);
            part.restart();
        }
    }

    #[export]
    fn mine_status(&mut self, mut owner: Spatial, started: bool) -> bool {
        let (ot, pos) =
            self.get_octree_at(
                self.cursor[0] as usize,
                self.cursor[1] as usize,
                self.cursor[2] as usize);
        let m = ot.get_inv_y(pos[0], pos[1], pos[2]);

        let (sysid, entid) = self.parent_info(&mut owner);
        lock_sscg!(sscg);
        let ret = sscg.call_cb(
            "on_mine",
            &vec![sysid, entid,
                  VVal::Bol(started),
                  VVal::Int(m.color as i64),
                  VVal::Int(self.cursor[0] as i64),
                  VVal::Int(self.cursor[1] as i64),
                  VVal::Int(self.cursor[2] as i64),
                  ]);

        ret.b()
    }

    #[export]
    fn mine_at_cursor(&mut self, mut owner: Spatial) -> bool {
        let (ot, pos) =
            self.get_octree_at(
                self.cursor[0] as usize,
                self.cursor[1] as usize,
                self.cursor[2] as usize);
        let m = ot.get_inv_y(pos[0], pos[1], pos[2]);

        if m.color != 0 {
            ot.set_inv_y(pos[0], pos[1], pos[2], 0.into());
            self.reload_at(
                self.cursor[0] as usize,
                self.cursor[1] as usize,
                self.cursor[2] as usize);

            lock_sscg!(sscg);
            let (sysid, entid) = self.parent_info(&mut owner);
            sscg.call_cb(
                "on_mined_voxel",
                &vec![sysid, entid,
                      VVal::Int(m.color as i64),
                      VVal::Int(self.cursor[0] as i64),
                      VVal::Int(self.cursor[1] as i64),
                      VVal::Int(self.cursor[2] as i64),
                      ]);

            self.spawn_mine_pop_at_cursor(owner, m.color);

            true
        } else {
            false
        }
    }

    #[export]
    fn looking_at_nothing(&mut self, owner: Spatial) {
        self.set_marker_status(owner, false, false);
    }

    #[export]
    fn _process(&mut self, mut _owner: Spatial, _delta: f64) {
    }

    fn reload_at(&mut self, x: usize, y: usize, z: usize) {
        let iz  = z / SUBVOL_SIZE;
        let iy  = y / SUBVOL_SIZE;
        let ix  = x / SUBVOL_SIZE;

        let sub_idx =
              iz * (SUBVOLS * SUBVOLS)
            + iy * SUBVOLS
            + ix;

        let n = self.octrees[sub_idx].recompute();

        let mut am = ArrayMesh::new();
        let mut cvshape = ConcavePolygonShape::new();
        let (mut static_body, shape_owner_idx) = self.collision_shapes[sub_idx];

        if !n.empty {
            let cm = self.color_map;

            render_octree_to_am(
                &mut am, &mut cvshape, &cm, &self.octrees[sub_idx]);
        }

        unsafe {
            self.meshes[sub_idx].set_mesh(am.cast::<Mesh>());
            let mut ssb = static_body.cast::<StaticBody>().unwrap();
            ssb.shape_owner_clear_shapes(shape_owner_idx);

            if !n.empty {
                ssb.shape_owner_add_shape(
                    shape_owner_idx,
                    cvshape.cast::<Shape>());
                self.meshes[sub_idx].show();
                static_body.show();
            } else {
                self.meshes[sub_idx].hide();
                static_body.hide();
            }
        }
    }
}
