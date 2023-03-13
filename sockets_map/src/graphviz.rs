//! This module leverages the Graphviz utility to generate graphs.

use anyhow::bail;
use std::{io::Write, process::Command};
use tempfile;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutEngine {
    Dot,
    Neato,
    Fdp,
    Circo,
}

impl std::str::FromStr for LayoutEngine {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "dot" => Ok(LayoutEngine::Dot),
            "neato" => Ok(LayoutEngine::Neato),
            "fdp" => Ok(LayoutEngine::Fdp),
            "circo" => Ok(LayoutEngine::Circo),
            _ => Err("unknown layout engine"),
        }
    }
}

impl From<&LayoutEngine> for &'static str {
    fn from(value: &LayoutEngine) -> Self {
        match value {
            LayoutEngine::Dot => "dot",
            LayoutEngine::Neato => "neato",
            LayoutEngine::Fdp => "fdp",
            LayoutEngine::Circo => "circo",
        }
    }
}

impl std::fmt::Display for LayoutEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.into())
    }
}

pub fn run_graphviz(
    dot_code: String,
    output_file_path: &std::path::Path,
    extension: String,
    dump_dot_code: Option<&std::path::PathBuf>,
    vertical: bool,
    layout_engine: Option<&LayoutEngine>,
) -> anyhow::Result<()> {
    // Write dot code to file
    let mut dot_file = match tempfile::NamedTempFile::new() {
        Ok(d) => d,
        Err(e) => bail!(format!("unable to create dot temporary file: {e}")),
    };
    if dot_file.write_all(dot_code.as_bytes()).is_err() {
        bail!("unable to write dot code to temporary file");
    }

    // Dump if necessary
    match dump_dot_code {
        None => (),
        Some(s) => {
            log::debug!("Dumping dot code");
            let mut dump_file = match std::fs::File::create(s) {
                Ok(d) => d,
                Err(e) => bail!(format!("unable to open dump file {s:?} for writing: {e}")),
            };
            if dump_file.write_all(dot_code.as_bytes()).is_err() {
                bail!(format!(
                    "unable to dump dot code to file {}",
                    s.to_string_lossy()
                ));
            }
        }
    }

    // Vertical options
    let vertical_options = &["-Grankdir=LR".to_string(), "-Grankdir=TB".to_string()];

    // Args
    let mut args: Vec<String> = vec![
        format!("-T{extension}"),
        "-o".into(),
        output_file_path.to_string_lossy().into(),
        dot_file.path().to_string_lossy().into(),
        match vertical {
            true => vertical_options[0].clone(),
            false => vertical_options[1].clone(),
        },
    ];
    if let Some(layout_engine) = layout_engine {
        args.push(format!("-K{layout_engine}"));
    }

    log::debug!("Generating graph with Graphviz");
    let output = Command::new("dot").args(args).output();
    match output {
        Ok(o) => {
            if let Some(code) = o.status.code() {
                if code != 0 {
                    bail!(String::from_utf8_lossy(&o.stderr).to_string());
                }
            }
            Ok(())
        }
        Err(e) => bail!(e),
    }
}
