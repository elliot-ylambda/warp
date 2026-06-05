use std::io::{self, Read, Write};
use std::os::unix::io::RawFd;
use std::sync::atomic::{AtomicI32, Ordering};

use anyhow::{Context, Result};
use crossterm::terminal;
use nix::unistd::{close, pipe, read, write as nix_write};

use crate::pty::sync_pty_size;

/// Write-end of the SIGWINCH self-pipe. Set by `install_sigwinch_handler`.
static SIGWINCH_WRITE_FD: AtomicI32 = AtomicI32::new(-1);

extern "C" fn sigwinch_handler(_sig: libc::c_int) {
    let fd = SIGWINCH_WRITE_FD.load(Ordering::Relaxed);
    if fd >= 0 {
        unsafe {
            libc::write(fd, b"W".as_ptr() as *const libc::c_void, 1);
        }
    }
}

fn install_sigwinch_handler() -> Result<RawFd> {
    let (read_fd, write_fd) = pipe().context("pipe for SIGWINCH")?;

    // Make both ends non-blocking so the signal handler never blocks.
    for fd in [read_fd, write_fd] {
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) };
    }

    SIGWINCH_WRITE_FD.store(write_fd, Ordering::Relaxed);

    unsafe {
        let mut sa: libc::sigaction = std::mem::zeroed();
        sa.sa_sigaction = sigwinch_handler as usize;
        sa.sa_flags = libc::SA_RESTART;
        libc::sigemptyset(&mut sa.sa_mask);
        if libc::sigaction(libc::SIGWINCH, &sa, std::ptr::null_mut()) < 0 {
            return Err(anyhow::anyhow!("sigaction SIGWINCH: {}", io::Error::last_os_error()));
        }
    }

    Ok(read_fd)
}

pub fn run(master_fd: RawFd) -> Result<()> {
    let sigwinch_fd = install_sigwinch_handler()?;

    // Sync terminal size before entering the loop.
    sync_pty_size(master_fd)?;

    terminal::enable_raw_mode().context("enable_raw_mode")?;

    let result = run_inner(master_fd, sigwinch_fd);

    // Always restore terminal state.
    let _ = terminal::disable_raw_mode();

    // Clean up the self-pipe.
    let write_fd = SIGWINCH_WRITE_FD.swap(-1, Ordering::Relaxed);
    if write_fd >= 0 {
        let _ = close(write_fd);
    }
    let _ = close(sigwinch_fd);

    result
}

fn run_inner(master_fd: RawFd, sigwinch_fd: RawFd) -> Result<()> {
    let stdin_fd: RawFd = libc::STDIN_FILENO;
    let mut buf = [0u8; 4096];

    loop {
        let mut pollfds = [
            libc::pollfd {
                fd: stdin_fd,
                events: libc::POLLIN,
                revents: 0,
            },
            libc::pollfd {
                fd: master_fd,
                events: libc::POLLIN,
                revents: 0,
            },
            libc::pollfd {
                fd: sigwinch_fd,
                events: libc::POLLIN,
                revents: 0,
            },
        ];

        let ret = unsafe { libc::poll(pollfds.as_mut_ptr(), pollfds.len() as libc::nfds_t, -1) };
        if ret < 0 {
            let err = io::Error::last_os_error();
            if err.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(err).context("poll");
        }

        // SIGWINCH — propagate terminal resize.
        if pollfds[2].revents & libc::POLLIN != 0 {
            // Drain the self-pipe.
            let mut drain = [0u8; 64];
            while let Ok(n) = read(sigwinch_fd, &mut drain) {
                if n == 0 {
                    break;
                }
            }
            if let Err(e) = sync_pty_size(master_fd) {
                log::warn!("resize_pty: {e}");
            }
        }

        // PTY master → stdout (shell output).
        if pollfds[1].revents & libc::POLLIN != 0 {
            match read(master_fd, &mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    io::stdout().write_all(&buf[..n]).context("write stdout")?;
                    io::stdout().flush().context("flush stdout")?;
                }
            }
        }

        // Child exited.
        if pollfds[1].revents & libc::POLLHUP != 0 {
            // Drain any remaining output.
            loop {
                match read(master_fd, &mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        let _ = io::stdout().write_all(&buf[..n]);
                    }
                }
            }
            let _ = io::stdout().flush();
            break;
        }

        // stdin → PTY master (user input).
        if pollfds[0].revents & libc::POLLIN != 0 {
            let n = io::stdin().read(&mut buf).context("read stdin")?;
            if n == 0 {
                break;
            }
            nix_write(master_fd, &buf[..n]).context("write to pty")?;
        }
    }

    Ok(())
}
