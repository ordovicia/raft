#![allow(dead_code)]

#[macro_use]
extern crate derive_builder;
#[macro_use]
extern crate failure;
extern crate rand;

type NodeId = usize;
type Term = u64;
type MilliSec = i32;

mod error;
mod log;
mod message;
mod node;
mod rpc;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
