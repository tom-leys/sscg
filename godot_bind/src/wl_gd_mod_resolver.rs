use wlambda::{GlobalEnv, EvalContext, SymbolTable};
use gdnative::godot_print;
use gdnative::{File, GodotString};
use wlambda::compiler::{GlobalEnvRef, ModuleResolver, ModuleLoadError};
use std::rc::Rc;
use std::cell::RefCell;

/// This structure implements the ModuleResolver trait and is
/// responsible for loading modules on `!@import` for WLambda.
#[derive(Debug, Clone, Default)]
pub struct GodotModuleResolver { }

#[allow(dead_code)]
impl GodotModuleResolver {
    pub fn new() -> GodotModuleResolver {
        GodotModuleResolver { }
    }
}

impl ModuleResolver for GodotModuleResolver {
    fn resolve(&self, global: GlobalEnvRef, path: &[String], _import_file_path: Option<&str>)
        -> Result<SymbolTable, ModuleLoadError>
    {
//        println!("***** GODOT RESOLVE MODULE: {:?}", path);
        let genv = GlobalEnv::new_empty_default();
        genv.borrow_mut().import_modules_from(&*global.borrow());
        genv.borrow_mut().set_resolver(Rc::new(RefCell::new(GodotModuleResolver::new())));
        let mut ctx = EvalContext::new(genv);
        let pth = path.join("/");

        let mut f = File::new();
        let mod_path = format!("res://gamelib/{}.wl", pth.clone());
        match f.open(GodotString::from_str(&mod_path), 1)
        {
            Ok(_) => {
                let txt = f.get_as_text().to_string();
                match ctx.eval_string(&txt, &(pth.clone() + ".wl")) {
                    Err(e) => Err(ModuleLoadError::ModuleEvalError(e)),
                    Ok(_v) => Ok(ctx.get_exports()),
                }
            },
            Err(e) => {
                godot_print!("Couldn't load module: '{}': {:?}", pth, e);
                Err(ModuleLoadError::NoSuchModule(mod_path))
            },
        }
    }
}

