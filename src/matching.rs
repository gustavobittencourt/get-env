//! Casamento do nome digitado com os nomes das variáveis.

/// Nomes que casam com `query`, ignorando caixa, na ordem da tabela:
/// iguais à query; senão os que a contêm; senão os que têm as letras dela
/// em ordem, pela menor janela.
pub fn find<'a>(query: &str, names: &[&'a str]) -> Vec<&'a str> {
    let q = query.to_lowercase();
    let mut sorted = names.to_vec();
    sorted.sort_unstable();

    let exact: Vec<&str> = sorted
        .iter()
        .copied()
        .filter(|n| n.to_lowercase() == q)
        .collect();
    if !exact.is_empty() {
        return exact;
    }

    let substring: Vec<&str> = sorted
        .iter()
        .copied()
        .filter(|n| n.to_lowercase().contains(&q))
        .collect();
    if !substring.is_empty() {
        return substring;
    }

    let query_chars: Vec<char> = q.chars().collect();
    let mut by_window: Vec<(usize, &str)> = sorted
        .iter()
        .filter_map(|n| shortest_window(&n.to_lowercase(), &query_chars).map(|w| (w, *n)))
        .collect();
    by_window.sort();
    by_window.into_iter().map(|(_, n)| n).collect()
}

/// Menor janela, em caracteres, de `name` que contém `query` como
/// subsequência; `None` se não contém.
fn shortest_window(name: &str, query: &[char]) -> Option<usize> {
    let chars: Vec<char> = name.chars().collect();
    let first = *query.first()?;
    (0..chars.len())
        .filter(|&start| chars[start] == first)
        .filter_map(|start| {
            let mut next = 1;
            if next == query.len() {
                return Some(1);
            }
            for (i, &c) in chars.iter().enumerate().skip(start + 1) {
                if c == query[next] {
                    next += 1;
                    if next == query.len() {
                        return Some(i - start + 1);
                    }
                }
            }
            None
        })
        .min()
}

#[cfg(test)]
mod tests {
    use super::*;

    const NAMES: [&str; 7] = [
        "PATH",
        "MANPATH",
        "INFOPATH",
        "OPENAI_API_KEY",
        "OPENAI_ORG",
        "HOME",
        "OAK",
    ];

    #[test]
    fn exact_match_wins() {
        assert_eq!(find("path", &NAMES), vec!["PATH"]);
    }

    #[test]
    fn substring_sorted_alpha() {
        assert_eq!(find("openai", &NAMES), vec!["OPENAI_API_KEY", "OPENAI_ORG"]);
    }

    #[test]
    fn substring_before_subsequence() {
        assert_eq!(find("oa", &NAMES), vec!["OAK"]);
    }

    #[test]
    fn subsequence_shortest_window_first() {
        // "ok": OPENAI_ORG não tem k; OAK janela 3; OPENAI_API_KEY janela longa
        assert_eq!(find("ok", &NAMES), vec!["OAK", "OPENAI_API_KEY"]);
    }

    #[test]
    fn case_insensitive() {
        assert_eq!(find("HoMe", &NAMES), vec!["HOME"]);
    }

    #[test]
    fn no_match_is_empty() {
        assert!(find("zzz", &NAMES).is_empty());
    }

    #[test]
    fn two_exact_names_differing_in_case() {
        assert_eq!(find("path", &["PATH", "path"]), vec!["PATH", "path"]);
    }
}
