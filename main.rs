// Write code here.
//
// To see what the code looks like after macro expansion:
//     $ cargo expand
//
// To run the code:
//     $ cargo run
use derive_builder::Builder;

#[derive(Builder)]
pub struct Command {
    #[builder(each = "arg")]
    args: Vec<String>,
}

fn main() {}
