use thiserror::Error;
use tracing::trace;
use windows::Win32::Foundation::HANDLE;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to find kernel32.dll")]
    Kernel32NotFound(windows::core::Error),
    #[error("Failed to find LoadLibraryW")]
    LoadLibraryNotFound,
    #[error("Failed to allocate memory")]
    VirtualAllocEx(windows::core::Error),
    #[error("Failed to write process memory")]
    WriteProcessMemory(windows::core::Error),
    #[error("Failed to create remote thread")]
    CreateRemoteThread(windows::core::Error),
    #[error("Failed to free memory")]
    VirtualFreeEx(windows::core::Error),
}

pub unsafe fn inject(handle: HANDLE, path: &str) -> Result<(), Error> {
    use windows::core::PCSTR;
    use windows::Win32::System::Diagnostics::Debug::WriteProcessMemory;
    use windows::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};
    use windows::Win32::System::Memory::{
        VirtualAllocEx, VirtualFreeEx, MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE,
    };
    use windows::Win32::System::Threading::{CreateRemoteThread, WaitForSingleObject};

    // Allocate memory for the path string.
    let mut bytes = path.encode_utf16().collect::<Vec<_>>();
    bytes.push(0);

    // Look up the LoadLibraryW function in kernel32.dll.
    let kernel32 = GetModuleHandleA(PCSTR::from_raw("kernel32.dll\0".as_ptr()))
        .map_err(|x| Error::Kernel32NotFound(x))?;
    trace!("kernel32 addr: {:?}", kernel32.0);
    let load_library = GetProcAddress(kernel32, PCSTR::from_raw("LoadLibraryW\0".as_ptr()))
        .ok_or(Error::LoadLibraryNotFound)?;
    trace!("load_library addr: {:?}", load_library as *const ());

    // Allocate memory for the path string.
    let addr = VirtualAllocEx(
        handle,
        None,
        bytes.len(),
        MEM_COMMIT | MEM_RESERVE,
        PAGE_READWRITE,
    );
    trace!("allocated mem: {:?}", addr);

    // Make sure its succeeded.
    if addr.is_null() {
        return Err(Error::VirtualAllocEx(windows::core::Error::from_win32()));
    }

    // Write the path string to the allocated memory.
    WriteProcessMemory(handle, addr, bytes.as_ptr() as _, bytes.len() * 2, None)
        .map_err(|x| Error::WriteProcessMemory(x))?;

    // Create a remote thread to call LoadLibraryW.
    let thread_handle = CreateRemoteThread(
        handle,
        None,
        0,
        Some(std::mem::transmute(load_library)),
        Some(addr as _),
        0,
        None,
    )
    .map_err(|x| Error::CreateRemoteThread(x))?;
    trace!("thread_handle: {:?}", thread_handle);

    // Wait for the thread to finish.
    WaitForSingleObject(thread_handle, 0xFFFFFFFF);

    // Free the allocated memory.
    VirtualFreeEx(handle, addr, 0, MEM_RELEASE).map_err(|x| Error::VirtualFreeEx(x))?;

    // We are done here.
    Ok(())
}
