//! What a script breaks before it is drawn (`check`, for the model to fix), and the script cut to what can be drawn
//! (`clamped`, with a problem for each cut, so a PNG is written whatever the model wrote).

use super::{Finding, FindingKind, Limits, LoosePose, LooseProp, Pose, PropKind, Script};

/// The ordinal word for small counts, a number past ten.
fn nth(n: usize) -> String {
    const WORDS: [&str; 10] = [
        "first", "second", "third", "fourth", "fifth", "sixth", "seventh", "eighth", "ninth",
        "tenth",
    ];
    WORDS
        .get(n.wrapping_sub(1))
        .map(|w| (*w).to_string())
        .unwrap_or_else(|| format!("{n}th"))
}

fn chars(text: &str) -> usize {
    text.chars().count()
}

fn catalogue_of_poses() -> String {
    Pose::ALL
        .iter()
        .map(|p| p.word())
        .collect::<Vec<_>>()
        .join(", ")
}

fn catalogue_of_props() -> String {
    PropKind::ALL
        .iter()
        .map(|p| p.word())
        .collect::<Vec<_>>()
        .join(", ")
}

/// A limit finding for text that ran over: "<what> runs to N characters and <a what> holds L: say it in about A".
fn too_long(
    element: String,
    what: &str,
    holder: &str,
    text: &str,
    limit: usize,
) -> Option<Finding> {
    let n = chars(text);
    (n > limit).then(|| {
        let about = Limits::about(limit);
        Finding::new(
            FindingKind::Limit,
            element,
            vec![n as i32, limit as i32, about as i32],
            format!(
                "{what} runs to {n} characters and {holder} holds {limit}: say it in about {about}"
            ),
        )
    })
}

impl Script {
    /// Every way the script breaks the standard, one sentence each with its fix; empty when it keeps it.
    pub fn check(&self) -> Vec<Finding> {
        let mut found = Vec::new();
        let count = self.panels.len();
        if count > Limits::PANELS {
            found.push(Finding::new(
                FindingKind::Limit,
                "panels",
                vec![count as i32, Limits::PANELS as i32],
                format!(
                    "the strip has {count} panels and holds {}: tell it in {} or fewer",
                    Limits::PANELS,
                    Limits::PANELS
                ),
            ));
        }
        if let Some(title) = &self.title {
            found.extend(too_long(
                "title".into(),
                "the title",
                "a title",
                title,
                Limits::TITLE,
            ));
        }
        if let Some(alt) = &self.alt {
            found.extend(too_long(
                "alt".into(),
                "the alt line",
                "an alt line",
                alt,
                Limits::ALT,
            ));
        }
        for (i, panel) in self.panels.iter().enumerate() {
            let p = i + 1;
            let at = format!("panel:{p}");
            if panel.figures.is_empty() && panel.props.is_empty() {
                found.push(Finding::new(
                    FindingKind::Limit,
                    at.clone(),
                    vec![],
                    format!("panel {p} has nothing to draw: give it a figure or a prop"),
                ));
            }
            if let Some(caption) = &panel.caption {
                found.extend(too_long(
                    format!("{at}/caption"),
                    &format!("panel {p}'s caption"),
                    "a caption",
                    &caption.text,
                    Limits::CAPTION,
                ));
            }
            if panel.figures.len() > Limits::FIGURES {
                found.push(Finding::new(
                    FindingKind::Limit,
                    at.clone(),
                    vec![panel.figures.len() as i32, Limits::FIGURES as i32],
                    format!(
                        "panel {p} has {} figures and a panel holds {}: move one to another panel",
                        panel.figures.len(),
                        Limits::FIGURES
                    ),
                ));
            }
            let mut seen: Vec<&str> = Vec::new();
            for figure in &panel.figures {
                let element = format!("{at}/figure:{}", figure.id);
                if seen.contains(&figure.id.as_str()) {
                    found.push(Finding::new(
                        FindingKind::Unknown,
                        element.clone(),
                        vec![],
                        format!(
                            "panel {p} has two figures called {}: give each its own id",
                            figure.id
                        ),
                    ));
                }
                seen.push(&figure.id);
                if let LoosePose::Unknown(word) = &figure.pose {
                    found.push(Finding::new(
                        FindingKind::Unknown,
                        element.clone(),
                        vec![],
                        format!(
                            "panel {p}'s figure {} asks for pose {word}, which the catalogue lacks: use one of {}",
                            figure.id,
                            catalogue_of_poses()
                        ),
                    ));
                }
                if let Some(label) = &figure.label {
                    found.extend(too_long(
                        element.clone(),
                        &format!("panel {p}'s label under {}", figure.id),
                        "a label",
                        label,
                        Limits::LABEL,
                    ));
                }
                if let Some(sign) = &figure.sign {
                    found.extend(too_long(
                        element.clone(),
                        &format!("panel {p}'s sign held by {}", figure.id),
                        "a sign",
                        sign,
                        Limits::SIGN,
                    ));
                    if figure.pose.pose() != Pose::HoldingSign {
                        found.push(Finding::new(
                            FindingKind::Unknown,
                            element.clone(),
                            vec![],
                            format!(
                                "panel {p}'s figure {} has a sign, and only the holding-sign pose draws one: give it that pose or leave the sign out",
                                figure.id
                            ),
                        ));
                    }
                }
            }
            if panel.bubbles.len() > Limits::BUBBLES {
                found.push(Finding::new(
                    FindingKind::Limit,
                    at.clone(),
                    vec![panel.bubbles.len() as i32, Limits::BUBBLES as i32],
                    format!(
                        "panel {p} has {} bubbles and a panel holds {}: move a line to the next panel",
                        panel.bubbles.len(),
                        Limits::BUBBLES
                    ),
                ));
            }
            for (j, bubble) in panel.bubbles.iter().enumerate() {
                let element = format!("{at}/bubble:{}", j + 1);
                if bubble.by != "off" && !panel.figures.iter().any(|f| f.id == bubble.by) {
                    found.push(Finding::new(
                        FindingKind::Unknown,
                        element.clone(),
                        vec![],
                        format!(
                            "panel {p}'s {} bubble is by {}, and no figure in panel {p} has that id: use a figure's id, or \"off\" for a voice from outside the panel",
                            nth(j + 1),
                            bubble.by
                        ),
                    ));
                }
                found.extend(too_long(
                    element,
                    &format!("panel {p}'s bubble by {}", bubble.by),
                    "a bubble",
                    &bubble.text,
                    Limits::BUBBLE,
                ));
            }
            if panel.props.len() > Limits::PROPS {
                found.push(Finding::new(
                    FindingKind::Limit,
                    at.clone(),
                    vec![panel.props.len() as i32, Limits::PROPS as i32],
                    format!(
                        "panel {p} has {} props and a panel holds {}: keep the ones the joke needs",
                        panel.props.len(),
                        Limits::PROPS
                    ),
                ));
            }
            for (j, prop) in panel.props.iter().enumerate() {
                let element = format!("{at}/prop:{}", j + 1);
                if let LooseProp::Unknown(word) = &prop.kind {
                    found.push(Finding::new(
                        FindingKind::Unknown,
                        element.clone(),
                        vec![],
                        format!("panel {p}'s {} prop is a {word}, which the catalogue lacks: use one of {}", nth(j + 1), catalogue_of_props()),
                    ));
                }
                if let Some(text) = &prop.text {
                    found.extend(too_long(
                        element,
                        &format!("panel {p}'s {} text", prop.kind.word()),
                        "a prop",
                        text,
                        Limits::PROP_TEXT,
                    ));
                }
            }
        }
        found
    }

    /// The script cut to what the standard holds, and what was cut: every cut is a problem, so the strip is still drawn.
    pub fn clamped(&self) -> (Script, Vec<super::Problem>) {
        use super::Problem;
        let mut problems = Vec::new();
        let mut script = self.clone();
        if script.panels.len() > Limits::PANELS {
            problems.push(Problem::skipped(format!(
                "panels {} to {} were not drawn: a strip holds {}",
                Limits::PANELS + 1,
                script.panels.len(),
                Limits::PANELS
            )));
            script.panels.truncate(Limits::PANELS);
        }
        if let Some(title) = &mut script.title {
            if cut(title, Limits::TITLE) {
                problems.push(Problem::short(format!(
                    "the title was cut to {} characters to fit",
                    Limits::TITLE
                )));
            }
        }
        if let Some(alt) = &mut script.alt {
            if cut(alt, Limits::ALT) {
                problems.push(Problem::short(format!(
                    "the alt line was cut to {} characters",
                    Limits::ALT
                )));
            }
        }
        for (i, panel) in script.panels.iter_mut().enumerate() {
            let p = i + 1;
            if let Some(caption) = &mut panel.caption {
                if cut(&mut caption.text, Limits::CAPTION) {
                    problems.push(Problem::short(format!(
                        "panel {p}'s caption was cut to {} characters to fit",
                        Limits::CAPTION
                    )));
                }
            }
            if panel.figures.len() > Limits::FIGURES {
                for figure in &panel.figures[Limits::FIGURES..] {
                    problems.push(Problem::skipped(format!(
                        "panel {p}'s figure {} was not drawn: a panel holds {}",
                        figure.id,
                        Limits::FIGURES
                    )));
                }
                panel.figures.truncate(Limits::FIGURES);
            }
            for figure in &mut panel.figures {
                if let LoosePose::Unknown(word) = &figure.pose {
                    problems.push(Problem::fallback(format!(
                        "panel {p}'s figure {} asked for pose {word}, which the catalogue lacks: drawn standing",
                        figure.id
                    )));
                    figure.pose = LoosePose::Known(Pose::Standing);
                }
                if let Some(label) = &mut figure.label {
                    if cut(label, Limits::LABEL) {
                        problems.push(Problem::short(format!(
                            "panel {p}'s label under {} was cut to {} characters",
                            figure.id,
                            Limits::LABEL
                        )));
                    }
                }
                if let Some(sign) = &mut figure.sign {
                    if cut(sign, Limits::SIGN) {
                        problems.push(Problem::short(format!(
                            "panel {p}'s sign held by {} was cut to {} characters",
                            figure.id,
                            Limits::SIGN
                        )));
                    }
                }
            }
            if panel.bubbles.len() > Limits::BUBBLES {
                for j in Limits::BUBBLES..panel.bubbles.len() {
                    problems.push(Problem::skipped(format!(
                        "panel {p}'s {} bubble was not drawn: a panel holds {}",
                        nth(j + 1),
                        Limits::BUBBLES
                    )));
                }
                panel.bubbles.truncate(Limits::BUBBLES);
            }
            let ids: Vec<String> = panel.figures.iter().map(|f| f.id.clone()).collect();
            for bubble in &mut panel.bubbles {
                if bubble.by != "off" && !ids.contains(&bubble.by) {
                    problems.push(Problem::fallback(format!(
                        "panel {p}'s bubble by {} has no figure with that id: drawn as a voice from outside the panel",
                        bubble.by
                    )));
                    bubble.by = "off".into();
                }
                if cut(&mut bubble.text, Limits::BUBBLE) {
                    problems.push(Problem::short(format!(
                        "panel {p}'s bubble by {} was cut to {} characters to fit",
                        bubble.by,
                        Limits::BUBBLE
                    )));
                }
            }
            panel.props.retain(|prop| {
                if let LooseProp::Unknown(word) = &prop.kind {
                    problems.push(Problem::skipped(format!(
                        "panel {p}'s prop {word} was not drawn: the catalogue lacks it"
                    )));
                    false
                } else {
                    true
                }
            });
            if panel.props.len() > Limits::PROPS {
                for prop in &panel.props[Limits::PROPS..] {
                    problems.push(Problem::skipped(format!(
                        "panel {p}'s {} was not drawn: a panel holds {} props",
                        prop.kind.word(),
                        Limits::PROPS
                    )));
                }
                panel.props.truncate(Limits::PROPS);
            }
            for prop in &mut panel.props {
                if let Some(text) = &mut prop.text {
                    if cut(text, Limits::PROP_TEXT) {
                        problems.push(Problem::short(format!(
                            "panel {p}'s {} text was cut to {} characters",
                            prop.kind.word(),
                            Limits::PROP_TEXT
                        )));
                    }
                }
            }
        }
        (script, problems)
    }
}

/// Cuts `text` to at most `limit` characters, at a word when one ends near, with "..." after; whether it cut.
fn cut(text: &mut String, limit: usize) -> bool {
    if chars(text) <= limit {
        return false;
    }
    let keep: String = text.chars().take(limit.saturating_sub(3)).collect();
    let at_word = keep
        .rfind(' ')
        .filter(|&i| keep.len() - i < 20)
        .map(|i| keep[..i].trim_end().to_string());
    *text = format!("{}...", at_word.unwrap_or(keep));
    true
}
