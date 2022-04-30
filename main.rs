// Write code here.
//
// To see what the code looks like after macro expansion:
//     $ cargo expand
//
// To run the code:
//     $ cargo run
use derive_builder::Builder;

#[derive(Builder)]
struct A{
    pub name:String,
    s:Option<String>,
    a:i32,
}
fn main() {
}
