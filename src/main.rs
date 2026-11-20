mod desktop;

use aiclip::{render, Entry, Store};
use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use std::{
    fs::OpenOptions,
    io::{self, Read, Write},
    path::PathBuf,
};

#[derive(Parser)]
#[command(
    version,
    about = "Assemble ordered files and instructions for a single clipboard write",
    after_help = "Quote wildcard patterns. ** traverses subdirectories. Each pattern must match a file. Repeated inputs are kept. Files must be UTF-8 (optional BOM). Use - once to read stdin, or 'text' for literal instructions. Empty files and empty literal strings are allowed. Save and get --file require a new destination."
)]
struct Cli {
    #[arg(long, global = true, env = "AICLIP_DB")]
    db: Option<PathBuf>,
    #[arg(long, global = true, default_value = "default")]
    session: String,
    #[arg(long, global = true)]
    no_headings: bool,
    #[arg(long, global = true, default_value = "\n\n")]
    separator: String,
    #[arg(long, global = true)]
    stats: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Open the desktop workspace.
    #[command(long_flag = "gui")]
    Gui,
    /// Stage file and wildcard references. Use - to stage text from stdin.
    #[command(long_flag = "add")]
    Add {
        buffer: String,
        #[arg(required = true)]
        sources: Vec<String>,
    },
    /// Stage literal instructions.
    Text { buffer: String, text: String },
    /// Preview exactly what put would copy.
    Show { buffer: String },
    /// Publish a buffer in one clipboard write.
    #[command(visible_aliases=["cp","copy","pub","publish"])]
    Put { buffer: String },
    /// Capture current clipboard text into a buffer or a new file.
    Get {
        #[arg(required_unless_present = "file", conflicts_with = "file")]
        buffer: Option<String>,
        #[arg(long)]
        file: Option<PathBuf>,
    },
    /// Save the rendered buffer to a new file.
    Save { buffer: String, file: PathBuf },
    /// Append rendered files to existing clipboard text.
    #[command(long_flag = "append")]
    Append {
        #[arg(required = true)]
        sources: Vec<String>,
    },
    /// Remove all entries from a named buffer.
    #[command(long_flag = "clear")]
    Clear { buffer: String },
    /// List buffers in this session.
    List,
    /// Save or preview a globally reusable live recipe.
    Template {
        #[command(subcommand)]
        action: Template,
    },
    /// Freeze a rendered buffer, preview it, or copy its frozen text into a buffer.
    Snapshot {
        #[command(subcommand)]
        action: Snapshot,
    },
}
#[derive(Subcommand)]
enum Template {
    Save { name: String, buffer: String },
    Run { name: String },
}
#[derive(Subcommand)]
enum Snapshot {
    /// List saved IDs and their originating buffers in this session.
    List,
    /// List the file paths included in a frozen snapshot.
    Sources {
        id: i64,
    },
    Save {
        buffer: String,
    },
    Show {
        id: i64,
    },
    Restore {
        id: i64,
        buffer: String,
    },
}

fn collect_sources(args: Vec<String>) -> Result<Vec<Entry>> {
    let mut entries = Vec::new();
    let mut used_stdin = false;
    for arg in args {
        if arg == "-" {
            if used_stdin {
                bail!("stdin can appear only once");
            }
            used_stdin = true;
            let mut content = String::new();
            io::stdin().read_to_string(&mut content)?;
            entries.push(Entry::new("literal", content));
        } else {
            entries.push(Entry::source(&arg)?);
        }
    }
    Ok(entries)
}
fn save(path: PathBuf, text: &str) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .with_context(|| format!("creating {}", path.display()))?;
    file.write_all(text.as_bytes())?;
    Ok(())
}
fn output(text: &str) -> Result<()> {
    io::stdout().lock().write_all(text.as_bytes())?;
    Ok(())
}
fn run() -> Result<()> {
    let cli = Cli::parse();
    let path = match cli.db {
        Some(p) => p,
        None => aiclip::database_path()?,
    };
    let mut store = Store::open(&path)?;
    let session = &cli.session;
    let report = |result: &aiclip::Rendered| {
        if cli.stats {
            eprintln!(
                "{} files, {} UTF-8 bytes, ~{} tokens (rough estimate)",
                result.files.len(),
                result.text.len(),
                result.text.chars().count().div_ceil(4)
            );
        }
        if result.text.len() > 1024 * 1024 {
            eprintln!("large output: {} bytes", result.text.len());
        }
    };
    let format = |entries: &[Entry]| -> Result<aiclip::Rendered> {
        let result = render(entries, !cli.no_headings, &cli.separator)?;
        report(&result);
        Ok(result)
    };
    match cli.command {
        Command::Gui => {
            return desktop::run(store, cli.session)
                .map_err(|e| anyhow::anyhow!("opening desktop workspace: {e}"))
        }
        Command::Add { buffer, sources } => {
            let entries = collect_sources(sources)?;
            format(&entries)?;
            store.add(session, &buffer, &entries)?;
        }
        Command::Text { buffer, text } => {
            store.add(session, &buffer, &[Entry::new("literal", text)])?
        }
        Command::Show { buffer } => output(&format(&store.read(session, &buffer)?)?.text)?,
        Command::Put { buffer } => {
            aiclip::publish_buffer(
                &store.read(session, &buffer)?,
                !cli.no_headings,
                &cli.separator,
                |doc| {
                    report(doc);
                    aiclip::publish(&doc.text)
                },
            )?;
        }
        Command::Get { buffer, file } => {
            let text = arboard::Clipboard::new()?.get_text()?;
            if let Some(file) = file {
                save(file, &text)?;
            } else {
                store.add(
                    session,
                    &buffer.context("missing buffer")?,
                    &[Entry::new("capture", text)],
                )?;
            }
        }
        Command::Save { buffer, file } => {
            save(file, &format(&store.read(session, &buffer)?)?.text)?
        }
        Command::Append { sources } => {
            let rendered = format(&collect_sources(sources)?)?;
            let mut clipboard = arboard::Clipboard::new()?;
            let mut text = match clipboard.get_text() {
                Ok(s) => s,
                Err(arboard::Error::ContentNotAvailable) => String::new(),
                Err(e) => return Err(e.into()),
            };
            if !text.is_empty() {
                text.push_str(&cli.separator);
            }
            text.push_str(&rendered.text);
            anyhow::ensure!(
                !text.contains('\0'),
                "clipboard text contains a NUL character"
            );
            clipboard.set_text(text)?;
        }
        Command::Clear { buffer } => store.clear(session, &buffer)?,
        Command::List => {
            for name in store.buffers(session)? {
                println!("{name}");
            }
        }
        Command::Template { action } => match action {
            Template::Save { name, buffer } => {
                store.template_save(&name, &store.read(session, &buffer)?)?
            }
            Template::Run { name } => output(&format(&store.template_read(&name)?)?.text)?,
        },
        Command::Snapshot { action } => match action {
            Snapshot::Save { buffer } => {
                let rendered = format(&store.read(session, &buffer)?)?;
                println!("{}", store.snapshot(session, &buffer, &rendered)?);
            }
            Snapshot::List => {
                for (id, buffer) in store.snapshots(session)? {
                    println!("{id}\t{buffer}");
                }
            }
            Snapshot::Sources { id } => {
                for path in store.snapshot_sources(session, id)? {
                    println!("{path}");
                }
            }
            Snapshot::Show { id } => output(&store.snapshot_text(session, id)?)?,
            Snapshot::Restore { id, buffer } => {
                let text = store.snapshot_text(session, id)?;
                store.restore_new(session, &buffer, &text)?;
            }
        },
    }
    Ok(())
}
fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("aiclip: {e:#}");
            std::process::ExitCode::FAILURE
        }
    }
}
