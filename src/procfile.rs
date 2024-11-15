//! # procfile
//!
//! A rust library for parsing Procfile(s).
//!
//! ## Examples
//!
//! ```rust
//! let my_procfile = "web: cargo run";
//! let parsed = procfile::parse(my_procfile).expect("Failed parsing procfile");
//! let web_process = parsed.get("web").expect("Failed getting web process");
//!
//! assert_eq!("cargo", web_process.command);
//! assert_eq!(vec!["run"], web_process.options);
//! ```

use std::fmt::{Display, Formatter, Result as FmtResult};

use regex::Regex;

pub(crate) type Error = Box<dyn std::error::Error + Send + Sync>;
pub(crate) type Result<T, E = Error> = std::result::Result<T, E>;

/// Parses a Procfile string.
///
/// # Examples
///
/// ```rust
/// use procfile;
///
/// let my_procfile = "web: cargo run";
/// let parsed = procfile::parse(my_procfile).expect("Failed parsing procfile");
/// let web_process = parsed.get("web").expect("Failed getting web process");
///
/// assert_eq!("cargo", web_process.command);
/// assert_eq!(vec!["run"], web_process.options);
/// ```
///
/// # Errors
///
/// - When building the regex fails
/// - When either the command, options, and the process name don't exist but the regex matched

static REGEX: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"^([A-Za-z0-9_]+):\s*(.+)$").expect("Failed building regex")
});

pub(crate) fn parse<'a>(content: &'a str) -> Result<Vec<Process>> {
    let mut entries: Vec<Process> = Vec::new();

    content
        .split('\n')
        .for_each(|line| match REGEX.captures(line) {
            Some(captures) => {
                let details = captures
                    .get(2)
                    .expect("Failed getting command and options")
                    .as_str()
                    .trim()
                    .split(' ')
                    .collect::<Vec<_>>();
                let name = captures
                    .get(1)
                    .expect("Failed getting process name")
                    .as_str();

                entries.push(Process {
                    name,
                    command: details[0],
                    options: details[1..].to_vec(),
                });
            }
            None => (),
        });

    Ok(entries)
}

/// Represents a single process.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Process<'a> {
    /// The command entry name. (e.g. `build`)
    pub(crate) name: &'a str,
    /// The command to use. (e.g. `cargo`)
    pub(crate) command: &'a str,
    /// The command options. (e.g. `["build", "--release"]`)
    pub(crate) options: Vec<&'a str>,
}

impl<'a> Display for Process<'a> {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{} {}", self.command, self.options.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_process() {
        let procfile = "web: node a.js --option-1 --option-2";
        let parsed = parse(procfile).unwrap();

        let process = parsed.get(0).unwrap();

        assert_eq!("node", process.command);
        assert_eq!(vec!["a.js", "--option-1", "--option-2"], process.options)
    }

    #[test]
    fn multiple_process() {
        let procfile = "\
web: py b.py --my-option
worker: gcc c.c
        ";

        let parsed = parse(procfile).unwrap();

        let web = parsed.get(0).unwrap();
        let worker = parsed.get(1).unwrap();

        assert_eq!("py", web.command);
        assert_eq!("gcc", worker.command);
        assert_eq!(vec!["b.py", "--my-option"], web.options);
        assert_eq!(vec!["c.c"], worker.options);
    }

    #[test]
    fn no_process() {
        let procfile = "";
        let parsed = parse(procfile).unwrap();

        assert!(parsed.is_empty());
    }

    #[test]
    fn invalid_process() {
        let procfile = "hedhehiidhodhidhiodiedhidwhio";
        let parsed = parse(procfile).unwrap();

        assert!(parsed.is_empty());
    }

    #[test]
    fn test_display() {
        let procfile = "web: node index.mjs --verbose";
        let parsed = parse(procfile).unwrap();
        let web_process = &*parsed.get(0).unwrap();

        assert_eq!("node index.mjs --verbose", &format!("{}", web_process));
    }
}
