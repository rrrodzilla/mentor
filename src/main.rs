// extern crate libc;
// extern crate nix;

// use nix::pty::{openpty, OpenptyResult};
use nix::pty::openpty;
//use nix::sys::termios::*;
//use libc::{ECHO, ICANON};
use nix::sys::termios::tcgetattr;
use nix::sys::termios::tcsetattr;
use nix::sys::termios::LocalFlags;
use nix::sys::termios::SetArg::TCSANOW;
//use nix::sys::termios::Termios;
use nix::sys::wait::waitpid;
use nix::unistd::{close, dup2, execvp, fork, setsid, ForkResult};
use std::ffi::CString;
use std::time::SystemTime;

fn main() {
    // let (primary_fd, secondary_fd) = openpty(None, None).unwrap();

    let pty_result = openpty(None, None).unwrap();
    let primary_fd = pty_result.master;
    let secondary_fd = pty_result.slave;
    let fork_result = unsafe { fork().unwrap() };

    match fork_result {
        ForkResult::Parent { child } => {
            close(secondary_fd).unwrap();

            let mut buffer = [0; 1024];
            loop {
                let n = nix::unistd::read(primary_fd, &mut buffer).unwrap();
                if n == 0 {
                    break;
                }

                // Capture the data and annotate it.
                let data = String::from_utf8_lossy(&buffer[..n]);
                let annotated_data = annotate_data(&data);
                println!("{}", annotated_data);
            }

            waitpid(child, None).unwrap();
            close(primary_fd).unwrap();
        }
        ForkResult::Child => {
            close(primary_fd).unwrap();

            setsid().unwrap();
            // Set the secondary side of the pseudoterminal in canonical mode
            set_canonical_mode(secondary_fd);
            dup2(secondary_fd, libc::STDIN_FILENO).unwrap();
            dup2(secondary_fd, libc::STDOUT_FILENO).unwrap();
            dup2(secondary_fd, libc::STDERR_FILENO).unwrap();
            close(secondary_fd).unwrap();

            // execvp(&CString::new("/bin/sh").unwrap(), &[]).unwrap();

            execvp(
                &CString::new("/bin/sh").unwrap(),
                &[&CString::new("/bin/sh").unwrap()],
            )
            .unwrap();
        }
    }
}
fn set_canonical_mode(fd: i32) {
    let mut termios = tcgetattr(fd).unwrap();

    termios.local_flags.insert(LocalFlags::ECHO);
    termios.local_flags.insert(LocalFlags::ICANON);
    tcsetattr(fd, TCSANOW, &termios).unwrap();
}
fn annotate_data(data: &str) -> String {
    let current_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let mut annotated_lines = Vec::new();

    for line in data.lines() {
        let annotated_line = format!("[{}]: {}", current_time, line);
        annotated_lines.push(annotated_line);
    }

    annotated_lines.join("\n")
}
