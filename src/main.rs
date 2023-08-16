
use std::io::{self, Read, Write};
use std::process::Command;
use std::thread;

use pty::fork::*;

fn main() {
    let fork = Fork::from_ptmx().unwrap();

    if let Some(mut master) = fork.is_parent().ok() {
        eprintln!("There was a fork");

        // Thread to continuously read output from the PTY master and echo to the screen
        let master_read_thread = thread::spawn(move || {
            let mut buffer = [0; 1024];
            loop {
                match master.read(&mut buffer) {
                    Ok(n) if n > 0 => {
                        io::stdout().write_all(&buffer[..n]).unwrap();
                        io::stdout().flush().unwrap();
                    }
                    Ok(_) => break,
                    Err(e) => panic!("read error: {}", e),
                }
            }
        });

        // Read from stdin and write to the PTY master
        let mut input = String::new();
        while io::stdin().read_line(&mut input).is_ok() && !input.trim().is_empty() {
            master.write_all(input.as_bytes()).unwrap();
            input.clear();
        }

        master_read_thread.join().unwrap();

    } else {
        // Set the child process as a session leader.
        eprintln!("I'm starting a shell");

        Command::new("sh")
            .arg("-c")
            .arg("stty icanon; bash")
            .status()
            .expect("could not start the shell");
    }
}

