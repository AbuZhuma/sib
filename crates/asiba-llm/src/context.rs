use crate::tokens::estimate_tokens;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    pub priority: u8,
    pub title: String,
    pub text: String,
}

impl Part {
    pub fn new(priority: u8, title: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            priority,
            title: title.into(),
            text: text.into(),
        }
    }
}

pub struct ContextBuilder {
    budget_tokens: usize,
    parts: Vec<Part>,
}

const TRUNCATED_MARK: &str = "... (truncated to fit context)";

impl ContextBuilder {
    pub fn new(budget_tokens: usize) -> Self {
        Self {
            budget_tokens,
            parts: Vec::new(),
        }
    }

    pub fn part(mut self, part: Part) -> Self {
        self.parts.push(part);
        self
    }

    pub fn build(self) -> (String, usize) {
        let mut order: Vec<usize> = (0..self.parts.len()).collect();
        order.sort_by_key(|&i| self.parts[i].priority);
        let mut remaining = self.budget_tokens;
        let mut included: Vec<(usize, String)> = Vec::new();
        for index in order {
            let part = &self.parts[index];
            let cost = estimate_tokens(&part.text);
            if cost <= remaining {
                remaining -= cost;
                included.push((index, part.text.clone()));
                continue;
            }
            if let Some(cut) = truncate_to(&part.text, remaining) {
                remaining = 0;
                included.push((index, cut));
            }
        }
        included.sort_by_key(|(index, _)| *index);
        let text: Vec<String> = included.into_iter().map(|(_, text)| text).collect();
        let joined = text.join("\n");
        let used = estimate_tokens(&joined);
        (joined, used)
    }
}

fn truncate_to(text: &str, budget_tokens: usize) -> Option<String> {
    const MIN_LINES: usize = 3;
    let mut kept = Vec::new();
    let mut used = estimate_tokens(TRUNCATED_MARK) + 1;
    for line in text.lines() {
        let cost = estimate_tokens(line) + 1;
        if used + cost > budget_tokens {
            break;
        }
        used += cost;
        kept.push(line);
    }
    if kept.len() < MIN_LINES {
        return None;
    }
    kept.push(TRUNCATED_MARK);
    Some(kept.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_keeps_original_order_and_drops_low_priority_when_over_budget() {
        let big = "x".repeat(1400);
        let (text, _) = ContextBuilder::new(250)
            .part(Part::new(2, "low", big.clone()))
            .part(Part::new(0, "high", "first\nsecond\nthird\nfourth"))
            .part(Part::new(1, "mid", "m1\nm2\nm3\nm4"))
            .build();
        assert!(text.starts_with("first"));
        assert!(text.contains("m1"));
        assert!(!text.contains(&big));
    }

    #[test]
    fn build_truncates_last_part_by_lines() {
        let lines: Vec<String> = (0..100)
            .map(|i| format!("line {i} {}", "y".repeat(30)))
            .collect();
        let (text, used) = ContextBuilder::new(200)
            .part(Part::new(0, "only", lines.join("\n")))
            .build();
        assert!(text.contains("line 0"));
        assert!(!text.contains("line 99"));
        assert!(text.ends_with(TRUNCATED_MARK));
        assert!(used <= 200);
    }
}
