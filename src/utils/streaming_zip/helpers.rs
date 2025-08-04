use chrono::{Datelike, Timelike};
use std::io;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::AsyncWrite;

/// Helper for no compression
pub struct NoCompressionWriter<'a, W: AsyncWrite + Unpin> {
    pub inner: &'a mut W,
}

impl<'a, W: AsyncWrite + Unpin> AsyncWrite for NoCompressionWriter<'a, W> {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<io::Result<usize>> {
        std::pin::Pin::new(&mut self.inner).poll_write(cx, buf)
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        std::pin::Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        std::pin::Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

/// Convert time to DOS format
pub fn dos_time(time: Option<SystemTime>) -> (u16, u16) {
    let time = time.unwrap_or_else(SystemTime::now);
    let secs_since_epoch = time
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Convert to DOS time format using the non-deprecated method
    let datetime = chrono::DateTime::<chrono::Utc>::from_timestamp(secs_since_epoch as i64, 0)
        .unwrap_or_else(|| chrono::DateTime::<chrono::Utc>::from_timestamp(0, 0).unwrap())
        .naive_utc();

    let year = datetime.year() as u16;
    let month = datetime.month() as u16;
    let day = datetime.day() as u16;
    let hour = datetime.hour() as u16;
    let minute = datetime.minute() as u16;
    let second = datetime.second() as u16;

    let date = ((year - 1980) << 9) | (month << 5) | day;
    let time = (hour << 11) | (minute << 5) | (second >> 1);

    (time, date)
}
