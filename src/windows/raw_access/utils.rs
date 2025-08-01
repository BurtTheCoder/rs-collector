use winapi::shared::minwindef::FILETIME;
use winapi::um::minwinbase::SYSTEMTIME;
use winapi::um::timezoneapi::FileTimeToSystemTime;
use winapi::um::sysinfoapi::GetSystemTimeAsFileTime;

/// Convert Windows FILETIME to ISO-8601 string
pub fn filetime_to_iso8601(ft: &FILETIME) -> String {
    let mut system_time = SYSTEMTIME {
        wYear: 0,
        wMonth: 0,
        wDayOfWeek: 0,
        wDay: 0,
        wHour: 0,
        wMinute: 0,
        wSecond: 0,
        wMilliseconds: 0,
    };
    
    let result = unsafe {
        FileTimeToSystemTime(ft, &mut system_time)
    };
    
    if result == 0 {
        // If conversion fails, return current time
        return chrono::Utc::now().to_rfc3339();
    }
    
    // Convert to chrono DateTime
    let date = chrono::NaiveDate::from_ymd_opt(
        system_time.wYear as i32,
        system_time.wMonth as u32,
        system_time.wDay as u32,
    ).unwrap_or_else(|| {
        // This should never fail since we're using known valid values
        chrono::NaiveDate::from_ymd_opt(1970, 1, 1)
            .expect("Failed to create epoch date - this should never happen")
    });
    
    let time = chrono::NaiveTime::from_hms_milli_opt(
        system_time.wHour as u32,
        system_time.wMinute as u32,
        system_time.wSecond as u32,
        system_time.wMilliseconds as u32,
    ).unwrap_or_else(|| {
        // This should never fail since we're using known valid values
        chrono::NaiveTime::from_hms_opt(0, 0, 0)
            .expect("Failed to create midnight time - this should never happen")
    });
    
    let naive_dt = chrono::NaiveDateTime::new(date, time);
    let dt = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(naive_dt, chrono::Utc);
    
    dt.to_rfc3339()
}

/// Get current time as FILETIME
pub fn get_current_filetime() -> FILETIME {
    let mut ft = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    
    unsafe {
        GetSystemTimeAsFileTime(&mut ft);
    }
    
    ft
}
