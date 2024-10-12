#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
enum MathNodeType {
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct MathNodeState {
    node_type: MathNodeType,
}

#[no_mangle]
fn do_math(state: MathNodeState) {
    let in1 = cubedaw_pluginlib::input::<0>();
    let in2 = cubedaw_pluginlib::input::<1>();

    let val = match state.node_type {
        MathNodeType::Add => in1 + in2,
        MathNodeType::Subtract => in1 - in2,
        MathNodeType::Multiply => in1 * in2,
        MathNodeType::Divide => in1 / in2,
    };

    cubedaw_pluginlib::output::<0>(val);
}

#[link_section = "cubedaw:pluginlist"]
static _ADD: [u8; 21] = *b"\x0ccubedaw:math\x07do_math";
