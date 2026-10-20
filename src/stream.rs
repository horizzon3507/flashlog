use crate::detector::Kind;

/// Select matches and their surrounding lines, merging overlapping context windows.
pub fn selected_lines<R: AsRef<[Option<Kind>]>>(kinds: R, only: bool, context: usize) -> Vec<bool> {
    let kinds = kinds.as_ref();
    if !only && context == 0 {
        return vec![true; kinds.len()];
    }
    let mut selected = vec![!only; kinds.len()];
    for (index, kind) in kinds.iter().enumerate() {
        if kind.is_some() {
            let start = index.saturating_sub(context);
            let end = index.saturating_add(context + 1).min(kinds.len());
            selected[start..end].fill(true);
        }
    }
    selected
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merges_overlapping_context_windows() {
        let kinds = [None, Some(Kind::Error), None, Some(Kind::Fatal), None, None];
        assert_eq!(
            selected_lines(kinds, true, 1),
            vec![true; 5].into_iter().chain([false]).collect::<Vec<_>>()
        );
    }

    #[test]
    fn only_without_context_selects_matches() {
        let kinds = [None, Some(Kind::Error), None];
        assert_eq!(selected_lines(kinds, true, 0), vec![false, true, false]);
    }
}
