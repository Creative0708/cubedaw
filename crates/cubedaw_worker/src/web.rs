// use std::io;

// use log::{warn, info};
// use wasm_bindgen::prelude::*;

// use cubedaw_lib::{misc, buffer::ReadBuffer};
// use crate::{State, StateChange, state::worker_state};

// #[wasm_bindgen]
// pub fn replace_state(data: Box<[u8]>){
//     info!("initializing state");
//     if let Err(err) = replace_state_impl(data) {
//         warn!("Error decoding state: {}", misc::stringify_ciborium_error(err));
//     }
// }
// #[wasm_bindgen]
// pub fn update_state(data: Box<[u8]>){
//     if let Err(err) = update_state_impl(data) {
//         warn!("Error decoding state change: {}", misc::stringify_ciborium_error(err));
//     }
// }

// fn update_state_impl(data: Box<[u8]>) -> Result<(), ciborium::de::Error<io::Error>> {
//     let buf = ReadBuffer::new(data);
//     let state_change: StateChange = ciborium::from_reader(buf)?;

//     state_change.apply(&mut worker_state().write().expect("Poisoned worker state"));

//     Ok(())
// }
// fn replace_state_impl(data: Box<[u8]>) -> Result<(), ciborium::de::Error<io::Error>> {
//     let buf = ReadBuffer::new(data);
//     let new_state: State = ciborium::from_reader(buf)?;

//     *worker_state().write().expect("Poisoned worker state") = new_state;

//     Ok(())
// }

// #[wasm_bindgen]
// pub fn main() {
//     eframe::WebLogger::init(log::LevelFilter::Debug).ok();
// }
