use anyhow::{bail, Context, Result};
use glob::{glob_with, MatchOptions, Pattern};
use rusqlite::{params, Connection};
use std::{fs, path::Path, time::Duration};

#[derive(Clone)]
pub struct Entry {
    pub kind: String,
    pub value: String,
}

impl Entry {
    pub fn new(kind: &str, value: String) -> Self {
        Self {
            kind: kind.into(),
            value,
        }
    }

    pub fn source(arg: &str) -> Result<Self> {
        let path = Path::new(arg);
        let cwd = std::env::current_dir()?;
        if path.is_file() || !arg.contains(['*', '?', '[']) {
            let full = cwd.join(path);
            return Ok(Self::new(
                "file",
                full.to_str().context("path is not Unicode")?.into(),
            ));
        }
        let pattern = if path.is_absolute() {
            arg.into()
        } else {
            format!(
                "{}/{}",
                Pattern::escape(cwd.to_str().context("path is not Unicode")?),
                arg
            )
        };
        Ok(Self::new("glob", pattern))
    }
}

pub struct Rendered {
    pub text: String,
    pub files: Vec<String>,
}

pub fn render(entries: &[Entry], headings: bool, separator: &str) -> Result<Rendered> {
    let mut parts = Vec::new();
    let mut files = Vec::new();
    for entry in entries {
        let paths = match entry.kind.as_str() {
            "literal" | "capture" => {
                parts.push(entry.value.clone());
                continue;
            }
            "file" => vec![std::path::PathBuf::from(&entry.value)],
            "glob" => {
                let opts = MatchOptions {
                    case_sensitive: !cfg!(windows),
                    require_literal_separator: true,
                    require_literal_leading_dot: false,
                };
                let mut matches = Vec::new();
                for found in glob_with(&entry.value, opts)? {
                    let path = found?;
                    if path.is_file() {
                        matches.push(path);
                    }
                }
                matches.sort();
                if matches.is_empty() {
                    bail!("no files match {}", entry.value);
                }
                matches
            }
            kind => bail!("unrecognized entry type {kind}"),
        };
        for path in paths {
            let content = fs::read_to_string(&path)
                .with_context(|| format!("reading UTF-8 file {}", path.display()))?;
            let content = content.strip_prefix('\u{feff}').unwrap_or(&content);
            let name = path
                .file_name()
                .context("file has no name")?
                .to_string_lossy();
            parts.push(if headings {
                format!("## {name}\n\n{content}")
            } else {
                content.to_owned()
            });
            files.push(path.display().to_string());
        }
    }
    Ok(Rendered {
        text: parts.join(separator),
        files,
    })
}

pub struct Store(Connection);
impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent)?;
        }
        let db = Connection::open(path)?;
        db.busy_timeout(Duration::from_secs(5))?;
        db.execute_batch("CREATE TABLE IF NOT EXISTS entries (
            id INTEGER PRIMARY KEY, session TEXT NOT NULL, buffer TEXT NOT NULL,
            kind TEXT NOT NULL, value TEXT NOT NULL);
            CREATE INDEX IF NOT EXISTS entries_buffer ON entries(session, buffer, id);
            CREATE TABLE IF NOT EXISTS templates (name TEXT NOT NULL, position INTEGER NOT NULL,
            kind TEXT NOT NULL, value TEXT NOT NULL, PRIMARY KEY(name,position));
            CREATE TABLE IF NOT EXISTS snapshots (id INTEGER PRIMARY KEY, session TEXT NOT NULL,
            buffer TEXT NOT NULL, body TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS snapshot_sources (snapshot INTEGER NOT NULL, position INTEGER NOT NULL, path TEXT NOT NULL,
            PRIMARY KEY(snapshot,position));")?;
        Ok(Self(db))
    }
    pub fn add(&mut self, session: &str, buffer: &str, entries: &[Entry]) -> Result<()> {
        let tx = self.0.transaction()?;
        for e in entries {
            tx.execute(
                "INSERT INTO entries(session,buffer,kind,value) VALUES(?,?,?,?)",
                params![session, buffer, e.kind, e.value],
            )?;
        }
        tx.commit()?;
        Ok(())
    }
    pub fn read(&self, session: &str, buffer: &str) -> Result<Vec<Entry>> {
        let mut stmt = self
            .0
            .prepare("SELECT kind,value FROM entries WHERE session=? AND buffer=? ORDER BY id")?;
        let rows = stmt.query_map(params![session, buffer], |r| {
            Ok(Entry {
                kind: r.get(0)?,
                value: r.get(1)?,
            })
        })?;
        let entries = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        if entries.is_empty() {
            bail!("empty or unknown buffer {buffer} in session {session}");
        }
        Ok(entries)
    }
    pub fn clear(&self, session: &str, buffer: &str) -> Result<()> {
        self.0.execute(
            "DELETE FROM entries WHERE session=? AND buffer=?",
            params![session, buffer],
        )?;
        Ok(())
    }
    pub fn template_save(&mut self, name: &str, entries: &[Entry]) -> Result<()> {
        let tx = self.0.transaction()?;
        tx.execute("DELETE FROM templates WHERE name=?", [name])?;
        for (i, e) in entries.iter().enumerate() {
            tx.execute(
                "INSERT INTO templates VALUES(?,?,?,?)",
                params![name, i as i64, e.kind, e.value],
            )?;
        }
        tx.commit()?;
        Ok(())
    }
    pub fn template_read(&self, name: &str) -> Result<Vec<Entry>> {
        let mut stmt = self
            .0
            .prepare("SELECT kind,value FROM templates WHERE name=? ORDER BY position")?;
        let rows = stmt.query_map([name], |r| {
            Ok(Entry {
                kind: r.get(0)?,
                value: r.get(1)?,
            })
        })?;
        let entries = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        if entries.is_empty() {
            bail!("unknown template {name}");
        }
        Ok(entries)
    }
    pub fn snapshot(&mut self, session: &str, buffer: &str, rendered: &Rendered) -> Result<i64> {
        let tx = self.0.transaction()?;
        tx.execute(
            "INSERT INTO snapshots(session,buffer,body) VALUES(?,?,?)",
            params![session, buffer, rendered.text],
        )?;
        let id = tx.last_insert_rowid();
        for (i, path) in rendered.files.iter().enumerate() {
            tx.execute(
                "INSERT INTO snapshot_sources VALUES(?,?,?)",
                params![id, i as i64, path],
            )?;
        }
        tx.commit()?;
        Ok(id)
    }
    pub fn snapshot_text(&self, session: &str, id: i64) -> Result<String> {
        self.0
            .query_row(
                "SELECT body FROM snapshots WHERE session=? AND id=?",
                params![session, id],
                |r| r.get(0),
            )
            .context("snapshot not found in this session")
    }
    pub fn buffers(&self, session: &str) -> Result<Vec<String>> {
        let mut q = self
            .0
            .prepare("SELECT DISTINCT buffer FROM entries WHERE session=? ORDER BY buffer")?;
        let names = q.query_map([session], |r| r.get(0))?;
        Ok(names.collect::<rusqlite::Result<_>>()?)
    }
}

pub fn database_path() -> Result<std::path::PathBuf> {
    if let Some(p) = std::env::var_os("AICLIP_DB") {
        return Ok(p.into());
    }
    Ok(dirs::data_local_dir()
        .context("set AICLIP_DB to a writable database path")?
        .join("aiclip")
        .join("aiclip.sqlite"))
}
pub fn publish(text: &str) -> Result<()> {
    anyhow::ensure!(
        !text.contains('\0'),
        "clipboard text contains a NUL character"
    );
    arboard::Clipboard::new()?.set_text(text)?;
    Ok(())
}
impl Store {
    pub fn restore_new(&self, session: &str, buffer: &str, text: &str) -> Result<()> {
        let changed=self.0.execute("INSERT INTO entries(session,buffer,kind,value)
            SELECT ?1,?2,'literal',?3 WHERE NOT EXISTS (SELECT 1 FROM entries WHERE session=?1 AND buffer=?2)",params![session,buffer,text])?;
        if changed == 0 {
            bail!("restore requires a new buffer");
        }
        Ok(())
    }
}

impl Store {
    pub fn snapshots(&self, session: &str) -> Result<Vec<(i64, String)>> {
        let mut q = self
            .0
            .prepare("SELECT id,buffer FROM snapshots WHERE session=? ORDER BY id DESC")?;
        let rows = q.query_map([session], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }
    pub fn snapshot_sources(&self, session: &str, id: i64) -> Result<Vec<String>> {
        self.0
            .query_row(
                "SELECT id FROM snapshots WHERE session=? AND id=?",
                params![session, id],
                |r| r.get::<_, i64>(0),
            )
            .context("snapshot not found in this session")?;
        let mut q = self
            .0
            .prepare("SELECT path FROM snapshot_sources WHERE snapshot=? ORDER BY position")?;
        let rows = q.query_map([id], |r| r.get(0))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }
}

/// Resolve every input before handing a complete document to the clipboard.
pub fn publish_buffer(
    entries: &[Entry],
    headings: bool,
    separator: &str,
    write: impl FnOnce(&Rendered) -> Result<()>,
) -> Result<Rendered> {
    let document = render(entries, headings, separator)?;
    write(&document)?;
    Ok(document)
}
