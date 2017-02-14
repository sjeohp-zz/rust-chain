use network::*;
use block::*;
use transaction::*;
use std::env;

extern crate mio;
use self::mio::channel::{channel};

extern crate rustyline;
use self::rustyline::error::ReadlineError;
use self::rustyline::Editor;

use std::thread;

#[ignore]
#[test]
fn test_network()
{
    let (transaction_snd_to_mine, transaction_rcv_from_network) = channel::<Transaction>();
    let (block_snd_to_mine, block_rcv_from_network) = channel::<Block>();
    let (block_snd_to_network, block_rcv_from_mine) = channel::<Block>();
    let (quit_snd, quit_rcv) = channel::<()>();
    let network_child = thread::spawn(move || {
        start_server(
            env::args().nth(1),
            quit_rcv,
            transaction_snd_to_mine,
            block_snd_to_mine,
            block_rcv_from_mine);
    });

    let _ = quit_snd.send(());
    let _ = network_child.join();
}
