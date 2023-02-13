use std::{fs, io};
use core::cmp;

use serde::Deserialize;
use serde::de::Deserializer;

#[derive(Debug)]
pub enum LogSeverity {
    Default,
    Debug,
    Info,
    Notice,
    Warning,
    Error,
    Critical,
    Alert,
    Emergency
}

impl LogSeverity {
    pub fn from_text(text: &str) -> Option<Self> {
        if text.eq_ignore_ascii_case("default") {
            Some(LogSeverity::Default)
        } else if text.eq_ignore_ascii_case("debug") {
            Some(LogSeverity::Debug)
        } else if text.eq_ignore_ascii_case("info") {
            Some(LogSeverity::Info)
        } else if text.eq_ignore_ascii_case("notice") {
            Some(LogSeverity::Notice)
        } else if text.eq_ignore_ascii_case("warning") {
            Some(LogSeverity::Warning)
        } else if text.eq_ignore_ascii_case("error") {
            Some(LogSeverity::Error)
        } else if text.eq_ignore_ascii_case("critical") {
            Some(LogSeverity::Critical)
        } else if text.eq_ignore_ascii_case("emergency") {
            Some(LogSeverity::Emergency)
        } else {
            None
        }
    }
}

struct LogSeverityVisitor;

impl<'de> serde::de::Visitor<'de> for LogSeverityVisitor {
    type Value = LogSeverity;

    #[inline(always)]
    fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
        formatter.write_str("a log severity")
    }

    #[inline]
    fn visit_str<E: serde::de::Error>(self, text: &str) -> Result<Self::Value, E> {
        match LogSeverity::from_text(text) {
            Some(result) => Ok(result),
            None => Err(serde::de::Error::invalid_value(serde::de::Unexpected::Str(text), &self))
        }
    }
}

impl<'de> Deserialize<'de> for LogSeverity {
    #[inline]
    fn deserialize<D: Deserializer<'de>>(des: D) -> Result<Self, D::Error> {
        des.deserialize_str(LogSeverityVisitor)
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    #[serde(default)]
    pub text_payload: String,
    pub timestamp: String,
    pub severity: LogSeverity,
    pub log_name: String,
    #[serde(default)]
    pub labels: serde_json::Map<String, serde_json::Value>,
}

enum IterState {
    NotStarted,
    Started,
    Finished,
}

const ARRAY_START: u8 = b'[';
const ARRAY_SEP: u8 = b',';
const ARRAY_END: u8 = b']';
const CHUNK_SIZE: usize = 4098;

pub struct LogEntryIter<T> {
    buffer: Vec<u8>,
    buffer_offset: usize,
    inner: T,
    state: IterState
}

impl<T: io::Read> LogEntryIter<T> {
    pub const fn new(inner: T) -> Self {
        Self {
            buffer: Vec::new(),
            buffer_offset: 0,
            inner,
            state: IterState::NotStarted,
        }
    }

    #[inline]
    pub fn extract_single_value(&mut self) -> io::Result<LogEntry> {
        match serde_json::Deserializer::from_reader(self).into_iter().next() {
            Some(Ok(entry)) => Ok(entry),
            Some(Err(error)) => Err(error.into()),
            None => Err(io::Error::new(io::ErrorKind::UnexpectedEof, "unexpected eof")),
        }
    }

    fn seek_until_byte(&mut self, expected: &[u8]) -> io::Result<Option<u8>> {
        if self.buffer.len() < CHUNK_SIZE {
            self.buffer.resize(CHUNK_SIZE, 0);
        }

        loop {
            let buffer = self.buffer.as_mut_slice();
            let read_size = match self.inner.read(buffer) {
                Ok(0) => break Ok(None),
                Ok(read_size) => read_size,
                Err(error) => break Err(error),
            };
            let read_buf = &buffer[..read_size];

            for (idx, byte) in read_buf.iter().enumerate() {
                for expected in expected {
                    if expected == byte {
                        self.buffer_offset = idx + 1;
                        return Ok(Some(*expected));
                    }
                }
            }

            if read_buf.len() == self.buffer.len() {
                self.buffer.resize(self.buffer.len() + CHUNK_SIZE, 0);
            }
        }
    }

    ///Read buffer until start of array
    pub fn seek_until_start(&mut self) -> io::Result<bool> {
        match self.seek_until_byte(&[ARRAY_START])? {
            Some(_) => Ok(true),
            None => Ok(false)
        }
    }

    pub fn seek_until_delim(&mut self) -> io::Result<bool> {
        let read_buf = &self.buffer[self.buffer_offset..];

        //Check if internal buffer already has part of next chunk
        if let Some(start) = read_buf.iter().position(|&byte| byte == ARRAY_SEP) {
            self.buffer_offset = start + 1;
            return Ok(true);
        }

        //If not read, until we encounter it, skipping everything else
        //this function should be only called after consuming 1 json element within array
        self.trim_until_offset();
        match self.seek_until_byte(&[ARRAY_SEP, ARRAY_END])? {
            Some(ARRAY_SEP) => Ok(true),
            _ => Ok(false),
        }
    }

    pub fn trim_until_offset(&mut self) {
        if self.buffer_offset > 0 {
            let new_len = self.buffer.len() - self.buffer_offset;
            if new_len > 0 {
                self.buffer.copy_within(self.buffer_offset.., 0);
                unsafe {
                    self.buffer.set_len(new_len);
                }
            } else {
                self.buffer.clear();
            }
            self.buffer_offset = 0;
        }
    }
}

impl<T: io::Read> Iterator for LogEntryIter<T> {
    type Item = io::Result<LogEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.state {
            IterState::NotStarted => {
                match self.seek_until_start() {
                    Ok(true) => {
                        self.state = IterState::Started;
                        match self.extract_single_value() {
                            Ok(value) => {
                                self.trim_until_offset();
                                Some(Ok(value))
                            },
                            Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => {
                                self.state = IterState::Finished;
                                Some(Err(error))
                            },
                            Err(error) => Some(Err(error)),
                        }
                    },
                    Ok(false) => None,
                    Err(error) => Some(Err(error)),
                }
            },
            IterState::Started => {
                match self.seek_until_delim() {
                    Ok(true) => {
                        match self.extract_single_value() {
                            Ok(value) => {
                                self.trim_until_offset();
                                Some(Ok(value))
                            },
                            Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => {
                                self.state = IterState::Finished;
                                Some(Err(error))
                            },
                            Err(error) => Some(Err(error)),
                        }
                    },
                    Ok(false) => {
                        self.state = IterState::Finished;
                        None
                    },
                    Err(error) => Some(Err(error)),
                }
            },
            IterState::Finished => None,
        }
    }
}

impl<T: io::Read> io::Read for LogEntryIter<T> {
    fn read(&mut self, mut out: &mut [u8]) -> io::Result<usize> {
        let read_buffer = &self.buffer[self.buffer_offset..];
        if read_buffer.len() > 0 {
            let read_size = cmp::min(read_buffer.len(), out.len());
            out[..read_size].copy_from_slice(&read_buffer[..read_size]);
            self.buffer_offset += read_size;

            out = &mut out[read_size..];
            if out.is_empty() {
                Ok(read_size)
            } else {
                io::Read::read(&mut self.inner, out).map(|size| size + read_size)
            }
        } else {
            io::Read::read(&mut self.inner, out)
        }
    }
}

pub fn read_file(path: &str) -> io::Result<LogEntryIter<fs::File>> {
    let file = fs::File::open(path)?;
    Ok(LogEntryIter::new(file))
}
