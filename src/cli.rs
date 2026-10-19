//! Argumentos da linha de comando.

use std::path::PathBuf;

use crate::i18n::lang;

#[derive(Debug, PartialEq, Eq)]
pub struct Args {
    pub query: Option<String>,
    pub file: Option<PathBuf>,
}

/// Lê os argumentos (sem o argv[0]); erro devolve a linha de uso.
pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Args, String> {
    let mut query = None;
    let mut file = None;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-f" | "--file" => file = Some(args.next().ok_or_else(usage)?.into()),
            flag if flag.starts_with('-') => return Err(usage()),
            _ if query.is_some() => return Err(usage()),
            _ => query = Some(arg),
        }
    }
    Ok(Args {
        query: query.filter(|q| !q.is_empty()),
        file,
    })
}

fn usage() -> String {
    lang().usage().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(args: &[&str]) -> Result<Args, String> {
        parse(args.iter().map(|a| a.to_string()))
    }

    #[test]
    fn query_only() {
        assert_eq!(
            p(&["fake"]),
            Ok(Args {
                query: Some("fake".into()),
                file: None
            })
        );
    }

    #[test]
    fn file_flag_in_any_position() {
        let expected = Ok(Args {
            query: Some("x".into()),
            file: Some("a.env".into()),
        });
        assert_eq!(p(&["-f", "a.env", "x"]), expected);
        assert_eq!(p(&["x", "--file", "a.env"]), expected);
    }

    #[test]
    fn empty_query_is_none() {
        let expected = Ok(Args {
            query: None,
            file: None,
        });
        assert_eq!(p(&[]), expected);
        assert_eq!(p(&[""]), expected);
    }

    #[test]
    fn bad_args_are_errors() {
        assert!(p(&["a", "b"]).is_err());
        assert!(p(&["-f"]).is_err());
        assert!(p(&["--bogus"]).is_err());
    }
}
