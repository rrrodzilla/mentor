use anyhow::{anyhow, Error, Result};
use libc::{ioctl, winsize, TIOCGWINSZ, TIOCSWINSZ};
use nix::errno::Errno;
use pty::fork::*;
use std::io::{self, Read, Write};
use std::os::unix::io::AsRawFd;
use std::process::Command;
use std::thread;
use std::time::Duration;

fn main() -> Result<()> {
    let fork = Fork::from_ptmx()?;

    if let Fork::Parent(_pid, mut master) = fork {
        // Spawn a new thread to wait on the receiver
        let handle = thread::spawn(move || -> Result<()> {
            //give the system a moment to get the terminal ready before we resize it
            thread::sleep(Duration::from_millis(100));
            // Ensure child is ready before we listen
            // Get the current terminal size
            let (rows, cols) = get_terminal_size()?;

            // Set the size of the child PTY to match
            set_terminal_size(master.as_raw_fd(), rows, cols)?;
            Ok(())
        });

        eprintln!("Starting Rust Mentor...");
        // Thread to continuously read output from the PTY master and echo to the screen
        let master_read_thread = thread::spawn(move || -> Result<()> {
            let mut buffer = [0; 1024];
            loop {
                match master.read(&mut buffer) {
                    Ok(n) if n > 0 => {
                        io::stdout().write_all(&buffer[..n])?;
                        io::stdout().flush()?;
                    }
                    Ok(_) => break,
                    Err(e) => return Err(Error::from(e)),
                }
            }
            Ok(())
        });

        // Read from stdin and write to the PTY master
        let mut input = String::new();
        while io::stdin().read_line(&mut input).is_ok() && !input.trim().is_empty() {
            if input.trim() == "quit" {
                eprintln!("bye");

                std::process::exit(0);
            }

            master.write_all(input.as_bytes())?;
            input.clear();
        }

        master_read_thread
            .join()
            .map_err(|_| anyhow!("Master read thread panicked"))??;
        // This will wait for the thread that's waiting on the receiver to finish.
        handle.join().map_err(|_| anyhow!("Terminal resizer thread panicked"))??;
    } else {
        // Child process
        eprintln!("Now running. Type 'quit' to end mentor session.");
        // Inform the parent that the child is ready

        let shell = std::env::var("SHELL").unwrap_or_else(|_| "sh".into());
        Command::new(shell).status()?;
    }
    Ok(())
}
fn get_terminal_size() -> Result<(u16, u16)> {
    let ws: winsize = unsafe {
        let mut ws: winsize = std::mem::zeroed();
        if ioctl(0, TIOCGWINSZ, &mut ws) != 0 {
            return Err(anyhow!("Failed to get terminal size"));
        }
        ws
    };
    Ok((ws.ws_row, ws.ws_col))
}

fn set_terminal_size(fd: i32, rows: u16, cols: u16) -> Result<()> {
    let ws = winsize {
        ws_row: rows,
        ws_col: cols,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    unsafe {
        if ioctl(fd, TIOCSWINSZ, &ws) != 0 {
            let e = Errno::last();
            return Err(anyhow!("Failed to set terminal size: {}", e.desc()));
        }
    }
    Ok(())
}
