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

    let query_chars = lower_chars(&q);
    let mut by_window: Vec<(usize, &str)> = sorted
        .iter()
        .filter_map(|n| {
            best_window(&lower_chars(n), &query_chars).map(|p| (p[p.len() - 1] - p[0] + 1, *n))
        })
        .collect();
    by_window.sort();
    by_window.into_iter().map(|(_, n)| n).collect()
}

/// Índices, em caracteres de `name`, das letras a destacar para `query`: a
/// primeira ocorrência da substring; senão as letras da menor janela.
#[cfg_attr(not(test), expect(dead_code))]
pub fn highlight(query: &str, name: &str) -> Vec<usize> {
    let query = lower_chars(query);
    let name = lower_chars(name);
    if query.is_empty() {
        return Vec::new();
    }
    if let Some(start) = name
        .windows(query.len())
        .position(|w| w == query.as_slice())
    {
        return (start..start + query.len()).collect();
    }
    best_window(&name, &query).unwrap_or_default()
}

/// Minúsculas caractere a caractere, sem mudar a contagem.
fn lower_chars(text: &str) -> Vec<char> {
    text.chars()
        .map(|c| c.to_lowercase().next().unwrap_or(c))
        .collect()
}

/// Posições das letras de `query` na menor janela de `name` que a contém
/// como subsequência (a primeira, no empate); `None` se não contém.
fn best_window(name: &[char], query: &[char]) -> Option<Vec<usize>> {
    let first = *query.first()?;
    (0..name.len())
        .filter(|&start| name[start] == first)
        .filter_map(|start| {
            let mut positions = vec![start];
            for (i, &c) in name.iter().enumerate().skip(start + 1) {
                if positions.len() == query.len() {
                    break;
                }
                if c == query[positions.len()] {
                    positions.push(i);
                }
            }
            (positions.len() == query.len()).then_some(positions)
        })
        .min_by_key(|p| p[p.len() - 1] - p[0])
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

    #[test]
    fn highlight_substring_first_occurrence() {
        assert_eq!(highlight("open", "OPENAI_OPEN"), vec![0, 1, 2, 3]);
        assert_eq!(highlight("AI", "openai"), vec![4, 5]);
    }

    #[test]
    fn highlight_subsequence_uses_shortest_window() {
        // "OPENAI_API_KEY" tem um só O (0) e um só K (11)
        assert_eq!(highlight("ok", "OPENAI_API_KEY"), vec![0, 11]);
        assert_eq!(highlight("ok", "OAK"), vec![0, 2]);
        assert_eq!(highlight("ab", "a_x_a_b"), vec![4, 6]);
    }

    #[test]
    fn highlight_empty_or_no_match() {
        assert!(highlight("", "PATH").is_empty());
        assert!(highlight("zz", "PATH").is_empty());
    }
}
