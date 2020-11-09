use json_dotpath::DotPaths;
use serde_json::Value;
use slog::{o, Error, Key, OwnedKVList, Record, Value as SlogValue, KV};
use std::cell::RefCell;
use std::fmt::Arguments;
use std::io;

pub struct NestedJsonFmt<W: io::Write> {
    writer: RefCell<W>,
}

impl<W: io::Write> NestedJsonFmt<W> {
    pub fn new(writer: W) -> Self {
        NestedJsonFmt {
            writer: RefCell::new(writer),
        }
    }
}

struct NestedJsonFmtSerializer<'a, W: io::Write> {
    writer: &'a mut W,
    initial_value: Value,
}

impl<'a, W> slog::Serializer for NestedJsonFmtSerializer<'a, W>
where
    W: io::Write,
{
    fn emit_usize(&mut self, key: slog::Key, val: usize) -> Result<(), Error> {
        self.initial_value
            .dot_set(key, val)
            .map_err(|_| slog::Error::Other)
    }

    fn emit_isize(&mut self, key: slog::Key, val: isize) -> Result<(), Error> {
        self.initial_value
            .dot_set(key, val)
            .map_err(|_| slog::Error::Other)
    }

    fn emit_bool(&mut self, key: slog::Key, val: bool) -> Result<(), Error> {
        self.initial_value
            .dot_set(key, val)
            .map_err(|_| slog::Error::Other)
    }

    fn emit_char(&mut self, key: slog::Key, val: char) -> Result<(), Error> {
        self.initial_value
            .dot_set(key, val)
            .map_err(|_| slog::Error::Other)
    }

    fn emit_u8(&mut self, key: slog::Key, val: u8) -> Result<(), Error> {
        self.initial_value
            .dot_set(key, val)
            .map_err(|_| slog::Error::Other)
    }

    fn emit_i8(&mut self, key: slog::Key, val: i8) -> Result<(), Error> {
        self.initial_value
            .dot_set(key, val)
            .map_err(|_| slog::Error::Other)
    }

    fn emit_u16(&mut self, key: slog::Key, val: u16) -> Result<(), Error> {
        self.initial_value
            .dot_set(key, val)
            .map_err(|_| slog::Error::Other)
    }

    fn emit_i16(&mut self, key: slog::Key, val: i16) -> Result<(), Error> {
        self.initial_value
            .dot_set(key, val)
            .map_err(|_| slog::Error::Other)
    }

    fn emit_u32(&mut self, key: slog::Key, val: u32) -> Result<(), Error> {
        self.initial_value
            .dot_set(key, val)
            .map_err(|_| slog::Error::Other)
    }

    fn emit_i32(&mut self, key: slog::Key, val: i32) -> Result<(), Error> {
        self.initial_value
            .dot_set(key, val)
            .map_err(|_| slog::Error::Other)
    }

    fn emit_f32(&mut self, key: slog::Key, val: f32) -> Result<(), Error> {
        self.initial_value
            .dot_set(key, val)
            .map_err(|_| slog::Error::Other)
    }

    fn emit_u64(&mut self, key: slog::Key, val: u64) -> Result<(), Error> {
        self.initial_value
            .dot_set(key, val)
            .map_err(|_| slog::Error::Other)
    }

    fn emit_i64(&mut self, key: slog::Key, val: i64) -> Result<(), Error> {
        self.initial_value
            .dot_set(key, val)
            .map_err(|_| slog::Error::Other)
    }

    fn emit_f64(&mut self, key: slog::Key, val: f64) -> Result<(), Error> {
        self.initial_value
            .dot_set(key, val)
            .map_err(|_| slog::Error::Other)
    }

    fn emit_u128(&mut self, key: slog::Key, val: u128) -> Result<(), Error> {
        self.initial_value
            .dot_set(key, val)
            .map_err(|_| slog::Error::Other)
    }

    fn emit_i128(&mut self, key: slog::Key, val: i128) -> Result<(), Error> {
        self.initial_value
            .dot_set(key, val)
            .map_err(|_| slog::Error::Other)
    }

    fn emit_str(&mut self, key: slog::Key, val: &str) -> Result<(), Error> {
        self.initial_value
            .dot_set(key, val)
            .map_err(|_| slog::Error::Other)
    }

    fn emit_unit(&mut self, key: slog::Key) -> Result<(), Error> {
        self.initial_value
            .dot_set(key, "()")
            .map_err(|_| slog::Error::Other)
    }

    fn emit_none(&mut self, key: slog::Key) -> Result<(), Error> {
        self.initial_value
            .dot_set(key, Value::Null)
            .map_err(|_| slog::Error::Other)
    }

    fn emit_arguments<'b>(&mut self, key: slog::Key, val: &Arguments<'b>) -> Result<(), Error> {
        let val = format!("{}", val);
        self.initial_value
            .dot_set(key, val)
            .map_err(|_| slog::Error::Other)
    }
}

impl<W> slog::Drain for NestedJsonFmt<W>
where
    W: io::Write,
{
    type Ok = ();
    type Err = io::Error;

    fn log<'a>(
        &self,
        record: &Record<'a>,
        logger_values: &OwnedKVList,
    ) -> Result<Self::Ok, Self::Err> {
        let mut writer = self.writer.borrow_mut();
        let mut ser = NestedJsonFmtSerializer {
            writer: &mut *writer,
            initial_value: Value::Null,
        };
        logger_values.serialize(record, &mut ser)?;
        record.kv().serialize(record, &mut ser)?;
        let json = serde_json::to_vec(&ser.initial_value)?;
        writer.write_all(&json)?;
        writer.write_all(b"\n")?;
        writer.flush()?;

        Ok(())
    }
}
#[cfg(test)]
pub mod tests {
    use super::*;
    use serde_json::json;
    use slog::{debug, o, Drain, Error, Logger, Serializer, KV};
    use std::io::Cursor;
    use std::str::from_utf8;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    struct LogCapture(Arc<Mutex<Cursor<Vec<u8>>>>);

    impl LogCapture {
        fn snapshot_buf(&self) -> Vec<u8> {
            let guard = self.0.lock().unwrap();
            (*guard).get_ref().clone()
        }

        fn snapshot_str(&self) -> String {
            let buf = self.snapshot_buf();
            from_utf8(&buf).unwrap().to_string()
        }
    }

    impl io::Write for LogCapture {
        fn write(&mut self, buf: &[u8]) -> Result<usize, io::Error> {
            let mut guard = self.0.lock().unwrap();
            (*guard).write(buf)
        }

        fn flush(&mut self) -> Result<(), io::Error> {
            let mut guard = self.0.lock().unwrap();
            (*guard).flush()
        }
    }

    #[test]
    fn logging_with_no_local_info_works() {
        let output = LogCapture::default();
        let drain = NestedJsonFmt::new(output.clone()).fuse();
        let drain = slog_async::Async::new(drain).build().fuse();
        let logger = Logger::root(drain, o!("ecs.version" => "1.6"));
        debug!(logger, "test msg";);

        drop(logger);
        assert_eq!(
            output.snapshot_str(),
            format!("{}\n", json!({"ecs": {"version": "1.6"}}).to_string())
        );
    }

    #[test]
    fn logging_with_local_info() {
        let output = LogCapture::default();
        let drain = NestedJsonFmt::new(output.clone()).fuse();
        let drain = slog_async::Async::new(drain).build().fuse();
        let logger = Logger::root(drain, o!("ecs.version" => "1.6"));
        debug!(logger, "test msg"; "event.kind" => "event");

        drop(logger);
        assert_eq!(
            output.snapshot_str(),
            format!(
                "{}\n",
                json!({"ecs": {"version": "1.6"}, "event":{"kind": "event"}}).to_string()
            )
        );
    }

    #[test]
    fn logging_with_same_info_twice_ignores_second_value() {
        let output = LogCapture::default();
        let drain = NestedJsonFmt::new(output.clone()).fuse();
        let drain = slog_async::Async::new(drain).build().fuse();
        let logger = Logger::root(drain, o!("ecs.version" => "1.6"));
        debug!(logger, "test msg"; "event.kind" => "event", "event.kind" => "error");

        drop(logger);
        assert_eq!(
            output.snapshot_str(),
            format!(
                "{}\n",
                json!({"ecs": {"version": "1.6"}, "event": {"kind": "event"}}).to_string()
            )
        );
    }

    #[test]
    fn adding_multiple_items_with_nesting_works() {
        let output = LogCapture::default();
        let drain = NestedJsonFmt::new(output.clone()).fuse();
        let drain = slog_async::Async::new(drain).build().fuse();
        let logger = Logger::root(drain, o!("ecs.version" => "1.6"));
        debug!(logger, "test msg"; "event.kind" => "event", "http.request.method" => "GET", "http.request.body.bytes" => "12345");

        drop(logger);
        assert_eq!(
            output.snapshot_str(),
            format!("{}\n", json!({"ecs": {"version": "1.6"}, "event": {"kind": "event"}, "http": {"request": {"method": "GET", "body": {"bytes": "12345"}}}}).to_string())
        );
    }
}
