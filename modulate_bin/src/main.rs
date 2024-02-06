use modulate_lib::ModManager;

fn main() {
    pretty_env_logger::init();

    let mut manager = ModManager::new("./examples/working_dir", "./examples/bak_dir").unwrap();

    let mod1 = manager.add_mod("./examples/mod1").unwrap();
    let _mod2 = manager.add_mod("./examples/mod2").unwrap();

    manager.activate_mod(mod1).unwrap();

    manager.deploy_mods();

    manager.deactivate_mod(mod1).unwrap();

    manager.deploy_mods();
}
