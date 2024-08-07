use std::io;



#[cfg(target_os = "windows")]
pub(crate) fn get_hostname() -> io::Result<String> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use winapi::um::sysinfoapi::{ComputerNameDnsHostname, GetComputerNameExW};
    
    let mut buffer: [u16; 256] = [0; 256];
    let mut size = buffer.len() as u32;

    let success = unsafe {
        GetComputerNameExW(ComputerNameDnsHostname, buffer.as_mut_ptr(), &mut size)
    };

    if success == 0 {
        return Err(io::Error::last_os_error());
    }

    let hostname = OsString::from_wide(&buffer[..size as usize]);
    Ok(hostname.to_string_lossy().into_owned())
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub(crate) fn get_hostname() -> io::Result<String> {
    use nix::unistd::gethostname;
    let result = gethostname()?;
    Ok(result.into_string().unwrap_or("".to_string()))
}
#[cfg(test)]
mod hostname_test {
    use super::get_hostname;

    #[test]
    fn test_get_hostname() {
        let hostname = get_hostname().unwrap();
       println!("hostname is {}", hostname);
    }
}