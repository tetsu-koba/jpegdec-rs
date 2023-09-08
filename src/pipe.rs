use std::fs::File;
use std::io;
use std::io::Write;
use std::os::fd::AsRawFd;
use std::os::unix::io::RawFd;

// Check if the given file descriptor is a pipe
#[allow(dead_code)]
pub fn is_pipe_fd(fd: RawFd) -> bool {
    match nix::sys::stat::fstat(fd) {
        Ok(stat) => stat.st_mode & libc::S_IFMT == libc::S_IFIFO,
        Err(_) => false,
    }
}

// Check if the given file is a pipe
#[allow(dead_code)]
pub fn is_pipe(f: &File) -> bool {
    is_pipe_fd(f.as_raw_fd())
}

// Get pipe max buffer size
#[cfg(target_os = "linux")]
pub fn get_pipe_max_size() -> Result<usize, io::Error> {
    // Read the maximum pipe size
    let pipe_max_size = std::fs::read_to_string("/proc/sys/fs/pipe-max-size")?;
    let max_size: usize = pipe_max_size.trim_end().parse().map_err(|err| {
        eprintln!("Failed to parse /proc/sys/fs/pipe-max-size: {:?}", err);
        io::Error::new(io::ErrorKind::InvalidData, "Failed to parse max pipe size")
    })?;
    Ok(max_size)
}

// Set the size of the given pipe file descriptor to the maximum size
#[cfg(target_os = "linux")]
pub fn set_pipe_max_size_fd(fd: RawFd) -> Result<(), io::Error> {
    use nix::fcntl::{fcntl, FcntlArg};
    let max_size: libc::c_int = get_pipe_max_size()? as _;

    // If the current size is less than the maximum size, set the pipe size to the maximum size
    let current_size = fcntl(fd, FcntlArg::F_GETPIPE_SZ)?;
    if current_size < max_size {
        _ = fcntl(fd, FcntlArg::F_SETPIPE_SZ(max_size))?;
    }
    Ok(())
}

// Set the size of the given pipe file to the maximum size
#[cfg(target_os = "linux")]
pub fn set_pipe_max_size(f: &File) -> Result<(), io::Error> {
    set_pipe_max_size_fd(f.as_raw_fd())
}

#[cfg(target_os = "linux")]
pub fn vmsplice_single_buffer_fd(mut buf: &[u8], fd: RawFd) -> Result<(), io::Error> {
    use nix::fcntl::{vmsplice, SpliceFFlags};
    use std::io::IoSlice;

    if buf.is_empty() {
        return Ok(());
    };
    loop {
        let iov = IoSlice::new(buf);
        match vmsplice(fd, &[iov], SpliceFFlags::SPLICE_F_GIFT) {
            Ok(n) if n == iov.len() => return Ok(()),
            Ok(n) if n != 0 => buf = &buf[n..],
            Ok(_) => unreachable!(),
            Err(err) if err == nix::errno::Errno::EINTR => {}
            Err(err) => return Err(err.into()),
        }
    }
}

#[cfg(target_os = "linux")]
pub fn vmsplice_single_buffer(buf: &[u8], f: &File) -> Result<(), io::Error> {
    vmsplice_single_buffer_fd(buf, f.as_raw_fd())
}

#[cfg(target_os = "linux")]
pub struct Writer {
    file: File,
    is_pipe: bool,
}

#[cfg(target_os = "linux")]
impl Writer {
    pub fn new(file: File) -> Self {
        let is_pipe = is_pipe(&file);
        if is_pipe {
            _ = set_pipe_max_size(&file);
        }
        Self { file, is_pipe }
    }

    pub fn write_all(&mut self, data: &[u8]) -> Result<(), io::Error> {
        if self.is_pipe {
            vmsplice_single_buffer(data, &self.file)
        } else {
            use std::io::Write;
            self.file.write_all(data)
        }
    }
}

#[cfg(not(target_os = "linux"))]
pub struct Writer {
    file: File,
}

#[cfg(not(target_os = "linux"))]
impl Writer {
    pub fn new(file: File) -> Self {
        Self { file }
    }

    pub fn write_all(&mut self, data: &[u8]) -> Result<(), io::Error> {
        self.file.write_all(data)
    }
}
