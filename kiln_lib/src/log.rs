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
        record.kv().serialize(record, &mut ser)?;
        let json = serde_json::to_vec(&ser.initial_value)?;
        writer.write_all(&json)?;
        writer.write_all(b"\n")?;
        writer.flush()?;

        Ok(())
    }
}
