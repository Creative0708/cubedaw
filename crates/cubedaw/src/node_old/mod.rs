pub mod math;
pub mod oscillator;

pub fn register_cubedaw_nodes(registry: &mut cubedaw_lib::NodeRegistry) {
    use cubedaw_lib::ResourceKey;
    registry.register_node::<math::MathNode>(ResourceKey::new("cubedaw:math"), "Math".into());
    registry.register_node::<oscillator::OscillatorNode>(
        ResourceKey::new("cubedaw:oscillator"),
        "Oscillator".into(),
    );
}
