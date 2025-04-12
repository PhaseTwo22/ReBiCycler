use std::{
    collections::HashMap,
    fmt::{self},
    fs::File,
    io::{self, Write},
};

use console::{measure_text_width, strip_ansi_codes, truncate_str, Term};
use itertools::Itertools;

struct Pane {
    name: String,
    width: usize,
    rows: usize,
    content: Vec<String>,
}

impl fmt::Debug for Pane {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} [{}x{}]: {:?}",
            self.name, self.width, self.rows, self.content,
        )
    }
}

impl fmt::Display for Pane {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rows_too_few = self.rows.saturating_sub(self.content.len());
        let padding: Vec<String> = (0..rows_too_few).map(|_| String::new()).collect();
        let nice = std::iter::once(&self.name)
            .chain(self.content.iter())
            .chain(padding.iter())
            .map(|s| self.correct_length(s))
            .join("\n");
        write!(f, "{nice}")
    }
}

impl Pane {
    pub const fn new(width: usize, rows: usize, name: String) -> Self {
        Self {
            width,
            rows,
            name,
            content: Vec::new(),
        }
    }
    pub fn add_line(&mut self, message: String) {
        self.content.push(message);
    }

    fn correct_length(&self, message: &str) -> String {
        let tail = "...";
        let truncated = truncate_str(message, self.width, tail).to_string();

        let too_short_by = self.width.saturating_sub(measure_text_width(&truncated));
        let long_enough = format!("{}{}", truncated, " ".repeat(too_short_by));

        long_enough
    }

    fn clear(&mut self) {
        self.content.clear();
    }
}

pub struct MultiPane {
    joiner: String,
    rows: usize,
    panes: [Pane; 4],
    pane_names: HashMap<String, usize>,
}

impl MultiPane {
    pub fn new(panes: [(String, usize); 4], rows: usize) -> Self {
        let names = panes
            .iter()
            .enumerate()
            .map(|(i, (n, _))| (strip_ansi_codes(n).to_string(), i))
            .collect();
        let panes = panes.map(|(n, w)| Pane::new(w, rows, n));
        Self {
            joiner: " | ".to_string(),
            rows,
            panes,
            pane_names: names,
        }
    }

    fn pane_join(&self, pane_string: &[String; 4]) -> String {
        format!(
            "{}{}{}\n",
            self.joiner,
            pane_string.iter().join(&self.joiner),
            self.joiner,
        )
    }

    pub fn write_line_to_pane(&mut self, pane_name: &str, line: String) {
        let index = self.pane_names.get(pane_name).unwrap_or_else(|| {
            panic!(
                "Display panel name not found: {}.\nOptions are: \n - {}",
                pane_name,
                self.pane_names.keys().join("\n - ")
            )
        });
        let pane = self
            .panes
            .get_mut(*index)
            .unwrap_or_else(|| panic!("Display panel index mapping not found: {pane_name}"));
        pane.add_line(line);
    }

    pub fn clear(&mut self) {
        let _: () = self.panes.iter_mut().map(Pane::clear).collect();
    }
}

impl fmt::Display for MultiPane {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pane_strings: [Vec<String>; 4] = self.panes.each_ref().map(|p| {
            p.to_string()
                .split('\n')
                .map(std::string::ToString::to_string)
                .collect()
        });

        let mut line_iter = String::new();
        for row in 0..self.rows {
            let shaslica: [String; 4] = pane_strings.each_ref().map(|content_vec| {
                let def = String::new();
                let existing = content_vec.get(row);
                existing.unwrap_or(&def).clone()
            });
            line_iter += &self.pane_join(&shaslica);
        }

        if let Some(line_length) = line_iter.find('\n') {
            let table_cap = format!(
                " {} \n",
                "─".repeat(measure_text_width(&line_iter[0..line_length]) - 2)
            );

            write!(f, "{table_cap}{line_iter}{table_cap}")
        } else {
            write!(f, "content failed to render: no newlines in output")
        }
    }
}

pub struct DisplayTerminal {
    multi_pane: MultiPane,
    terminal: Term,
    history: Vec<String>,
}

impl Default for DisplayTerminal {
    fn default() -> Self {
        Self {
            multi_pane: MultiPane::new(
                [
                    ("Production".to_owned(), 20),
                    ("Construction".to_owned(), 20),
                    ("Research".to_owned(), 20),
                    ("Army".to_owned(), 20),
                ],
                25,
            ),
            terminal: Term::stdout(),
            history: Vec::new(),
        }
    }
}

impl DisplayTerminal {
    pub fn new(panes: [(String, usize); 4], rows: usize) -> Self {
        Self {
            multi_pane: MultiPane::new(panes, rows),
            terminal: Term::stdout(),
            history: Vec::new(),
        }
    }

    pub fn flush(&mut self) {
        let content = self.multi_pane.to_string();
        let _ = self.terminal.clear_last_lines(self.multi_pane.rows + 100);
        let _ = self.terminal.write_line(&content);
        self.history.push(content);
        self.multi_pane.clear();
    }

    pub fn write_line_to_pane(&mut self, pane_name: &str, msg: String) {
        self.multi_pane.write_line_to_pane(pane_name, msg);
    }

    pub fn save_history(&self, filename: &str) -> io::Result<()> {
        let mut output = File::create(filename)?;
        let line = self.history.iter().join("\n\n");
        write!(output, "{line}")
    }
}
