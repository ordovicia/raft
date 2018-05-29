#![allow(dead_code)]

#[macro_use]
extern crate failure;
extern crate rand;

type NodeId = usize;
type Term = u64;
type Millisec = i32;

mod error;
mod log;
mod message;
mod node;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        use NodeId;
        use node::{LocalNode, RemoteNode};
        use std::collections::HashMap;

        let election_timeout_range = (150, 300);

        let (mut locals, remotes): (Vec<LocalNode<i32>>, HashMap<NodeId, RemoteNode<i32>>) = (0..5)
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
