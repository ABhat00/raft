use std::{
    fs::{self, File},
    io::{Result, Write},
};

pub trait Logger {
    fn log(&mut self, to_write: String) -> Result<()>;
}

pub struct FileLogger {
    file: File,
}

pub fn new_file_logger<'a>(path: String) -> FileLogger {
    let file = fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(path.to_string())
        .expect(&format!("could not open file {}", path.to_string()));

    file.set_len(0);

    return FileLogger { file: file };
}

impl Logger for FileLogger {
    fn log(&mut self, to_write: String) -> Result<()> {
        writeln!(self.file, "{}", to_write)
    }
}
