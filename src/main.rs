use std::{
    io::{Read, Write},
    net::{Shutdown, TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use god::{Connection, StackItem};

fn sleep(duration: u64) {
    thread::sleep(Duration::from_millis(duration));
}
const MAX:usize = 100;

fn ok_state(stream:&mut TcpStream) -> bool{
    let mut buf = [0;1];
    let size = stream.peek(&mut buf).unwrap();
    if size == 0 {
        eprintln!("okay_state() not okay state");
        return false;
    }
    return true;
}

fn safe_close(stream:&mut TcpStream) -> bool{
    match stream.shutdown(Shutdown::Both){
        Ok(_) => {},
        Err(_) => {
            println!("safe_close() warning.");
            return false;
        },
    }
    return true;
}

fn main() {
    let server = TcpListener::bind("127.0.0.1:8080").expect("server not started");
    server
        .set_nonblocking(true)
        .expect("failed to set non blocking mode");
    let connection_pool = Arc::new(Mutex::new(Vec::<Connection>::new()));
    let t_connection_pool = Arc::clone(&connection_pool);
    let mut stack: Vec<StackItem> = Vec::new();

    thread::spawn(move || {
        loop {
            {
                let pool = &mut *t_connection_pool.lock().unwrap();
                let mut index = 0;
                let mut to_remove = Vec::new();
                while index < pool.len() {
                    to_remove.clear();
                    let connection = pool.get_mut(index).unwrap();
                    // let header = peek_header(&mut connection.stream)[0];
                    let mut header = [0; 1];
                    let picked_size = connection.stream.peek(&mut header).unwrap();
                    if picked_size == 0 {
                        // connection.stream.shutdown(Shutdown::Both).expect("error 37");
                        safe_close(&mut connection.stream);
                        println!("jinga jinga");
                        to_remove.push(index);
                        break;
                    }
                    let msb = header[0] & (1 << 7);
                    if msb == 0 {
                        if !ok_state(&mut connection.stream) {
                            safe_close(&mut connection.stream);
                            println!("pinga pinga");
                            to_remove.push(index);
                            break;
                        }
                        if stack.len() == MAX {
                            index += 1;
                            continue;
                        }
                        // push op
                        let size = header[0] & !(1 << 7);
                        let mut payload = vec![0; (size+1).into()];
                        match connection.stream.read(&mut payload.as_mut_slice()) {
                            Ok(n) => {
                                if n == 0 {
                                    safe_close(&mut connection.stream);
                                    println!("tillu");
                                    to_remove.push(index);
                                    break;
                                }
                            }
                            _ => {println!("Some error occurred")}
                        }
                        // connection.stream.read_exact(&mut payload);
                        payload.remove(0); // remove header byte 
                        let si = StackItem::new(size, payload.to_vec());
                        eprintln!("push_item: {}", si);
                        stack.push(si);
                        connection.stream.write(&[0x00]).expect("write error push");
                        // connection.stream.shutdown(Shutdown::Both).expect("error 01");
                        if !safe_close(&mut connection.stream){
                            stack.pop();
                        }
                        println!("pillu");
                        to_remove.push(index);
                        break;
                    } else {
                        if !ok_state(&mut connection.stream) {
                            safe_close(&mut connection.stream);
                            println!("nallu");
                            to_remove.push(index);
                            break;
                        }
                        if stack.len() == 0 {
                            index += 1;
                            continue;
                        }
                        // pop op
                        let mut popped_item = stack.pop().unwrap();
                        eprintln!("popped item = {}", popped_item);
                        popped_item.data.insert(0, popped_item.size);
                        connection.stream.write(popped_item.data.as_slice()).expect("error line 78");
                        // connection.stream.shutdown(Shutdown::Both).expect("error line 79");
                        safe_close(&mut connection.stream);
                        println!("hiccu");
                        to_remove.push(index);
                        break;
                    }
                }
                // to remove
                for i in to_remove {
                    pool.remove(i);
                }
            }
            // to remove
            sleep(20);
        }
    });

    loop {
        if let Ok((mut socket, addr)) = server.accept() {
            let clients = &mut *connection_pool.lock().unwrap();
            if clients.len() < MAX {
                eprintln!("Got a client");
                clients.push(Connection::new(socket));
            } else {
                println!("connection pool full trying to removing oldest:");
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                let oldest_client = clients.get_mut(0).unwrap();
                //try to disconnect oldest client
                if (now - oldest_client.connect_time) >= 10 {
                    println!("removing oldest client");
                    oldest_client.stream.write(&[0xff]);
                    oldest_client.stream.shutdown(Shutdown::Both);
                    clients.remove(0);
                    eprintln!("Got a client via 2 ");
                    clients.push(Connection::new(socket))
                } else {
                    //no space for you
                    println!("no space for you");
                    socket.write(&[0xff]);
                    // socket.shutdown(Shutdown::Both);
                    safe_close(&mut socket);
                };
            }
        }
        sleep(20);
    }
}
