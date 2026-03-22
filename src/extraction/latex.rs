use crate::utils::errors::{ProvenanceError, Result};
use std::path::Path;

/// Extract plain text from a LaTeX (.tex) file.
///
/// Strips LaTeX commands, environments, and math markup while preserving
/// the prose content. This handles the most common LaTeX constructs
/// found in academic papers and technical documents.
pub fn extract(path: &Path) -> Result<String> {
    let content = std::fs::read_to_string(path).map_err(|e| ProvenanceError::IoWithPath {
        path: path.display().to_string(),
        source: e,
    })?;

    Ok(strip_latex(&content))
}

/// Strip LaTeX markup and return prose text.
pub fn strip_latex(latex: &str) -> String {
    let mut output = String::with_capacity(latex.len() / 2);
    let mut in_math = false;
    let mut brace_depth: i32 = 0;
    let mut skip_until_brace_close = false;
    let mut skip_depth: i32 = 0;

    // First pass: remove comments (lines starting with % that aren't \%)
    let lines: Vec<&str> = latex.lines().collect();
    let mut cleaned = String::new();
    for line in &lines {
        let mut prev_was_backslash = false;
        let mut found_comment = None;
        for (i, ch) in line.char_indices() {
            if ch == '%' && !prev_was_backslash {
                found_comment = Some(i);
                break;
            }
            prev_was_backslash = ch == '\\';
        }
        if let Some(idx) = found_comment {
            cleaned.push_str(&line[..idx]);
        } else {
            cleaned.push_str(line);
        }
        cleaned.push('\n');
    }

    let mut chars = cleaned.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            // Math mode delimiters
            '$' => {
                if let Some(&'$') = chars.peek() {
                    chars.next(); // $$
                    in_math = !in_math;
                } else {
                    in_math = !in_math;
                }
            }
            _ if in_math => continue,

            '\\' => {
                // Read command name
                let mut cmd = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_ascii_alphabetic() || c == '@' {
                        cmd.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }

                if cmd.is_empty() {
                    // Escaped character: \%, \$, \&, etc.
                    if let Some(c) = chars.next() {
                        match c {
                            '%' | '$' | '&' | '#' | '_' => output.push(c),
                            '\\' => output.push('\n'), // line break
                            ' ' => output.push(' '),
                            _ => {}
                        }
                    }
                    continue;
                }

                // Skip optional arguments [...]
                if let Some(&'[') = chars.peek() {
                    let mut bracket_depth = 1;
                    chars.next();
                    while let Some(c) = chars.next() {
                        if c == '[' { bracket_depth += 1; }
                        if c == ']' { bracket_depth -= 1; }
                        if bracket_depth == 0 { break; }
                    }
                }

                match cmd.as_str() {
                    // Commands whose arguments we want to keep
                    "textbf" | "textit" | "emph" | "underline" | "textsc"
                    | "textrm" | "textsf" | "texttt" | "mbox" | "text" => {
                        // Consume opening brace, let content pass through
                        if let Some(&'{') = chars.peek() {
                            chars.next();
                            brace_depth += 1;
                        }
                    }
                    // Section commands — extract title, add paragraph break
                    "section" | "subsection" | "subsubsection" | "chapter"
                    | "paragraph" | "subparagraph" => {
                        // Handle starred variants
                        if let Some(&'*') = chars.peek() {
                            chars.next();
                        }
                        output.push_str("\n\n");
                        if let Some(&'{') = chars.peek() {
                            chars.next();
                            brace_depth += 1;
                        }
                    }
                    "title" | "author" | "date" => {
                        if let Some(&'{') = chars.peek() {
                            chars.next();
                            brace_depth += 1;
                        }
                    }
                    // Paragraph-like breaks
                    "par" | "newline" | "linebreak" => {
                        output.push_str("\n\n");
                    }
                    // Environment commands
                    "begin" => {
                        if let Some(&'{') = chars.peek() {
                            chars.next();
                            let mut env = String::new();
                            while let Some(&c) = chars.peek() {
                                if c == '}' { chars.next(); break; }
                                env.push(c);
                                chars.next();
                            }
                            // Skip math and code environments
                            match env.as_str() {
                                "equation" | "equation*" | "align" | "align*"
                                | "gather" | "gather*" | "math" | "displaymath"
                                | "eqnarray" | "eqnarray*" | "figure" | "table"
                                | "lstlisting" | "verbatim" | "tikzpicture" => {
                                    // Skip until \end{env}
                                    let end_marker = format!("\\end{{{env}}}");
                                    let remaining: String = chars.clone().collect();
                                    if let Some(pos) = remaining.find(&end_marker) {
                                        for _ in 0..pos + end_marker.len() {
                                            chars.next();
                                        }
                                    }
                                }
                                "itemize" | "enumerate" | "description" => {
                                    output.push('\n');
                                }
                                _ => {}
                            }
                        }
                    }
                    "end" => {
                        // Consume {envname}
                        if let Some(&'{') = chars.peek() {
                            chars.next();
                            while let Some(c) = chars.next() {
                                if c == '}' { break; }
                            }
                        }
                        output.push('\n');
                    }
                    "item" => {
                        output.push_str("\n");
                    }
                    // Commands whose arguments we skip entirely
                    "usepackage" | "documentclass" | "label" | "ref" | "cite"
                    | "bibliography" | "bibliographystyle" | "includegraphics"
                    | "input" | "include" | "newcommand" | "renewcommand"
                    | "setlength" | "addtolength" | "pagestyle" | "thispagestyle"
                    | "geometry" | "hypersetup" | "lstset" | "definecolor"
                    | "setcounter" | "pagenumbering" | "tableofcontents"
                    | "maketitle" | "hspace" | "vspace" | "hfill" | "vfill" => {
                        // Skip all brace arguments
                        while let Some(&'{') = chars.peek() {
                            chars.next();
                            let mut depth = 1;
                            while let Some(c) = chars.next() {
                                if c == '{' { depth += 1; }
                                if c == '}' { depth -= 1; }
                                if depth == 0 { break; }
                            }
                        }
                    }
                    // Spacing commands
                    "quad" | "qquad" => output.push(' '),
                    // Footnote — include the text
                    "footnote" => {
                        if let Some(&'{') = chars.peek() {
                            chars.next();
                            brace_depth += 1;
                        }
                    }
                    _ => {
                        // Unknown command: skip its brace argument if present
                        if let Some(&'{') = chars.peek() {
                            skip_until_brace_close = true;
                            skip_depth = brace_depth + 1;
                            chars.next();
                            brace_depth += 1;
                        }
                    }
                }
            }
            '{' => {
                brace_depth += 1;
            }
            '}' => {
                brace_depth = brace_depth.saturating_sub(1);
                if skip_until_brace_close && brace_depth < skip_depth {
                    skip_until_brace_close = false;
                }
            }
            '~' => output.push(' '), // non-breaking space
            _ if !skip_until_brace_close => {
                output.push(ch);
            }
            _ => {}
        }
    }

    output.trim().to_string()
}
