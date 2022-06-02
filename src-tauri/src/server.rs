use std::io::{ErrorKind, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::thread;

use crate::display_info_msg;

const LOCAL: &str = "0.0.0.0:2478";
const MSG_SIZE: usize = 512;

fn sleep() {
    thread::sleep(::std::time::Duration::from_millis(100));
}

pub fn start() {
    let server = TcpListener::bind(LOCAL).expect("[SERVER] Listnener failed to bind");
    server.set_nonblocking(true).expect("[SERVER] Non-blocking failed");
    let mut msg_string = String::from("Server listening on ");
    msg_string.push_str(LOCAL);
    msg_string.push_str("...");
    display_info_msg(&msg_string, "Success");
    println!("[SERVER] Server listening on {}...", LOCAL);

    let mut clients = vec![];
    let (tx, rx) = mpsc::channel::<String>();
    thread::spawn (move || loop {
        if let Ok((mut socket, addr)) = server.accept() {
            println!("[SERVER] Client connected with {}", addr);

            let tx = tx.clone();
            clients.push(socket.try_clone().expect("[SERVER] failed to clone client"));

            thread::spawn (move || loop {
                let mut buff = vec![0; MSG_SIZE];

                match socket.read_exact(&mut buff) {
                    Ok(_) =>{
                        let msg = buff.into_iter().take_while(|&x| x != 0).collect::<Vec<_>>();
                        let msg = String::from_utf8(msg).expect("[SERVER] Invalid message");

                        println!("{}: {:?}", addr, msg);
                        tx.send(msg).expect("[SERVER] Failed to send msg");
                        
                    },
                    Err(ref err) if err.kind() == ErrorKind::WouldBlock => (),
                    Err(_) => {
                        println!("[SERVER] Closing connection with {}", addr);
                        break;
                    }
                }
                sleep();
            });
        }
        if let Ok(msg) = rx.try_recv() {
            clients = clients.into_iter().filter_map(|mut client| {
                let mut buff = msg.clone().into_bytes();
                buff.resize(MSG_SIZE, 0);

                client.write_all(&buff).map(|_| client).ok()
            }).collect::<Vec<_>>();
        }

        sleep();
    });

}