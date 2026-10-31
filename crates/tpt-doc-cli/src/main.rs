//! `tpt` — command line front end for the tpt-doc workspace.
//!
//! Exit codes: `0` success / valid, `1` document invalid, `2` usage or I/O
//! error.

mod commands;
mod sniff;

use std::io::{Read, Write};
use std::process::ExitCode;

const USAGE: &str = "\
tpt — inspect, validate and render documents

USAGE:
    tpt sniff <FILE>                 print the detected format
    tpt inspect <FILE>               structural summary
    tpt validate <FILE> [--json]     validate against the format's rules
    tpt render <FILE.html> [--data D.json] [-o OUT]
                                     render HTML to PDF; with --data, merge {{fields}} first
    tpt text <FILE.docx>             extract plain text
    tpt convert <FILE.hl7> [-o OUT]  HL7 v2 ADT/ORU to a FHIR R5 bundle (JSON)
    tpt redact <FILE.hl7> [-o OUT]   de-identify HL7 v2 or FHIR JSON (Safe Harbor profile)
    tpt diff <A> <B> [--ignore PREFIX]... [--json]
                                     structural diff by path; exit 1 when they differ

<FILE> may be `-` for stdin. Formats are detected from content.
Exit codes: 0 ok/valid, 1 invalid, 2 usage or I/O error.
";

fn read_input(path: &str) -> Result<Vec<u8>, String> {
    if path == "-" {
        let mut buf = Vec::new();
        std::io::stdin()
            .read_to_end(&mut buf)
            .map_err(|e| format!("reading stdin: {e}"))?;
        Ok(buf)
    } else {
        std::fs::read(path).map_err(|e| format!("{path}: {e}"))
    }
}

fn run(args: &[String]) -> Result<ExitCode, String> {
    let mut json = false;
    let mut out_path: Option<String> = None;
    let mut data_path: Option<String> = None;
    let mut ignore: Vec<String> = Vec::new();
    let mut positional = Vec::new();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--json" => json = true,
            "--data" => {
                data_path = Some(iter.next().ok_or("`--data` needs a path")?.clone());
            }
            "--ignore" => ignore.push(iter.next().ok_or("`--ignore` needs a path prefix")?.clone()),
            "-o" | "--output" => {
                out_path = Some(iter.next().ok_or("`-o` needs a path")?.clone());
            }
            "-h" | "--help" => {
                print!("{USAGE}");
                return Ok(ExitCode::SUCCESS);
            }
            "-" => positional.push(arg.as_str()),
            flag if flag.starts_with('-') => return Err(format!("unknown option `{flag}`")),
            _ => positional.push(arg.as_str()),
        }
    }
    if positional.first() == Some(&"diff") {
        let [_, left, right] = positional.as_slice() else {
            print!("{USAGE}");
            return Err("diff needs two files".to_owned());
        };
        let outcome = commands::diff(&read_input(left)?, &read_input(right)?, &ignore, json)?;
        print!("{}", outcome.output);
        return Ok(if outcome.identical {
            ExitCode::SUCCESS
        } else {
            ExitCode::from(1)
        });
    }
    let [command, file] = positional.as_slice() else {
        print!("{USAGE}");
        return Err("expected a command and one file".to_owned());
    };
    let bytes = read_input(file)?;
    match *command {
        "sniff" => println!("{}", commands::sniff_cmd(&bytes)?),
        "inspect" => print!("{}", commands::inspect(&bytes)?),
        "text" => println!("{}", commands::text(&bytes)?),
        "validate" => {
            let outcome = commands::validate(&bytes, json)?;
            print!("{}", outcome.output);
            if !outcome.valid {
                return Ok(ExitCode::from(1));
            }
        }
        "convert" => {
            let (json, notes) = commands::convert(&bytes)?;
            for note in notes {
                eprintln!("warning: {note}");
            }
            match out_path {
                Some(path) => std::fs::write(&path, json).map_err(|e| format!("{path}: {e}"))?,
                None => std::io::stdout()
                    .write_all(&json)
                    .map_err(|e| format!("stdout: {e}"))?,
            }
        }
        "redact" => {
            let (redacted, audit) = commands::redact(&bytes)?;
            eprintln!("redacted {} field(s): {}", audit.len(), audit.join(" "));
            match out_path {
                Some(path) => {
                    std::fs::write(&path, redacted).map_err(|e| format!("{path}: {e}"))?
                }
                None => std::io::stdout()
                    .write_all(&redacted)
                    .map_err(|e| format!("stdout: {e}"))?,
            }
        }
        "render" => {
            let data = data_path.as_deref().map(read_input).transpose()?;
            let pdf = commands::render(&bytes, data.as_deref())?;
            match out_path {
                Some(path) => std::fs::write(&path, pdf).map_err(|e| format!("{path}: {e}"))?,
                None => std::io::stdout()
                    .write_all(&pdf)
                    .map_err(|e| format!("stdout: {e}"))?,
            }
        }
        other => return Err(format!("unknown command `{other}`")),
    }
    Ok(ExitCode::SUCCESS)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(code) => code,
        Err(message) => {
            eprintln!("tpt: {message}");
            ExitCode::from(2)
        }
    }
}
