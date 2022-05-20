#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use serde::{Serialize, Deserialize};
use tauri::api::Error;
use tauri::{CustomMenuItem, Menu, MenuItem, Submenu, Manager, App};
use std::io::{self, ErrorKind, Read, Write};
use std::net::{TcpStream};
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use tokio::task;
use std::sync::{Mutex, Arc};
use std::thread;
use std::time::Duration;
use tauri::async_runtime::{self};
use lazy_static::lazy_static;

const MSG_SIZE: usize = 32;
const PORT: &str = "2478";

lazy_static! {
  static ref CHANNEL: Mutex<Option<Sender<String>>> = Mutex::new(None);
}



fn start() -> Menu {
  let new = CustomMenuItem::new("new".to_string(), "New");
  let quit = CustomMenuItem::new("quit".to_string(), "Quit");
  let about = CustomMenuItem::new("about".to_string(), "About");

  let help = Submenu::new("Help", Menu::new().add_item(about));
  let file = Submenu::new("File", Menu::new().add_item(quit).add_item(new));



  Menu::new()
    .add_native_item(MenuItem::Copy)
    .add_submenu(file)
    .add_submenu(help)
  
}



  async fn connect(name: String, mut ipaddr: String) {

    ipaddr.push_str(":");
    ipaddr.push_str(PORT);
    
    let mut stream = TcpStream::connect(ipaddr).expect("Connection Error");
    stream.set_nonblocking(true).expect("Non-blocking failed");

    let tx: Sender<String>;
    let mut rx: Receiver<String>;

    (tx, rx) = std::sync::mpsc::channel::<String>();

    *CHANNEL.lock().unwrap() = Some(tx);
    

    let conn = thread::spawn(move || loop {

      let mut buff = vec![0; MSG_SIZE];
      match stream.read_exact(&mut buff) {
        Ok(_) => {
          let msg = String::from_utf8(buff.into_iter().take_while(|&x| x != 0).collect::<Vec<_>>());
          println!("Message resv {:?}", msg);
        },
        Err(ref err) if err.kind() == ErrorKind::WouldBlock => (),
        Err(_) => {
          println!("Connection to server broke");
          break;
        }
      }
  d
      match rx.try_recv() {
        Ok(msg) => {
          let mut buff = msg.clone().into_bytes();
          buff.resize(MSG_SIZE, 0);
          stream.write_all(&buff).expect("Writing to socket failed");
          println!("Message sent {:?}", msg);
        },
        Err(TryRecvError::Empty) => (),
        Err(TryRecvError::Disconnected) => break
      }
  
      thread::sleep(Duration::from_millis(100));
    });
   

    conn.join().unwrap();
  

  }


  


  async fn send_msg(msg: String)  {

    let lock = CHANNEL.lock().unwrap();
    let tx = lock.as_ref().clone().unwrap();
    
    let msg = msg.trim().to_string();
    tx.send(msg).expect("Failed to send message");
    
    drop(lock);
    
  }

  
  #[derive(Serialize, Deserialize)]
  struct Connector {
      name: String,
      ipaddr: String,
  }
 

fn main() {
  let menu = start();

  
  

  tauri::Builder::default()
    .setup(|app| {

      app.listen_global("connect", |event| {
          
        let payload: Connector = serde_json::from_str(event.payload().unwrap()).unwrap();
        let name: String = payload.name;
        let ip: String = payload.ipaddr;

        task::spawn(connect(name, ip));
        


      });


      app.listen_global("send_msg", |event| {

        let msg = "dav".to_string();//event.payload().unwrap().to_string();

        task::spawn(send_msg(msg));
        
        
  
      
      });

      
      Ok(())
    })
    .menu(menu)
    .on_menu_event(|event| {
      match event.menu_item_id() {
        "quit" => {
          std::process::exit(0);
        }
        _ => {}
      }
    })
    .invoke_handler(tauri::generate_handler![])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
