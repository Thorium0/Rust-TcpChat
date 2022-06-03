use std::io::{ErrorKind, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::thread;
use lazy_static::lazy_static;
use std::sync::Mutex;

use crate::INFO_MSG;

const LOCAL: &str = "0.0.0.0:2478";
const MSG_SIZE: usize = 512;

lazy_static! {
    static ref SERVER_IS_RUNNING: Mutex<bool> = Mutex::new(false);
    static ref SERVER_KEEP_ALIVE: Mutex<bool> = Mutex::new(true);
}

fn display_info_msg(msg: &str, kind: &str) {
    let mut mut_msg = String::from("{\"msg\": \"");
    mut_msg.push_str(&msg);
    mut_msg.push_str("\"}");
    let mut msg_json = json::parse(&mut_msg).unwrap();
    let mut kind_string = String::from("[SERVER] {");
    kind_string.push_str(kind);
    kind_string.push_str("}");
    
    msg_json
        .insert("kind", kind_string)
        .expect("Could not insert kind on info-message");
    let final_msg = msg_json.to_string();

    println!("{}", final_msg);
    *INFO_MSG.lock().unwrap() = final_msg;
}

fn sleep() {
    thread::sleep(::std::time::Duration::from_millis(100));
}

pub fn stop() {
    if *SERVER_IS_RUNNING.lock().unwrap() {
        *SERVER_KEEP_ALIVE.lock().unwrap() = false;
    } else {
        display_info_msg("Server is not running", "Alert");
    }
}


pub fn start() {
    if *SERVER_IS_RUNNING.lock().unwrap() {
        display_info_msg("Server is already running", "Alert");
        return;
    }
    let server = match TcpListener::bind(LOCAL) {
        Ok(tcpl) => tcpl,
        Err(err) => {
            display_info_msg("Listnener failed to bind", "Error");
            println!("{}", err);
            return;
        }
    };
  
    server.set_nonblocking(true).expect("[SERVER] Non-blocking failed");
    let mut msg_string = String::from("Server listening on ");
    msg_string.push_str(LOCAL);
    msg_string.push_str("...");
    display_info_msg(&msg_string, "Success");

    let mut clients = vec![];
    let (tx, rx) = mpsc::channel::<String>();
    *SERVER_IS_RUNNING.lock().unwrap() = true;
    thread::spawn (move || loop {
        if !*SERVER_KEEP_ALIVE.lock().unwrap() {
            *SERVER_KEEP_ALIVE.lock().unwrap() = true;
            *SERVER_IS_RUNNING.lock().unwrap() = false;
            display_info_msg("Server shutting down...", "Success");
            break;
        }
        if let Ok((mut socket, addr)) = server.accept() {
            let mut msg_string = String::from("Client connected with ");
            msg_string.push_str(&addr.to_string());
            display_info_msg(&msg_string, "Info");

            let tx = tx.clone();
            clients.push(socket.try_clone().expect("failed to clone client"));

            thread::spawn (move || loop {
                let mut buff = vec![0; MSG_SIZE];

                match socket.read_exact(&mut buff) {
                    Ok(_) =>{
                        let msg = buff.into_iter().take_while(|&x| x != 0).collect::<Vec<_>>();
                        let msg = String::from_utf8(msg).expect("Invalid message");

                        println!("{}: {:?}", addr, msg);
                        tx.send(msg).expect("Failed to send msg");
                        
                    },
                    Err(ref err) if err.kind() == ErrorKind::WouldBlock => (),
                    Err(_) => {
                        let mut msg_string = String::from("Closing connection to ");
                        msg_string.push_str(&addr.to_string());
                        display_info_msg(&msg_string, "Info");
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