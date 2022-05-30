// Write code here.
//
// To see what the code looks like after macro expansion:
//     $ cargo expand
//
// To run the code:
//     $ cargo run
// use derive_builder::Builder;

// #[derive(Builder)]
// pub struct Command {
//     #[builder(each = "arg")]
//     args: Vec<String>,
// }

use derive_debug::CustomDebug;

#[derive(CustomDebug)]
pub struct Field {
    name: &'static str,
    bitmask: u8,
}
fn main() {}
