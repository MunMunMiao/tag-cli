use std::io;
use std::sync::{Arc, Mutex};
use tracing_subscriber::fmt;

/// A writer that appends to a shared in-memory buffer. Cloneable so it can be
/// used as both the [`MakeWriter`] and the [`Writer`](io::Write) it produces.
#[derive(Clone)]
struct TestWriter(Arc<Mutex<Vec<u8>>>);

impl io::Write for TestWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for TestWriter {
    type Writer = Self;

    fn make_writer(&self) -> Self::Writer {
        self.clone()
    }
}

/// Runs `f` with a default tracing subscriber that writes log output into an
/// in-memory buffer, then returns the result of `f` together with the captured
/// log output as a UTF-8 string.
pub fn capture_logs<F, R>(f: F) -> (R, String)
where
    F: FnOnce() -> R,
{
    let writer = TestWriter(Arc::new(Mutex::new(Vec::new())));
    let buf = writer.0.clone();
    let subscriber = fmt()
        .with_writer(writer)
        .with_ansi(false)
        .without_time()
        .with_level(false)
        .with_target(false)
        .finish();
    let result = tracing::subscriber::with_default(subscriber, f);
    let logs = String::from_utf8(buf.lock().unwrap().clone()).unwrap();
    (result, logs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_writer_flush_succeeds() {
        let mut writer = TestWriter(Arc::new(Mutex::new(Vec::new())));
        assert!(writer.flush().is_ok());
    }
}
