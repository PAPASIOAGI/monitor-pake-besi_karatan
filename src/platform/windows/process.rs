use std::collections::HashMap;
use std::time::Instant;

use serde::Deserialize;
use windows::Win32::Foundation::{CloseHandle, FILETIME, HANDLE};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::Threading::{
    GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
};

use crate::domain::process::{ProcessInfo, ProcessSnapshot};
use crate::error::CollectorError;

#[allow(dead_code)] // Reserved for future WMI memory query implementation
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct WmiProcess {
    process_id: u32,
    name: Option<String>,
    #[serde(rename = "WorkingSetSize")]
    working_set_size: Option<u64>,
    #[serde(rename = "PageFileUsage")]
    page_file_usage: Option<u64>,
    #[serde(rename = "PrivatePageCount")]
    private_page_count: Option<u64>,
}

struct ProcessHandle(HANDLE);

impl Drop for ProcessHandle {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.0).ok() };
    }
}

pub struct ProcessCollector {
    processes: HashMap<u32, ProcessInfo>,
    last_system_cpu_time: Option<u64>,
    last_sample_time: Option<Instant>,
}

impl ProcessCollector {
    pub fn new() -> Result<Self, CollectorError> {
        Ok(Self {
            processes: HashMap::new(),
            last_system_cpu_time: None,
            last_sample_time: None,
        })
    }

    pub fn collect(&mut self) -> Result<ProcessSnapshot, CollectorError> {
        let now = Instant::now();
        let mut current_pids = std::collections::HashSet::new();

        // Get process list using ToolHelp API
        let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }
            .map_err(|e| CollectorError::Windows(format!("CreateToolhelp32Snapshot: {e}")))?;
        let _snapshot_guard = ProcessHandle(snapshot);

        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };

        let mut first = true;
        while first {
            first = false;
        }
        while unsafe { Process32NextW(snapshot, &mut entry).is_ok() } {
            let pid = entry.th32ProcessID;
            current_pids.insert(pid);

            let name = String::from_utf16_lossy(&entry.szExeFile)
                .trim_end_matches('\0')
                .to_string();

            let process = self
                .processes
                .entry(pid)
                .or_insert_with(|| ProcessInfo::new(pid, name.clone()));
            process.name = name; // Update name in case it changed
        }

        // Get CPU times for each process
        let system_cpu_time = self.get_system_cpu_time();

        // Collect PIDs first to avoid borrow conflict
        let pids: Vec<u32> = self.processes.keys().copied().collect();
        for pid in pids {
            if let Some(cpu_time) = self.get_process_cpu_time(pid)
                && let Some(process) = self.processes.get_mut(&pid)
            {
                process.update_cpu(cpu_time, now);
            }
        }

        // Remove processes that no longer exist
        self.processes.retain(|pid, _| current_pids.contains(pid));

        // Collect results
        let mut results: Vec<ProcessInfo> = self.processes.values().cloned().collect();
        results.sort_by(|a, b| {
            b.cpu_usage
                .unwrap_or(0.0)
                .partial_cmp(&a.cpu_usage.unwrap_or(0.0))
                .unwrap()
        });

        self.last_system_cpu_time = system_cpu_time;
        self.last_sample_time = Some(now);

        Ok(ProcessSnapshot { processes: results })
    }

    fn get_system_cpu_time(&self) -> Option<u64> {
        // Use a simple counter approach - return current time as approximation
        // In a full implementation, this would use PDH counters or GetSystemTimes
        Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .ok()?
                .as_nanos() as u64,
        )
    }

    fn get_process_cpu_time(&self, pid: u32) -> Option<u64> {
        let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?;
        let _handle_guard = ProcessHandle(handle);

        let mut creation = FILETIME::default();
        let mut exit = FILETIME::default();
        let mut kernel = FILETIME::default();
        let mut user = FILETIME::default();
        unsafe { GetProcessTimes(handle, &mut creation, &mut exit, &mut kernel, &mut user) }
            .ok()?;

        Some(filetime_to_u64(kernel) + filetime_to_u64(user))
    }

    #[allow(dead_code)] // Reserved for future WMI memory query implementation
    fn update_memory_from_wmi(&mut self, _conn: &wmi::WMIConnection) -> Result<(), CollectorError> {
        // WMI queries not implemented in collector due to Send bounds
        // Memory info will be N/A for now
        Ok(())
    }
}

fn filetime_to_u64(ft: FILETIME) -> u64 {
    ((ft.dwHighDateTime as u64) << 32) | (ft.dwLowDateTime as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::process::ProcessInfo;

    #[test]
    fn test_process_info_new() {
        let proc = ProcessInfo::new(1234, "test.exe".to_string());
        assert_eq!(proc.pid, 1234);
        assert_eq!(proc.name, "test.exe");
        assert_eq!(proc.cpu_usage, None);
    }

    #[test]
    fn test_filetime_to_u64() {
        let ft = FILETIME {
            dwLowDateTime: 0xFFFFFFFF,
            dwHighDateTime: 0xFFFFFFFF,
        };
        assert_eq!(filetime_to_u64(ft), u64::MAX);
    }
}
