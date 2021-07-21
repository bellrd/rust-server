use std::{
    io::{Read, Write},
    net::{Shutdown, TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use uuid::Uuid;

fn generate_id() -> Uuid {
    return Uuid::new_v4();
}

fn ok_connection(stream: &mut TcpStream) -> bool {
    let mut buf = [0; 100];
    stream.set_nonblocking(true);
    match stream.read(&mut buf) {
        Ok(n) => {
            if n == 0 {
                return false;
            }
        }
        Err(e) => {}
    }
    stream.set_nonblocking(false);
    return true;
}

use god::{Connection, StackItem};
const MAX: usize = 100;

fn dt_read(stream: &mut TcpStream) -> Vec<u8> {
    let mut data = Vec::new();
    let mut buffer = [0; 1024];
    let mut header = [0; 1];
    let mut peeked_size = 0;
    match stream.peek(&mut header) {
        Ok(n) => {
            peeked_size = n;
        }
        Err(e) => {println!("fail silently")}
    };
    if peeked_size == 0 {
        return data;
    }
    let msb = header[0] & (1 << 7);
    if msb != 0 {
        match stream.read(&mut buffer) {
            Ok(n) => {
                for i in 0..n {
                    data.push(buffer[i]);
                }
                return data;
            }
            Err(_) => {
                //handle later
                println!("read error from pop request");
            }
        }
        return data;
    }
    let size = header[0] & !(1 << 7);
    while data.len() != (size + 1).into() {
        let mut buffer = [0; 1024];
        match stream.read(&mut buffer) {
            Ok(n) => {
                if n == 0 {
                    data.clear();
                    return data;
                } else {
                    for i in 0..n {
                        data.push(buffer[i]);
                    }
                }
            }
            Err(_) => {
                data.clear();
                return data;
            }
        }
    }
    return data;
}

fn safe_close(stream: &mut TcpStream) -> bool {
    match stream.shutdown(Shutdown::Both) {
        Ok(_) => {}
        Err(_) => {
            println!("un-safe_close() warning.");
            return false;
        }
    }
    return true;
}

fn sleep(millis: u64) {
    thread::sleep(Duration::from_millis(millis));
}

fn main() {
    let server = TcpListener::bind("127.0.0.1:8080").expect("error in starting server");
    server
        .set_nonblocking(true)
        .expect("can't set non blocking");

    let connection_pool = Arc::new(Mutex::new(Vec::<Connection>::new()));
    let h_connection_pool = Arc::clone(&connection_pool);
    let mut stack: Vec<StackItem> = Vec::new();

    // handler thread
    thread::spawn(move || {
        loop {
            {
                let connections = &mut *h_connection_pool.lock().unwrap();
                let mut index = 0;
                while index < connections.len() {
                    let connection = connections.get_mut(index).unwrap();
                    let data = match &connection.data {
                        Some(s) => s,
                        None => {
                            index += 1;
                            continue;
                        }
                    };
                    let header = data[0];
                    let msb = header & (1 << 7);
                    if msb == 0 {
                        // eprintln!("got push");
                        // ********* push *****
                        if stack.len() == MAX {
                            index += 1;
                            continue;
                        }
                        let size = header & !(1 << 7);
                        connection.stream.write(&[0x00]);
                        if safe_close(&mut connection.stream) {
                            let to_push = StackItem::new(size as usize, data[1..].to_vec());
                            eprintln!("pushing: {}", to_push);
                            stack.push(to_push);
                            // index = 0; // restart loop
                        }
                        connections.remove(index);
                        break;
                        // ********* push end **
                    } else {
                        //pop
                        // eprintln!("got pop");
                        // ********* pop start *****
                        if stack.len() == 0 {
                            if !ok_connection(&mut connection.stream) {
                                safe_close(&mut connection.stream);
                                connections.remove(index);
                                break;
                            }
                            index += 1;
                            continue;
                        }

                        let mut popped_item = stack.pop().unwrap();
                        eprintln!("popped: {}", popped_item);
                        popped_item.data.insert(0, popped_item.size as u8);
                        connection
                            .stream
                            .write(popped_item.data.as_mut_slice())
                            .expect("some error in write");
                        safe_close(&mut connection.stream);
                        connections.remove(index);
                        break;
                        //********* pop end */
                    }
                }
            }
            sleep(5);
        }
    });

    // handler thread ends

    loop {
        if let Ok((mut socket, _)) = server.accept() {
            {
                //lock block
                socket.set_nonblocking(false).expect("tree tree");
                let connections = &mut *connection_pool.lock().unwrap();
                println!("total connection: {}", connections.len());
                // check if connection is greater than or eq 100
                // try to remove older client 10s policy
                // then add
                if connections.len() >= MAX {
                    let oldest = connections.get_mut(0).unwrap();
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs();
                    if now - oldest.connect_time >= 10 {
                        //safe close;
                        // oldest.stream.write(&[0x00]);
                        println!("remove oldest one");
                        safe_close(&mut oldest.stream);
                        connections.remove(0);
                    }
                }
                //
                if connections.len() < MAX {
                    let id = generate_id();
                    connections.push(Connection::new(socket.try_clone().unwrap(), id.clone()));
                    let dt_connection_pool = Arc::clone(&connection_pool);
                    // dt closure
                    thread::spawn(move || {
                        let data = dt_read(&mut socket);
                        let connections = &mut *dt_connection_pool.lock().unwrap();
                        let mut position = -1;
                        match connections.iter().position(|r| r.id == id) {
                            Some(n) => position = n as i32,
                            None => {
                                position = -1;
                            }
                        };
                        if position == -1 {
                            return;
                        }
                        if data.len() == 0 {
                            println!("I am causing warning");
                            safe_close(&mut socket);
                            // remove connection after safe closing
                            connections.remove(position as usize);
                            //handle edge case
                            // find and delete else go on
                        } else {
                            let c = connections.get_mut(position as usize).unwrap();
                            println!("read data = {}", String::from_utf8_lossy(&data));
                            c.data = Some(data);
                        }
                        return;
                    });
                    // dt closure end
                    // thread::spawn(dt_closure);
                } else {
                    socket.write(&[0xff]).expect("some unknown error occurred");
                    safe_close(&mut socket);
                }
            }
        }
        // sleep(40);
    }
}
