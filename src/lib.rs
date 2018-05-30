#![feature(deadline_api)]
#![allow(dead_code)]

// TODO: proper privacy
// TODO: client command

#[macro_use]
extern crate failure;
extern crate rand;

pub type Term = u64;
pub type Millisec = u64;

pub mod error;
pub mod log;
pub mod message;
pub mod node;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        use node::{LocalNode, RemoteNode};
        use std::collections::HashMap;

        let election_timeout_range = (150, 300);

        type Remote = RemoteNode<i32>;
        type Local = LocalNode<i32, Remote>;

        let (mut locals, remotes): (Vec<Local>, HashMap<usize, Remote>) = (0..5)
            .map(|id| {
                let (local, tx) = LocalNode::new(id, election_timeout_range);
                let remote = RemoteNode::new(id, tx);
                (local, (id, remote))
            })
            .unzip();

        for (id, local) in locals.iter_mut().enumerate() {
            let mut remotes = remotes.clone();
            remotes.remove(&id);
            local.set_peers(remotes);
            // println!("{}: {:#?}", id, local);
        }
    }
}
