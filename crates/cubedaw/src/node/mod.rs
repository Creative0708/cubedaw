pub mod math;

pub fn register_cubedaw_nodes(registry: &mut cubedaw_workerlib::NodeRegistry) {
    use cubedaw_lib::ResourceKey;
    registry.register_node::<math::MathNode>(ResourceKey::new("cubedaw:math"), "Math".into());
}
