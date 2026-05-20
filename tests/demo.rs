fn demo_space_wgsl() -> &'static str {
    let source = include_str!("../examples/demo/render.rs");
    let marker = "pub const SPACE_WGSL: &str = r#\"";
    let Some(start) = source.find(marker) else {
        return "";
    };
    let body_start = start + marker.len();
    let Some(body_end) = source[body_start..].find("\"#;") else {
        return "";
    };
    &source[body_start..body_start + body_end]
}

#[test]
fn demo_space_shader_parses_as_wgsl() {
    let shader = demo_space_wgsl();
    assert!(
        !shader.is_empty(),
        "demo shader source should be discoverable"
    );

    let parsed = naga::front::wgsl::parse_str(shader);
    assert!(parsed.is_ok(), "demo shader must parse: {parsed:?}");
    let Ok(module) = parsed else {
        return;
    };

    let validated = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module);
    assert!(
        validated.is_ok(),
        "demo shader must validate: {validated:?}"
    );
}
