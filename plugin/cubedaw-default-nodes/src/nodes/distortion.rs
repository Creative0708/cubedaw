#[no_mangle]
fn do_distortion(state: &OscillatorNodeArgs, buf: &mut OscillatorNodeState) {
    let input = cubedaw_pluginlib::input::<0>();

    let val = input.pow(0.1);

    cubedaw_pluginlib::output::<0>(val);
}

cubedaw_pluginlib::export_node!("cubedaw:distortion", do_distortion);
