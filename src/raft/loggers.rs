use std::{
    fs::{self, File},
    io::{Result, Write},
};

pub trait Logger: Send + Sync {
    fn log(&mut self, to_write: String) -> Result<()>;
}

pub struct FileLogger {
    file: File,
}

pub fn new_file_logger<'a>(path: String) -> FileLogger {
    let file = fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(&path)
        .expect(&format!("could not open file {}", path));

    file.set_len(0);

    FileLogger { file }
}

impl Logger for FileLogger {
    fn log(&mut self, to_write: String) -> Result<()> {
        writeln!(self.file, "{}", to_write)
    }
}

#[derive(Default)]
pub struct MemLogger {
    events: Vec<String>,
}

impl Logger for MemLogger {
    fn log(&mut self, to_write: String) -> Result<()> {
        Ok(self.events.push(format!("{}", to_write)))
    }
}
