use windows::Win32::System::SystemInformation::{
    GlobalMemoryStatusEx, MEMORYSTATUSEX,
};

use crate::domain::memory::MemorySnapshot;
use crate::error::CollectorError;
use crate::platform::windows::pdh::{PdhQuery, PdhCounter};

pub struct MemoryCollector {
    query: PdhQuery,
    commit_used_counter: Option<PdhCounter>,
    commit_limit_counter: Option<PdhCounter>,
    cache_counter: Option<PdhCounter>,
}

impl MemoryCollector {
    pub fn new() -> Result<Self, CollectorError> {
        let query = PdhQuery::new()?;
        
        let commit_used_counter = query.add_counter(r"\Memory\Committed Bytes").ok();
        let commit_limit_counter = query.add_counter(r"\Memory\Commit Limit").ok();
        let cache_counter = query.add_counter(r"\Memory\Cache Bytes").ok();

        Ok(Self {
            query,
            commit_used_counter,
            commit_limit_counter,
            cache_counter,
        })
    }

    pub fn collect(&mut self) -> Result<MemorySnapshot, CollectorError> {
        let mut mem_status = MEMORYSTATUSEX::default();
        mem_status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
        
        let result = unsafe { GlobalMemoryStatusEx(&mut mem_status) };
        if result == 0 {
            return Err(CollectorError::Windows("GlobalMemoryStatusEx failed".into()));
        }

        self.query.collect()?;

        let total_bytes = mem_status.ullTotalPhys;
        let available_bytes = mem_status.ullAvailPhys;
        let used_bytes = total_bytes.saturating_sub(available_bytes);

        let commit_used_bytes = self.commit_used_counter
            .as_ref()
            .and_then(|c| c.get_double().ok())
            .map(|v| v as u64)
            .unwrap_or(used_bytes);

        let commit_limit_bytes = self.commit_limit_counter
            .as_ref()
            .and_then(|c| c.get_double().ok())
            .map(|v| v as u64)
            .unwrap_or(total_bytes);

        let cached_bytes = self.cache_counter
            .as_ref()
            .and_then(|c| c.get_double().ok())
            .map(|v| v as u64);

        Ok(MemorySnapshot {
            total_bytes,
            used_bytes,
            available_bytes,
            commit_used_bytes,
            commit_limit_bytes,
            page_file_used_bytes: None,
            page_file_total_bytes: None,
            cached_bytes,
        })
    }
}