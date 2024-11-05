#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
#[allow(unused)] // passed in as parameter; never constructed in this plugin
enum MathNodeType {
    Add = 0,
    Subtract = 1,
    Multiply = 2,
    Divide = 3,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct MathNodeArgs {
    node_type: MathNodeType,
}

#[no_mangle]
extern "C" fn do_math(args: &MathNodeArgs, _state: *mut ()) {
    let in1 = cubedaw_pluginlib::input::<0>();
    let in2 = cubedaw_pluginlib::input::<1>();

    let val = match args.node_type {
        MathNodeType::Add => in1 + in2,
        MathNodeType::Subtract => in1 - in2,
        MathNodeType::Multiply => in1 * in2,
        MathNodeType::Divide => in1 / in2,
    };

    cubedaw_pluginlib::output::<0>(val);
}

cubedaw_pluginlib::export_node!("cubedaw:math", do_math);
