#[allow(dead_code)]
#[path = "../examples/demo/render.rs"]
mod render;
#[allow(dead_code)]
#[path = "../examples/demo/sim.rs"]
mod sim;

#[test]
fn demo_space_shader_parses_as_wgsl() {
    let module = naga::front::wgsl::parse_str(render::SPACE_WGSL).expect("demo shader must parse");
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .expect("demo shader must validate");
}

#[test]
fn demo_seed_scene_is_nonempty() {
    let sim = sim::Simulation::new();
    assert_eq!(sim.bodies().len(), 4);
    assert!(sim.asteroids().len() >= 1_000);
}
