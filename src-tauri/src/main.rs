#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use json;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use serde_json;
use std::io::{ErrorKind, Read, Write};
use std::net::TcpStream;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use tauri::{CustomMenuItem, Manager, Menu, MenuItem, Submenu, Window};
use tokio::task;

mod server;

const MSG_SIZE: usize = 512;
const PORT: &str = "2478";
const CONNECTION_TRIES: u8 = 5;

lazy_static! {
    static ref CHANNEL: Mutex<Option<Sender<String>>> = Mutex::new(None);
    static ref MSG_TO_SEND: Mutex<String> = Mutex::new(String::new());
    static ref INFO_MSG: Mutex<String> = Mutex::new(String::new());
    static ref USER: Mutex<String> = Mutex::new(String::new());
    static ref IS_RUNNING: Mutex<bool> = Mutex::new(false);
}

fn get_menu() -> Menu {
    let start = CustomMenuItem::new("start-server".to_string(), "Start server");
    let stop = CustomMenuItem::new("stop-server".to_string(), "Stop server");
    let quit = CustomMenuItem::new("quit".to_string(), "Quit");
    let about = CustomMenuItem::new("about".to_string(), "About");

    let help = Submenu::new("Help", Menu::new().add_item(about));
    let main = Submenu::new("Main", Menu::new().add_item(start).add_item(stop).add_item(quit));

    Menu::new()
        .add_native_item(MenuItem::Copy)
        .add_submenu(main)
        .add_submenu(help)
}

async fn connect(name: String, mut ipaddr: String) {
    ipaddr.push_str(":");
    ipaddr.push_str(PORT);

    display_info_msg("Connecting to server...", "Info");
    let mut stream_type: Option<TcpStream> = None;
    for _ in 0..CONNECTION_TRIES {
        let stream_result = TcpStream::connect(&ipaddr);
        if stream_result.is_ok() {
            stream_type = Some(stream_result.unwrap());
            break;
        }

        thread::sleep(Duration::from_millis(1000));
    }

    let mut stream;
    if stream_type.is_some() {
        display_info_msg("Connected to server", "Success");
        stream = stream_type.unwrap();
    } else {
        display_info_msg("Could not connect to server", "Error");
        return;
    }
    stream.set_nonblocking(true).expect("Non-blocking failed");

    let tx: Sender<String>;
    let rx: Receiver<String>;

    (tx, rx) = std::sync::mpsc::channel::<String>();

    *CHANNEL.lock().unwrap() = Some(tx);
    *USER.lock().unwrap() = name;
    *IS_RUNNING.lock().unwrap() = true;

    let conn = thread::spawn(move || loop {
        let mut buff = vec![0; MSG_SIZE];
        match stream.read_exact(&mut buff) {
            Ok(_) => {
                let msg =
                    String::from_utf8(buff.into_iter().take_while(|&x| x != 0).collect::<Vec<_>>());
                if msg.is_ok() {
                    println!("Message resv {:?}", msg);
                    *MSG_TO_SEND.lock().unwrap() = msg.unwrap();
                } else {
                    println!("Error parsing message");
                }
            }
            Err(ref err) if err.kind() == ErrorKind::WouldBlock => (),
            Err(_) => {
                display_info_msg("Connection to server broke", "Alert");
                *IS_RUNNING.lock().unwrap() = false;
                break;
            }
        }

        match rx.try_recv() {
            Ok(msg) => {
                let mut buff = msg.clone().into_bytes();
                buff.resize(MSG_SIZE, 0);
                stream.write_all(&buff).expect("Writing to socket failed");
                println!("Message sent {:?}", msg);
            }
            Err(TryRecvError::Empty) => (),
            Err(TryRecvError::Disconnected) =>  {
                *IS_RUNNING.lock().unwrap() = false;
                break;
            },
        }

        thread::sleep(Duration::from_millis(100));
    });

    conn.join().unwrap();
}

fn display_info_msg(msg: &str, kind: &str) {
    let mut mut_msg = String::from("{\"msg\": \"");
    mut_msg.push_str(&msg);
    mut_msg.push_str("\"}");
    let mut msg_json = json::parse(&mut_msg).unwrap();
    let mut kind_string = String::from("{");
    kind_string.push_str(kind);
    kind_string.push_str("}");
    
    msg_json
        .insert("kind", kind_string)
        .expect("Could not insert kind on info-message");
    let final_msg = msg_json.to_string();

    println!("{}", final_msg);
    *INFO_MSG.lock().unwrap() = final_msg;
}

fn send_msg(msg: String) {
    let lock = CHANNEL.lock().unwrap();
    let tx = lock.as_ref().clone().unwrap();

    let mut msg_json = json::parse(&msg.trim().to_string()).unwrap();
    msg_json
        .insert("user", USER.lock().unwrap().to_string())
        .expect("Json insert failed");
    let msg = msg_json.to_string();
    tx.send(msg).expect("Failed to send message");

    drop(lock);
}

#[derive(Serialize, Deserialize)]
struct Connector {
    name: String,
    ipaddr: String,
}

#[derive(Clone, serde::Serialize)]
struct Message {
    message: String,
    user: String,
}

#[derive(Clone, serde::Serialize)]
struct Info {
    message: String,
    kind: String,
}

#[tauri::command]
fn add_to_chatbox(window: Window) {
    thread::spawn(move || {
        let mut raw_str_msg;
        let mut raw_str_info;
        loop {
            raw_str_msg = MSG_TO_SEND.lock().unwrap().to_string();
            raw_str_info = INFO_MSG.lock().unwrap().to_string();
            if raw_str_msg != "" {
                let result = json::parse(&MSG_TO_SEND.lock().unwrap().to_string());
                let msg_json = if result.is_ok() {
                    result.unwrap()
                } else {
                    continue;
                };
                let msg = msg_json["msg"].to_string();
                let user = msg_json["user"].to_string();

                window
                    .emit("add_to_chatbox", Message { message: msg, user })
                    .unwrap();
                *MSG_TO_SEND.lock().unwrap() = String::new();
            } else if raw_str_info != "" {
                let result = json::parse(&INFO_MSG.lock().unwrap().to_string());
                let msg_json = if result.is_ok() {
                    result.unwrap()
                } else {
                    continue;
                };
                let msg = msg_json["msg"].to_string();
                let kind = msg_json["kind"].to_string();

                window
                    .emit("add_info_to_chatbox", Info { message: msg, kind })
                    .unwrap();
                *INFO_MSG.lock().unwrap() = String::new();
            }
            thread::sleep(Duration::from_millis(100));
        }
    });
}

fn main() {
    let menu = get_menu();

    tauri::Builder::default()
        .setup(|app| {
            let main_window = app.get_window("main").unwrap();
            add_to_chatbox(main_window);

            app.listen_global("connect", |event| {
                let payload: Connector = serde_json::from_str(event.payload().unwrap()).unwrap();
                let name: String = payload.name;
                let ip: String = payload.ipaddr;

                if *IS_RUNNING.lock().unwrap() {
                    display_info_msg("Already connected to server", "Alert");
                } else if name == "" {
                    display_info_msg("Name-field is empty", "Alert");
                } else {
                    task::spawn(connect(name, ip));
                }
            });

            app.listen_global("send_msg", |event| {
                if *IS_RUNNING.lock().unwrap() {
                    let msg = event.payload().unwrap().to_string();

                    send_msg(msg);
                } else {
                    display_info_msg("Not connected to server", "Alert");
                }
            });

            Ok(())
        })
        .menu(menu)
        .on_menu_event(|event| match event.menu_item_id() {
            "quit" => {
                std::process::exit(0);
            },
            "start-server" => {
                server::start();
            },
            "stop-server" => {
                server::stop();
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![add_to_chatbox])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
