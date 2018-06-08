#![feature(deadline_api)]
#![allow(dead_code)]

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
        use std::collections::HashMap;
        // use std::thread;

        use node::{LocalNode, RemoteNode};

        let election_timeout_range = (150, 300);

        type Remote = RemoteNode<i32>;
        type Local = LocalNode<i32, Remote>;

        const NODE_NUM: usize = 5;

        let (mut locals, remotes): (Vec<Local>, HashMap<usize, Remote>) = (0..NODE_NUM)
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
        }

        // let mut threads = Vec::with_capacity(NODE_NUM);
        //
        // for mut local in locals {
        //     threads.push(thread::spawn(move || loop {
        //         if let Err(e) = local.tick_one() {
        //             eprintln!("{}", e);
        //             break;
        //         }
        //     }));
        // }
        //
        // for thread in threads {
        //     thread
        //         .join()
        //         .expect("Couldn't join on the associated thread");
        // }
    }
}
