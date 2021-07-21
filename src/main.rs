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

use god::{Connection, StackItem};
const MAX: usize = 100;

fn dt_read(stream: &mut TcpStream) -> Vec<u8> {
    let mut data = Vec::new();
    let mut header = [0; 1];
    let peeked_size = stream.peek(&mut header).unwrap();
    if peeked_size == 0 {
        // return false;
        return data;
    }
    let msb = header[0] & (1 << 7);
    if msb != 0 {
        data.push(header[0]);
        // connection.data = Some(data);
        return data;
        // return true;
        //pop mode
    }
    let size = header[0] & !(1 << 7);
    while data.len() != (size + 1).into() {
        println!("inside blocking read");
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
                println!("error in reading data..");
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
            println!("safe_close() warning.");
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
            //acquire lock
            // then loop over connections if data skip if no data
            // if data then do your thing remove from connections and safe close
            //

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
                        eprintln!("got push");
                        // ********* push *****
                        if stack.len() == MAX {
                            eprintln!("but blocking...");
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
                        eprintln!("got pop");
                        // ********* pop start *****
                        if stack.len() == 0 {
                            index += 1;
                            continue;
                        }

                        let mut popped_item = stack.pop().unwrap();
                        eprintln!("popped: {}", popped_item);
                        popped_item.data.insert(0, popped_item.size as u8);
                        connection.stream.write(popped_item.data.as_mut_slice());
                        safe_close(&mut connection.stream);
                        connections.remove(index);
                        break;
                        //********* pop end */
                    }
                }
            }
            sleep(10);
        }
    });

    // handler thread ends

    loop {
        if let Ok((mut socket, _)) = server.accept() {
            {
                //lock block
                // println!("got a clinet");
                let connections = &mut *connection_pool.lock().unwrap();
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
                    socket.write(&[0xff]);
                    safe_close(&mut socket);
                }
            }
        }
        sleep(10);
    }
}
