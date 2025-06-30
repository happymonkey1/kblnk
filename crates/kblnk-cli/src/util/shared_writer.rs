use std::io::Write;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct SharedWriter {
    inner: Arc<Mutex<Box<dyn Write + Send + 'static>>>
}

impl SharedWriter {
    pub fn new<W>(writer: W) -> Self 
    where
        W: Write + Send + 'static
    {
        Self {
            inner: Arc::new(Mutex::new(Box::new(writer)))
        }
    }
    
    pub fn stdout() -> Self {
        Self::new(std::io::stdout())
    }
    
    pub fn stderr() -> Self {
        Self::new(std::io::stderr())
    }
    
    pub fn null() -> Self {
        Self::new(NullWriter {})
    }
}

impl std::fmt::Debug for SharedWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharedWriter").finish()
    }
}

impl Write for SharedWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.inner.lock().expect("Mutex locks").write(buf)
    }
    
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.lock().expect("Mutex locks").flush()
    }
}

#[derive(Clone, Debug)]
pub struct NullWriter {}

impl Write for NullWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Ok(buf.len())
    }
    
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}