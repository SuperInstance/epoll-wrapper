//! Epoll wrapper for Linux I/O multiplexing
//!
//! High-level safe wrapper around the Linux epoll(7) API.

use std::os::unix::io::{AsRawFd, RawFd};

/// Epoll event types
#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum EpollEvent {
    In = libc::EPOLLIN as u32,
    Out = libc::EPOLLOUT as u32,
    Err = libc::EPOLLERR as u32,
    Hup = libc::EPOLLHUP as u32,
    EdgeTriggered = libc::EPOLLET as u32,
}

/// Epoll instance wrapper
pub struct Epoll {
    fd: RawFd,
}

impl Epoll {
    /// Create a new epoll instance
    pub fn new() -> std::io::Result<Self> {
        let fd = unsafe { libc::epoll_create1(0) };
        if fd < 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(Self { fd })
    }

    /// Add a file descriptor to the epoll instance
    pub fn add(&self, fd: RawFd, events: &[EpollEvent], data: u64) -> std::io::Result<()> {
        let mut ev: libc::epoll_event = unsafe { std::mem::zeroed() };
        ev.events = events.iter().map(|e| *e as u32).fold(0, |a, b| a | b);
        ev.u64 = data;
        let ret = unsafe { libc::epoll_ctl(self.fd, libc::EPOLL_CTL_ADD, fd, &mut ev) };
        if ret < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    /// Wait for events
    pub fn wait(&self, events: &mut [libc::epoll_event], timeout_ms: i32) -> std::io::Result<usize> {
        let ret = unsafe {
            libc::epoll_wait(self.fd, events.as_mut_ptr(), events.len() as i32, timeout_ms)
        };
        if ret < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(ret as usize)
        }
    }

    /// Modify an existing descriptor
    pub fn modify(&self, fd: RawFd, events: &[EpollEvent], data: u64) -> std::io::Result<()> {
        let mut ev: libc::epoll_event = unsafe { std::mem::zeroed() };
        ev.events = events.iter().map(|e| *e as u32).fold(0, |a, b| a | b);
        ev.u64 = data;
        let ret = unsafe { libc::epoll_ctl(self.fd, libc::EPOLL_CTL_MOD, fd, &mut ev) };
        if ret < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    /// Remove a file descriptor
    pub fn delete(&self, fd: RawFd) -> std::io::Result<()> {
        let ret = unsafe { libc::epoll_ctl(self.fd, libc::EPOLL_CTL_DEL, fd, std::ptr::null_mut()) };
        if ret < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
}

impl AsRawFd for Epoll {
    fn as_raw_fd(&self) -> RawFd { self.fd }
}

impl Drop for Epoll {
    fn drop(&mut self) {
        unsafe { libc::close(self.fd); }
    }
}
