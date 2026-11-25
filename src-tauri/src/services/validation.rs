//! Input validation. The frontend is untrusted: everything that reaches the database
//! is checked here first, with messages written for the learner. The SQL CHECK
//! constraints are the backstop and mirror these limits.

use std::collections::HashSet;

use crate::error::{AppError, AppResult};
use crate::models::challenge::*;

pub const MAX_STEPS: usize = 50;
pub const MAX_NOTE_CHARS: usize = 100_000;

fn len(s: &str) -> usize {
    s.chars().count()
}

fn require(field: &str, value: &str, max: usize) -> AppResult<()> {
    let n = len(value.trim());
    if n == 0 {
        return Err(AppError::validation(format!("{field} can't be empty.")));
    }
    if len(value) > max {
        return Err(AppError::validation(format!(
            "{field} must be {max} characters or fewer."
        )));
    }
    Ok(())
}

/// A key or slug: lowercase ASCII letters, digits and hyphens.
pub fn is_valid_key(key: &str) -> bool {
    (1..=64).contains(&key.len())
        && key
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn require_key(field: &str, key: &str) -> AppResult<()> {
    if is_valid_key(key) {
        Ok(())
    } else {
        Err(AppError::validation(format!(
            "{field} must be 1-64 characters: lowercase letters, digits and hyphens."
        )))
    }
}

/// Trim every text field so stored content has no stray edge whitespace.
pub fn normalize(mut draft: ChallengeDraft) -> ChallengeDraft {
    draft.title = draft.title.trim().to_owned();
    draft.summary = draft.summary.trim().to_owned();
    draft.objectives_md = draft.objectives_md.trim().to_owned();
    for step in &mut draft.steps {
        step.title = step.title.trim().to_owned();
        step.instructions_md = step.instructions_md.trim().to_owned();
        step.completion_criteria = step.completion_criteria.trim().to_owned();
        for hint in &mut step.hints {
            *hint = hint.trim().to_owned();
        }
        for choice in &mut step.choices {
            choice.label = choice.label.trim().to_owned();
            choice.explanation = choice.explanation.trim().to_owned();
        }
    }
    draft
}

pub fn validate_draft(draft: &ChallengeDraft) -> AppResult<()> {
    require("Title", &draft.title, 120)?;
    require("Summary", &draft.summary, 500)?;
    if len(&draft.objectives_md) > 5000 {
        return Err(AppError::validation(
            "Objectives must be 5000 characters or fewer.",
        ));
    }
    if draft.steps.is_empty() {
        return Err(AppError::validation("Add at least one step."));
    }
    if draft.steps.len() > MAX_STEPS {
        return Err(AppError::validation(format!(
            "A challenge can have at most {MAX_STEPS} steps."
        )));
    }

    let mut keys = HashSet::new();
    for (i, step) in draft.steps.iter().enumerate() {
        let n = i + 1;
        require_key(&format!("Step {n} key"), &step.step_key)?;
        if !keys.insert(step.step_key.as_str()) {
            return Err(AppError::validation(format!(
                "Step key '{}' is used twice.",
                step.step_key
            )));
        }
        require(&format!("Step {n} title"), &step.title, 120)?;
        require(
            &format!("Step {n} instructions"),
            &step.instructions_md,
            10_000,
        )?;
        require(
            &format!("Step {n} completion criteria"),
            &step.completion_criteria,
            500,
        )?;
        if step.hints.len() > 5 {
            return Err(AppError::validation(format!(
                "Step {n} can have at most 5 hints."
            )));
        }
        for hint in &step.hints {
            require(&format!("Step {n} hint"), hint, 2000)?;
        }
        validate_choices(n, step)?;
    }

    let mut evidence_keys = HashSet::new();
    for evidence in &draft.evidence {
        require_key("Evidence key", &evidence.evidence_key)?;
        if !evidence_keys.insert(evidence.evidence_key.as_str()) {
            return Err(AppError::validation(format!(
                "Evidence key '{}' is used twice.",
                evidence.evidence_key
            )));
        }
        require("Evidence title", &evidence.title, 120)?;
        if evidence.packets.is_empty() || evidence.packets.len() > 200 {
            return Err(AppError::validation(
                "A packet summary needs 1-200 packets.",
            ));
        }
        for p in &evidence.packets {
            for (field, value) in [
                ("Packet source", &p.source),
                ("Packet destination", &p.destination),
                ("Packet protocol", &p.protocol),
                ("Packet info", &p.info),
            ] {
                require(field, value, 300)?;
            }
        }
    }
    Ok(())
}

fn validate_choices(n: usize, step: &StepDraft) -> AppResult<()> {
    match step.kind {
        StepKind::Check => {
            if !(2..=6).contains(&step.choices.len()) {
                return Err(AppError::validation(format!(
                    "Step {n} is a knowledge check and needs 2-6 choices."
                )));
            }
            if step.choices.iter().filter(|c| c.is_correct).count() != 1 {
                return Err(AppError::validation(format!(
                    "Step {n} must have exactly one correct choice."
                )));
            }
            for choice in &step.choices {
                require(&format!("Step {n} choice"), &choice.label, 300)?;
                require(
                    &format!("Step {n} choice explanation"),
                    &choice.explanation,
                    1000,
                )?;
            }
        }
        StepKind::Task | StepKind::Note => {
            if !step.choices.is_empty() {
                return Err(AppError::validation(format!(
                    "Step {n} can't have choices."
                )));
            }
        }
    }
    Ok(())
}

pub fn validate_note(body_md: &str) -> AppResult<()> {
    if len(body_md) > MAX_NOTE_CHARS {
        return Err(AppError::validation(format!(
            "Notes can be up to {MAX_NOTE_CHARS} characters. This one is {}.",
            len(body_md)
        )));
    }
    Ok(())
}

/// Convert frontend input for a custom challenge into a draft. Every step is a plain
/// task; keys are generated so the frontend never chooses identifiers.
pub fn custom_to_draft(input: CustomChallengeInput) -> ChallengeDraft {
    let steps = input
        .steps
        .into_iter()
        .enumerate()
        .map(|(i, s)| StepDraft {
            step_key: format!("step-{}", i + 1),
            kind: StepKind::Task,
            title: s.title,
            instructions_md: s.instructions_md,
            completion_criteria: s.completion_criteria,
            hints: s
                .hint
                .into_iter()
                .filter(|h| !h.trim().is_empty())
                .collect(),
            choices: Vec::new(),
        })
        .collect();
    normalize(ChallengeDraft {
        title: input.title,
        summary: input.summary,
        difficulty: input.difficulty,
        objectives_md: input.objectives_md,
        steps,
        evidence: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(key: &str, kind: StepKind) -> StepDraft {
        StepDraft {
            step_key: key.into(),
            kind,
            title: "T".into(),
            instructions_md: "Do it".into(),
            completion_criteria: "Done".into(),
            hints: vec![],
            choices: vec![],
        }
    }

    fn draft(steps: Vec<StepDraft>) -> ChallengeDraft {
        ChallengeDraft {
            title: "Title".into(),
            summary: "Summary".into(),
            difficulty: Difficulty::Beginner,
            objectives_md: String::new(),
            steps,
            evidence: vec![],
        }
    }

    fn choice(correct: bool) -> ChoiceDraft {
        ChoiceDraft {
            label: "A".into(),
            is_correct: correct,
            explanation: "Because".into(),
        }
    }

    #[test]
    fn rejects_blank_title_and_no_steps() {
        let mut d = draft(vec![step("a", StepKind::Task)]);
        d.title = "   ".into();
        assert!(validate_draft(&d).is_err());
        assert!(validate_draft(&draft(vec![])).is_err());
    }

    #[test]
    fn rejects_duplicate_or_malformed_step_keys() {
        let dup = draft(vec![step("a", StepKind::Task), step("a", StepKind::Task)]);
        assert!(validate_draft(&dup).is_err());
        let bad = draft(vec![step("Not Valid!", StepKind::Task)]);
        assert!(validate_draft(&bad).is_err());
    }

    #[test]
    fn knowledge_check_needs_exactly_one_correct_choice() {
        let mut s = step("q", StepKind::Check);
        s.choices = vec![choice(false), choice(false)];
        assert!(
            validate_draft(&draft(vec![s.clone()])).is_err(),
            "zero correct"
        );
        s.choices = vec![choice(true), choice(true)];
        assert!(
            validate_draft(&draft(vec![s.clone()])).is_err(),
            "two correct"
        );
        s.choices = vec![choice(true), choice(false)];
        assert!(validate_draft(&draft(vec![s])).is_ok());
    }

    #[test]
    fn non_check_steps_cannot_carry_choices() {
        let mut s = step("a", StepKind::Task);
        s.choices = vec![choice(true), choice(false)];
        assert!(validate_draft(&draft(vec![s])).is_err());
    }

    #[test]
    fn note_length_is_bounded() {
        assert!(validate_note(&"x".repeat(MAX_NOTE_CHARS)).is_ok());
        assert!(validate_note(&"x".repeat(MAX_NOTE_CHARS + 1)).is_err());
    }

    #[test]
    fn custom_input_gets_generated_keys_and_trimmed_text() {
        let d = custom_to_draft(CustomChallengeInput {
            title: "  Mine  ".into(),
            summary: "S".into(),
            difficulty: Difficulty::Advanced,
            objectives_md: String::new(),
            steps: vec![
                CustomStepInput {
                    title: " One ".into(),
                    instructions_md: "x".into(),
                    completion_criteria: "y".into(),
                    hint: Some("   ".into()),
                },
                CustomStepInput {
                    title: "Two".into(),
                    instructions_md: "x".into(),
                    completion_criteria: "y".into(),
                    hint: Some("try this".into()),
                },
            ],
        });
        assert_eq!(d.title, "Mine");
        assert_eq!(d.steps[0].step_key, "step-1");
        assert_eq!(d.steps[1].step_key, "step-2");
        assert!(d.steps[0].hints.is_empty(), "blank hint is dropped");
        assert_eq!(d.steps[1].hints, vec!["try this"]);
        assert!(validate_draft(&d).is_ok());
    }
}
