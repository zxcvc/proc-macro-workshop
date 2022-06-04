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
use std::fmt::Debug;

// #[derive(CustomDebug)]
// #[debug(bound = "T::Value: Debug")]
// pub struct Wrapper<T: Trait> {
//     field: Field<T>,
// }
pub trait Trait {
    type Value;
}


#[derive(CustomDebug)]
#[debug(bound = "T::Value: Debug,T:Debug,U:Debug")]
pub struct Field<T:Trait,U> {
    // value: T,
    #[debug = "0b{:08b}"]
    bitmask: u8,
    s:std::marker::PhantomData<T>,
    ss:Box<T>,
    f:Box<Vec<U>>,
    values: Vec<T::Value>,
}

fn main() {
    // let f = Field {
    //     name: "F",
    //     bitmask: 255,
    // };

    // println!("{:?}",f);
}
