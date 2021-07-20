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
                    connection.stream.peek(&mut header).unwrap();
                    let msb = header[0] & (1 << 7);
                    eprintln!("msb = {}", msb);
                    if msb == 0 {
                        if stack.len() == 100 {
                            index += 1;
                            continue;
                        }
                        // push op
                        let size = header[0] & !(1 << 7);
                        // let payload = read_payload(&mut connection.stream, size.into());
                        let mut payload = vec![0; (size + 1) as usize];
                        connection.stream.read_exact(&mut payload);
                        let si = StackItem::new(size, payload[1..].to_vec());
                        eprintln!("si: {}", si);
                        stack.push(si);
                        connection.stream.write(&[0x00]);
                        connection.stream.shutdown(Shutdown::Both);
                        to_remove.push(index);
                        break;
                    } else {
                        if stack.len() == 0 {
                            index += 1;
                            continue;
                        }
                        // pop op
                        let mut popped_item = stack.pop().unwrap();
                        eprintln!("popped item = {}", popped_item);
                        popped_item.data.insert(0, popped_item.size);
                        connection.stream.write(popped_item.data.as_slice());
                        connection.stream.shutdown(Shutdown::Both);
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
            if clients.len() < 100 {
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
                    socket.shutdown(Shutdown::Both);
                };
            }
        }
        sleep(20);
    }
}
