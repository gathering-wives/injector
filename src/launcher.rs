use thiserror::Error;
use tracing::{debug, trace};
use windows::Win32::System::Threading::PROCESS_INFORMATION;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to get explorer PID")]
    GetExplorerPidFailed,
    #[error("Failed to open process token")]
    OpenProcessTokenFailed(windows::core::Error),
    #[error("Failed to create process")]
    CreateProcessFailed(windows::core::Error),
    #[error("Failed to initialize process thread attribute list")]
    InitializeProcThreadAttributeListFailed(windows::core::Error),
    #[error("Failed to update process thread attribute")]
    UpdateProcThreadAttributeFailed(windows::core::Error),
}

pub unsafe fn launch(
    path: &String,
    _cmdline: Option<&Vec<String>>,
    cwd: Option<&String>,
) -> Result<PROCESS_INFORMATION, Error> {
    use std::path::PathBuf;
    use windows::core::{HSTRING, PWSTR};
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::Security::TOKEN_ALL_ACCESS;
    use windows::Win32::System::Threading::{
        CreateProcessAsUserW, GetCurrentProcess, InitializeProcThreadAttributeList, OpenProcess,
        OpenProcessToken, UpdateProcThreadAttribute, CREATE_SUSPENDED,
        EXTENDED_STARTUPINFO_PRESENT, LPPROC_THREAD_ATTRIBUTE_LIST, PROCESS_ALL_ACCESS,
        PROCESS_INFORMATION, PROC_THREAD_ATTRIBUTE_PARENT_PROCESS, STARTUPINFOEXW,
    };

    // Obtain PID of explorer.exe
    let explorer_pid = get_explorer_pid().ok_or(Error::GetExplorerPidFailed)?;
    trace!("explorer pid: {}", explorer_pid);

    // Open current process token.
    let mut token: HANDLE = std::mem::zeroed();
    OpenProcessToken(GetCurrentProcess(), TOKEN_ALL_ACCESS, &raw mut token)
        .map_err(Error::OpenProcessTokenFailed)?;
    trace!("current process token: {:?}", token);

    // Open handle to explorer.exe
    // TODO: proper access rights
    let explorer_handle =
        OpenProcess(PROCESS_ALL_ACCESS, false, explorer_pid).map_err(Error::CreateProcessFailed)?;
    trace!("explorer handle: {:?}", explorer_handle);

    // Initialize process thread attribute list.
    let mut attr_size = 0;
    let _ = InitializeProcThreadAttributeList(
        LPPROC_THREAD_ATTRIBUTE_LIST(std::ptr::null_mut()),
        1,
        0,
        &raw mut attr_size,
    );
    trace!("attr size: {}", attr_size);

    // Populate process thread attribute list.
    let attribute_list = vec![0u8; attr_size];
    let attribute_list_ptr = LPPROC_THREAD_ATTRIBUTE_LIST(attribute_list.as_ptr() as _);
    InitializeProcThreadAttributeList(attribute_list_ptr, 1, 0, &raw mut attr_size)
        .map_err(Error::InitializeProcThreadAttributeListFailed)?;

    // FIXME: fix attribute
    UpdateProcThreadAttribute(
        attribute_list_ptr,
        0,
        PROC_THREAD_ATTRIBUTE_PARENT_PROCESS as _,
        Some(&raw const explorer_handle as _),
        8,
        None,
        None,
    )
    .map_err(Error::UpdateProcThreadAttributeFailed)?;

    // Prepare structs.
    let mut pi = PROCESS_INFORMATION::default();
    let mut si = STARTUPINFOEXW::default();
    si.StartupInfo.cb = std::mem::size_of::<STARTUPINFOEXW>() as u32;
    si.lpAttributeList = attribute_list_ptr;

    // Resolve current working directory.
    let cwd_buf = match cwd {
        Some(cwd) => PathBuf::from(cwd),
        None => PathBuf::from(path).parent().unwrap().to_path_buf(),
    };
    trace!("cwd: {}", cwd_buf.as_path().display());

    // TODO: support command line args

    // Create process.
    CreateProcessAsUserW(
        token,
        &HSTRING::from(path),
        PWSTR(std::ptr::null_mut()),
        // PWSTR(cmdline.as_ptr() as _),
        None,
        None,
        false,
        EXTENDED_STARTUPINFO_PRESENT | CREATE_SUSPENDED,
        None,
        &HSTRING::from(cwd_buf.as_path()),
        &raw const si as _,
        &raw mut pi,
    )
    .map_err(Error::CreateProcessFailed)?;

    debug!(
        "Process ID: {} (Handle: {:?})",
        pi.dwProcessId, pi.hProcess.0
    );
    debug!("Thread ID: {} (Handle: {:?})", pi.dwThreadId, pi.hThread.0);

    // We are done here.
    Ok(pi)
}

pub unsafe fn resume_process(pi: &PROCESS_INFORMATION) {
    use windows::Win32::System::Threading::ResumeThread;
    ResumeThread(pi.hThread);
}

pub unsafe fn free_info(pi: PROCESS_INFORMATION) {
    use windows::Win32::Foundation::CloseHandle;
    _ = CloseHandle(pi.hProcess);
    _ = CloseHandle(pi.hThread);
}

fn get_explorer_pid() -> Option<u32> {
    use windows::Win32::UI::WindowsAndMessaging::{GetShellWindow, GetWindowThreadProcessId};

    let hwnd = unsafe { GetShellWindow() };
    let mut pid = 0;

    let tid = unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    if tid == 0 {
        return None;
    }

    Some(pid)
}
