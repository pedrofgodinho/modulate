use modulate_lib::ModManager;

fn main() {
    pretty_env_logger::init();

    let mut manager = ModManager::new("./examples/working_dir".parse().unwrap(), "./examples/bak_dir".parse().unwrap()).unwrap();

    let mod1 = manager.add_mod("./examples/mod1".into()).unwrap();
    let mod2 = manager.add_mod("./examples/mod2".into()).unwrap();

    manager.activate_mod(mod1).unwrap();
    manager.activate_mod(mod2).unwrap();

    manager.deploy_mods();

    manager.deactivate_mod(mod1).unwrap();
    manager.deactivate_mod(mod2).unwrap();

    manager.deploy_mods();
}
