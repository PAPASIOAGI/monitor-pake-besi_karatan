use std::ptr;

use windows::Win32::System::Performance::{
    PDH_CSTATUS_VALID_DATA, PDH_FMT_COUNTERVALUE, PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE,
    PDH_HCOUNTER, PDH_HQUERY, PDH_MORE_DATA, PdhAddEnglishCounterW, PdhCloseQuery,
    PdhCollectQueryData, PdhGetFormattedCounterArrayW, PdhGetFormattedCounterValue, PdhOpenQueryW,
};
use windows::core::{PCWSTR, PWSTR};

use crate::error::CollectorError;

pub struct PdhQuery {
    handle: PDH_HQUERY,
}

// PDH query handles may move between threads; access remains serialized by &self/&mut collector.
unsafe impl Send for PdhQuery {}

impl PdhQuery {
    pub fn new() -> Result<Self, CollectorError> {
        let mut handle = PDH_HQUERY::default();
        let status = unsafe { PdhOpenQueryW(None, 0, &mut handle) };
        check(status, "PdhOpenQueryW")?;
        Ok(Self { handle })
    }

    pub fn add_counter(&self, path: &str) -> Result<PdhCounter, CollectorError> {
        let path = wide(path);
        let mut handle = PDH_HCOUNTER::default();
        let status =
            unsafe { PdhAddEnglishCounterW(self.handle, PCWSTR(path.as_ptr()), 0, &mut handle) };
        check(status, "PdhAddEnglishCounterW")?;
        Ok(PdhCounter { handle })
    }

    pub fn collect(&self) -> Result<(), CollectorError> {
        check(
            unsafe { PdhCollectQueryData(self.handle) },
            "PdhCollectQueryData",
        )
    }
}

impl Drop for PdhQuery {
    fn drop(&mut self) {
        // SAFETY: the query handle is owned by this object and closed exactly once.
        unsafe { PdhCloseQuery(self.handle) };
    }
}

pub struct PdhCounter {
    handle: PDH_HCOUNTER,
}

// PDH counter handles may move between threads; access remains serialized by &self/&mut collector.
unsafe impl Send for PdhCounter {}

impl PdhCounter {
    pub fn get_double(&self) -> Result<f64, CollectorError> {
        let mut value = PDH_FMT_COUNTERVALUE::default();
        let status =
            unsafe { PdhGetFormattedCounterValue(self.handle, PDH_FMT_DOUBLE, None, &mut value) };
        check(status, "PdhGetFormattedCounterValue")?;
        if value.CStatus != PDH_CSTATUS_VALID_DATA {
            return Err(CollectorError::Unavailable(format!(
                "counter status 0x{:X}",
                value.CStatus
            )));
        }
        // SAFETY: PDH_FMT_DOUBLE selects the double member of the union.
        Ok(unsafe { value.Anonymous.doubleValue })
    }

    pub fn get_instance_values(&self) -> Result<Vec<(String, f64)>, CollectorError> {
        let mut bytes = 0;
        let mut count = 0;
        let status = unsafe {
            PdhGetFormattedCounterArrayW(
                self.handle,
                PDH_FMT_DOUBLE,
                &mut bytes,
                &mut count,
                Some(ptr::null_mut()),
            )
        };
        if status != PDH_MORE_DATA {
            check(status, "PdhGetFormattedCounterArrayW (size)")?;
        }
        if bytes == 0 {
            return Ok(Vec::new());
        }

        let mut buffer = vec![0_u8; bytes as usize];
        let status = unsafe {
            PdhGetFormattedCounterArrayW(
                self.handle,
                PDH_FMT_DOUBLE,
                &mut bytes,
                &mut count,
                Some(buffer.as_mut_ptr().cast()),
            )
        };
        check(status, "PdhGetFormattedCounterArrayW")?;

        // SAFETY: PDH returned `count` contiguous wide counter items in this buffer.
        let items = unsafe {
            std::slice::from_raw_parts(
                buffer.as_ptr().cast::<PDH_FMT_COUNTERVALUE_ITEM_W>(),
                count as usize,
            )
        };
        Ok(items
            .iter()
            .filter(|item| item.FmtValue.CStatus == PDH_CSTATUS_VALID_DATA)
            .filter_map(|item| {
                let name = pwstr_to_string(item.szName)?;
                // SAFETY: PDH_FMT_DOUBLE selects the double member of the union.
                Some((name, unsafe { item.FmtValue.Anonymous.doubleValue }))
            })
            .collect())
    }
}

fn pwstr_to_string(value: PWSTR) -> Option<String> {
    if value.is_null() {
        return None;
    }
    // SAFETY: PDH supplies a null-terminated string whose lifetime is the result buffer.
    Some(unsafe { value.to_string().ok()? })
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

fn check(status: u32, operation: &str) -> Result<(), CollectorError> {
    if status == 0 {
        Ok(())
    } else {
        Err(CollectorError::Pdh(format!(
            "{operation} failed: 0x{status:X}"
        )))
    }
}
