use std::{fs, io, path::PathBuf};

pub struct File {
    path: PathBuf,
    pub content: String,
}

impl File {
    pub fn at_path(path: PathBuf) -> io::Result<Self> {
        let content = fs::read_to_string(&path)?;
        Ok(Self { path, content })
    }

    pub fn atomic_overwrite(self, content: &str) -> io::Result<()> {
        let mut tmp_path = self.path.clone();
        tmp_path.set_extension("tmp.md");
        fs::write(&tmp_path, content)?;
        fs::rename(tmp_path, self.path)?;
        Ok(())
    }
}
